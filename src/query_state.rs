use leptos::*;
use std::{cell::Cell, future::Future, rc::Rc, time::Duration};

use crate::{
    ensure_valid_stale_time,
    instant::{get_instant, Instant},
    util::{time_until_stale, use_timeout},
    QueryOptions, ResourceOption,
};

#[derive(Clone)]
pub(crate) struct QueryState<K, V>
where
    K: 'static,
    V: 'static,
{
    pub(crate) key: K,
    pub(crate) resource: Resource<(), V>,
    pub(crate) observers: Rc<Cell<usize>>,
    pub(crate) value: RwSignal<Option<V>>,
    pub(crate) stale_time: RwSignal<Option<Duration>>,
    pub(crate) refetch_interval: RwSignal<Option<Duration>>,
    pub(crate) updated_at: RwSignal<Option<Instant>>,
    pub(crate) invalidated: RwSignal<bool>,
    pub(crate) fetching: RwSignal<bool>,
}

impl<K: PartialEq, V> PartialEq for QueryState<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K: PartialEq, V> Eq for QueryState<K, V> {}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + Serializable + 'static,
{
    pub(crate) fn new<Fu>(
        cx: Scope,
        key: K,
        query: Rc<impl Fn(K) -> Fu + 'static>,
        options: QueryOptions<V>,
    ) -> Self
    where
        Fu: Future<Output = V> + 'static,
    {
        let stale_time = ensure_valid_stale_time(&options.stale_time, &options.cache_time);
        let stale_time = create_rw_signal(cx, stale_time);
        let refetch_interval = create_rw_signal(cx, options.refetch_interval);

        let value = create_rw_signal(cx, None);
        let updated_at = create_rw_signal(cx, None);
        let invalidated = create_rw_signal(cx, false);
        let fetching = create_rw_signal(cx, false);

        let fetcher = {
            let key = key.clone();
            move |_: ()| {
                let key = key.clone();
                let query = query.clone();
                async move {
                    fetching.set(true);
                    let result = query(key).await;
                    updated_at.set(Some(get_instant()));
                    fetching.set(false);
                    value.set(Some(result.clone()));
                    result
                }
            }
        };
        let QueryOptions {
            default_value,
            ref resource_option,
            ..
        } = options;

        let resource = {
            match resource_option {
                ResourceOption::NonBlocking => {
                    create_resource_with_initial_value(cx, || (), fetcher, default_value)
                }
                ResourceOption::Blocking => create_blocking_resource(cx, || (), fetcher),
            }
        };

        QueryState {
            key,
            resource,
            observers: Rc::new(Cell::new(0)),
            value,
            stale_time,
            refetch_interval,
            updated_at,
            invalidated,
            fetching,
        }
    }
}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    /// Marks the resource as invalidated, which will cause it to be refetched on next read.
    pub(crate) fn invalidate(&self) {
        self.invalidated.set(true);
    }

    pub(crate) fn refetch(&self) {
        if self.updated_at.get_untracked().is_some() {
            if !self.resource.loading().get_untracked() {
                self.resource.refetch();
                self.invalidated.set(false);
            }
        }
    }

    pub(crate) fn is_stale(&self, cx: Scope) -> Signal<bool> {
        let updated_at = self.updated_at;
        let stale_time = self.stale_time;
        let (stale, set_stale) = create_signal(cx, false);
        use_timeout(cx, move || match (updated_at.get(), stale_time.get()) {
            (Some(updated_at), Some(stale_time)) => {
                let timeout = time_until_stale(updated_at, stale_time);
                if !timeout.is_zero() {
                    set_stale.set(false);
                }
                set_timeout_with_handle(
                    move || {
                        set_stale.set(true);
                    },
                    timeout,
                )
                .ok()
            }
            _ => None,
        });

        stale.into()
    }

    /// If the query is being fetched for the first time.
    /// IMPORTANT: If the query is never [read](QueryState::read), this will always return false.
    pub(crate) fn is_loading(&self, cx: Scope) -> Signal<bool> {
        let updated_at = self.updated_at;
        let fetching = self.fetching;

        Signal::derive(cx, move || updated_at.get().is_none() && fetching.get())
    }

    // Enables having different stale times & refetch intervals for the same query.
    // The lowest stale time & refetch interval will be used.
    // When the scope is dropped, the stale time & refetch interval will be reset to the previous value (if they existed).
    pub(crate) fn set_options(&self, cx: Scope, options: QueryOptions<V>) {
        let curr_stale = self.stale_time.get_untracked();
        let curr_refetch_interval = self.refetch_interval.get_untracked();

        let (prev_stale, new_stale) = match (curr_stale, options.stale_time) {
            (Some(current), Some(new)) if current > new => (Some(current), Some(new)),
            (None, Some(new)) => (None, Some(new)),
            _ => (None, None),
        };

        let (prev_refetch, new_refetch) = match (curr_refetch_interval, options.refetch_interval) {
            (Some(current), Some(new)) if current > new => (Some(current), Some(new)),
            (None, Some(new)) => (None, Some(new)),
            _ => (None, None),
        };

        if let Some(new_stale) = new_stale {
            self.stale_time.set(Some(new_stale));
        }

        if let Some(new_refetch) = new_refetch {
            self.refetch_interval.set(Some(new_refetch));
        }

        // Reset stale time and refetch interval to previous values when scope is dropped.
        let stale_time = self.stale_time;
        let refetch_interval = self.refetch_interval;
        on_cleanup(cx, move || {
            if let Some(prev_stale) = prev_stale {
                stale_time.set(Some(prev_stale));
            }

            if let Some(prev_refetch) = prev_refetch {
                refetch_interval.set(Some(prev_refetch));
            }
        })
    }

    pub(crate) fn read(&self, cx: Scope) -> Option<V> {
        let updated_at = self.updated_at;
        let stale_time = self.stale_time;
        let resource = self.resource;
        let invalidated = self.invalidated;

        let fetching = self.fetching;

        // On mount, ensure that the resource is not stale
        match (updated_at.get_untracked(), stale_time.get_untracked()) {
            (Some(updated_at), Some(stale_time)) => {
                if time_until_stale(updated_at, stale_time).is_zero() && !fetching.get_untracked() {
                    resource.refetch();
                    invalidated.set(false);
                }
            }
            _ => (),
        }

        // Happens when the resource is SSR'd.
        let read = resource.read(cx);
        if read.is_some() && updated_at.get_untracked().is_none() {
            updated_at.set(Some(get_instant()));
        }

        read
    }
}

impl<K, V> QueryState<K, V> {
    pub(crate) fn dispose(&self) {
        self.value.dispose();
        self.stale_time.dispose();
        self.refetch_interval.dispose();
        self.updated_at.dispose();
        self.invalidated.dispose();
        self.fetching.dispose();
    }
}
