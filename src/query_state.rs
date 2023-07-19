use leptos::{leptos_dom::helpers::TimeoutHandle, *};
use std::{cell::Cell, future::Future, hash::Hash, rc::Rc, time::Duration};

use crate::{
    instant::{get_instant, Instant},
    QueryOptions, ResourceOption,
};

#[derive(Clone)]
pub struct QueryState<K, V>
where
    K: 'static,
    V: 'static,
{
    pub(crate) observers: Rc<Cell<usize>>,
    pub(crate) key: K,
    pub(crate) resource: RwSignal<Resource<(), V>>,
    pub(crate) stale_time: RwSignal<Option<Duration>>,
    pub(crate) refetch_interval: RwSignal<Option<Duration>>,
    pub(crate) updated_at: RwSignal<Option<Instant>>,
    // Whether the resource must be refetched on next read.
    pub(crate) invalidated: RwSignal<bool>,
}

impl<K, V> QueryState<K, V>
where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + Serializable + 'static,
{
    // Creates a new query cache.
    pub(crate) fn new<Fu>(
        scope: Scope,
        key: K,
        fetcher: impl Fn(K) -> Fu + 'static,
        options: QueryOptions<V>,
    ) -> Self
    where
        Fu: Future<Output = V> + 'static,
    {
        let stored_fetcher = Rc::new(fetcher);
        let updated_at: RwSignal<Option<Instant>> = create_rw_signal(scope, None);

        let key = key.clone();
        let fetcher = {
            let stored_fetcher = stored_fetcher.clone();
            let key = key.clone();
            move |_: ()| {
                let stored_fetcher = stored_fetcher.clone();
                let key = key.clone();
                async move {
                    let result = stored_fetcher(key).await;
                    let instant = get_instant();
                    updated_at.set(Some(instant));
                    result
                }
            }
        };

        let default_value: Option<V> = options.default_value;
        let resource = {
            match options.resource_option {
                ResourceOption::NonBlocking => {
                    create_resource_with_initial_value(scope, || (), fetcher, default_value)
                }
                ResourceOption::Blocking => create_blocking_resource(scope, || (), fetcher),
            }
        };

        QueryState {
            observers: Rc::new(Cell::new(0)),
            key: key.clone(),
            // fetcher: boxed_fetcher,
            resource: create_rw_signal(scope, resource),
            stale_time: create_rw_signal(scope, options.stale_time),
            refetch_interval: create_rw_signal(scope, options.refetch_interval),
            updated_at: updated_at,
            invalidated: create_rw_signal(scope, false),
        }
    }
}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    /// Key for the current query.
    pub fn key(&self) -> &K {
        &self.key
    }

    pub(crate) fn refetch(&self) {
        let resource = self.resource.get();
        if !resource.loading().get() {
            resource.refetch();
            self.invalidated.set(false);
        }
    }

    /// Marks the resource as invalidated, which will cause it to be refetched on next read.
    pub fn invalidate(&self) {
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
        Signal::derive(cx, move || {
            let updated_at = updated_at.get();
            let stale_time = stale_time.get();
            if let (Some(updated_at), Some(stale_time)) = (updated_at, stale_time) {
                time_until_stale(updated_at, stale_time).is_zero()
            } else {
                false
            }
        })
    }

    /// If the query is being fetched for the first time.
    /// IMPORTANT: If the query is never [read](QueryState::read), this will always return false.
    pub fn is_loading(&self, cx: Scope) -> Signal<bool> {
        let updated_at = self.updated_at;
        let is_loading = self.resource;
        Signal::derive(cx, move || {
            updated_at.get().is_none() && is_loading.get().loading().get()
        })
    }

    pub(crate) fn set_options(&self, options: QueryOptions<V>) {
        if let Some(stale_time) = options.stale_time {
            self.stale_time.set(Some(stale_time));
        }
        if let Some(refetch_interval) = options.refetch_interval {
            self.refetch_interval.set(Some(refetch_interval));
        }
    }

    pub fn read(&self, cx: Scope) -> Signal<Option<V>> {
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

        // Saves last interval to be cleared on cleanup.
        let interval: Rc<Cell<Option<TimeoutHandle>>> = Rc::new(Cell::new(None));
        let clean_up = {
            let interval = interval.clone();
            move || {
                if let Some(handle) = interval.take() {
                    handle.clear();
                }
            }
        };
        on_cleanup(cx, clean_up);

        // Sets refetch interval timeout, if it exists.
        create_effect(cx, {
            let interval = interval.clone();

            move |maybe_handle: Option<Option<TimeoutHandle>>| {
                let maybe_handle = maybe_handle.flatten();
                if let Some(handle) = maybe_handle {
                    handle.clear();
                };
                match (updated_at.get(), refetch_interval.get()) {
                    (Some(updated_at), Some(refetch_interval)) => {
                        let timeout = time_until_stale(updated_at, refetch_interval);
                        let handle = set_timeout_with_handle(
                            move || {
                                refetch.with_value(|r| r());
                            },
                            timeout,
                        )
                        .ok();
                        interval.set(handle);
                        handle
                    }
                    _ => None,
                }
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

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + PartialEq + 'static,
{
    /// Render optimized version of [`QueryState::read`].
    pub fn read_memo(&self, cx: Scope) -> Memo<Option<V>> {
        let signal = self.read(cx);
        create_memo(cx, move |_| signal.get())
    }
}

pub(crate) fn time_until_stale(updated_at: Instant, stale_time: Duration) -> Duration {
    let updated_at = updated_at.0.as_millis() as i64;
    let now = get_instant().0.as_millis() as i64;
    let stale_time = stale_time.as_millis() as i64;
    let result = (updated_at + stale_time) - now;
    let ensure_non_negative = result.max(0);
    Duration::from_millis(ensure_non_negative as u64)
}