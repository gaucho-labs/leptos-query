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
    pub(crate) observers: Rc<Cell<usize>>,
    #[allow(dead_code)]
    pub(crate) key: K,
    pub(crate) resource: RwSignal<Resource<(), V>>,
    pub(crate) stale_time: RwSignal<Option<Duration>>,
    pub(crate) refetch_interval: RwSignal<Option<Duration>>,
    pub(crate) updated_at: RwSignal<Option<Instant>>,
    pub(crate) invalidated: RwSignal<bool>,
}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + Serializable + 'static,
{
    pub(crate) fn new<Fu>(
        scope: Scope,
        key: K,
        fetcher: impl Fn(K) -> Fu + 'static,
        options: QueryOptions<V>,
    ) -> Self
    where
        Fu: Future<Output = V> + 'static,
    {
        let fetcher = Rc::new(fetcher);
        let updated_at: RwSignal<Option<Instant>> = create_rw_signal(scope, None);

        let key = key.clone();
        let fetcher = {
            let fetcher = fetcher.clone();
            let key = key.clone();
            move |_: ()| {
                let fetcher = fetcher.clone();
                let key = key.clone();
                async move {
                    let result = fetcher(key).await;
                    let instant = get_instant();
                    updated_at.set(Some(instant));
                    result
                }
            }
        };

        let QueryOptions {
            default_value,
            ref resource_option,
            refetch_interval,
            ref stale_time,
            ref cache_time,
            ..
        } = options;

        let resource = {
            match resource_option {
                ResourceOption::NonBlocking => {
                    create_resource_with_initial_value(scope, || (), fetcher, default_value)
                }
                ResourceOption::Blocking => create_blocking_resource(scope, || (), fetcher),
            }
        };

        let stale_time = ensure_valid_stale_time(stale_time, cache_time);

        QueryState {
            observers: Rc::new(Cell::new(0)),
            key: key.clone(),
            resource: create_rw_signal(scope, resource),
            stale_time: create_rw_signal(scope, stale_time),
            refetch_interval: create_rw_signal(scope, refetch_interval),
            updated_at,
            invalidated: create_rw_signal(scope, false),
        }
    }
}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    pub(crate) fn refetch(&self) {
        let resource = self.resource.get();
        if !resource.loading().get() {
            resource.refetch();
            self.invalidated.set(false);
        }
    }

    /// Marks the resource as invalidated, which will cause it to be refetched on next read.
    pub(crate) fn invalidate(&self) {
        self.invalidated.set(true);
    }

    /// If the query is currently being fetched in the background.
    pub(crate) fn is_refetching(&self, cx: Scope) -> Signal<bool> {
        let resource = self.resource;

        Signal::derive(cx, move || resource.get().loading().get())
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
        let is_loading = self.resource;
        Signal::derive(cx, move || {
            updated_at.get().is_none() && is_loading.get().loading().get()
        })
    }

    pub(crate) fn set_options(&self, cx: Scope, options: QueryOptions<V>) {
        let curr_stale = self.stale_time.get();
        let curr_refetch_interval = self.refetch_interval.get();

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

    pub(crate) fn read(&self, cx: Scope) -> Signal<Option<V>> {
        let invalidated = self.invalidated;
        let refetch_interval = self.refetch_interval;
        let resource = self.resource;
        let stale_time = self.stale_time;
        let updated_at = self.updated_at;

        let refetch = move || {
            if updated_at.get_untracked().is_some() {
                let resource = resource.get_untracked();
                if !resource.loading().get_untracked() {
                    resource.refetch();
                    invalidated.set(false);
                }
            }
        };
        let refetch = store_value(cx, refetch);

        // Effect for refetching query on interval.
        use_timeout(cx, move || {
            match (updated_at.get(), refetch_interval.get()) {
                (Some(updated_at), Some(refetch_interval)) => {
                    let timeout = time_until_stale(updated_at, refetch_interval);
                    set_timeout_with_handle(
                        move || {
                            refetch.with_value(|r| r());
                        },
                        timeout,
                    )
                    .ok()
                }
                _ => None,
            }
        });

        // Refetch query if invalidated.
        create_effect(cx, {
            move |_| {
                if invalidated.get() {
                    refetch.with_value(|r| r());
                }
            }
        });

        Signal::derive(cx, move || {
            // On mount, ensure that the resource is not stale
            match (updated_at.get_untracked(), stale_time.get_untracked()) {
                (Some(updated_at), Some(stale_time)) => {
                    if time_until_stale(updated_at, stale_time).is_zero() {
                        refetch.with_value(|r| r());
                    }
                }
                _ => (),
            }

            // Happens when the resource is SSR'd.
            let read = resource.get().read(cx);
            if read.is_some() && updated_at.get_untracked().is_none() {
                updated_at.set(Some(get_instant()));
            }

            read
        })
    }
}

impl<K, V> QueryState<K, V> {
    pub(crate) fn dispose(&self) {
        // TODO: Dispose Resource with runtime.
        self.resource.dispose();
        self.stale_time.dispose();
        self.refetch_interval.dispose();
        self.updated_at.dispose();
        self.invalidated.dispose();
    }
}
