use leptos::{leptos_dom::helpers::TimeoutHandle, *};
use std::{cell::Cell, rc::Rc, time::Duration};

use crate::{
    instant::{get_instant, Instant},
    QueryOptions,
};

#[derive(Clone, Debug)]
pub struct QueryState<K, V>
where
    K: 'static,
    V: 'static,
{
    key: K,
    stale_time: RwSignal<Option<Duration>>,
    refetch_interval: RwSignal<Option<Duration>>,
    // Epoch Millis timestamp of last update.
    last_updated: RwSignal<Option<Instant>>,
    // Whether the resource must be refetched on next read.
    invalidated: RwSignal<bool>,
    resource: Rc<Resource<K, V>>,
}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    pub(crate) fn new(
        cx: Scope,
        // TODO: Should this be an Rc?
        key: K,
        stale_time: Rc<Cell<Option<Duration>>>,
        refetch_interval: Rc<Cell<Option<Duration>>>,
        resource: Resource<K, V>,
        last_updated: RwSignal<Option<Instant>>,
    ) -> Self {
        Self {
            key,
            stale_time: create_rw_signal(cx, stale_time.get()),
            refetch_interval: create_rw_signal(cx, refetch_interval.get()),
            resource: Rc::new(resource),
            last_updated,
            invalidated: create_rw_signal(cx, false),
        }
    }

    pub fn key(&self) -> K {
        self.key.clone()
    }

    pub fn refetch(&self) {
        self.resource.refetch()
    }

    // Marks the resource as invalidated, which will cause it to be refetched on next read.
    pub fn invalidate(&self) {
        self.invalidated.set(true);
    }

    // If the query is currently being fetched in the background.
    pub fn is_refetching(&self) -> Signal<bool> {
        self.resource.loading().into()
    }

    // If the query is being fetched for the first time.
    // IMPORTANT: If the query is never `read`, this will always return false.
    pub fn is_loading(&self, cx: Scope) -> Memo<bool> {
        let last_updated = self.last_updated;
        let is_loading = self.resource.loading();
        create_memo(cx, move |_| {
            last_updated.get().is_none() && is_loading.get()
        })
    }

    pub fn stale_time(&self) -> Signal<Option<Duration>> {
        self.stale_time.read_only().into()
    }

    pub fn refetch_interval(&self) -> Signal<Option<Duration>> {
        self.refetch_interval.read_only().into()
    }

    pub fn set_options(&self, options: QueryOptions) {
        let QueryOptions {
            stale_time,
            refetch_interval,
        } = options;

        self.stale_time.set(stale_time);
        self.refetch_interval.set(refetch_interval);
    }

    // Update the stale time on the query associated with the current Key.
    pub fn set_stale_time(&self, stale_time: Duration) {
        self.stale_time.set(Some(stale_time));
    }

    // Update the refetch interval on the query associated with the current Key.
    pub fn set_refetch_interval(&self, refetch_interval: Duration) {
        self.refetch_interval.set(Some(refetch_interval));
    }

    pub fn clear_refetch_interval(&self) {
        self.refetch_interval.set(None);
    }

    // Query will never be considered stale.
    pub fn clear_stale_time(&self) {
        self.stale_time.set(None);
    }

    pub fn read(&self, cx: Scope) -> Signal<Option<V>> {
        let resource = self.resource.clone();
        let invalidated = self.invalidated;
        let stale_time = self.stale_time;
        let last_updated = self.last_updated;

        // Saves last interval to be cleared on cleanup.
        // TODO: Ensure this is necessary.
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
            let resource = resource.clone();
            let refetch_interval = self.refetch_interval;
            let interval = interval.clone();

            move |maybe_handle: Option<Option<TimeoutHandle>>| {
                let maybe_handle = maybe_handle.flatten();
                if let Some(handle) = maybe_handle {
                    handle.clear();
                };
                match (last_updated.get(), refetch_interval.get()) {
                    (Some(last_updated), Some(refetch_interval)) => {
                        let timeout = time_until_stale(last_updated, refetch_interval);
                        log!("Setting refetch timeout for: {:?}", timeout);
                        let resource = resource.clone();
                        let handle = set_timeout_with_handle(
                            move || {
                                resource.refetch();
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
            let resource = resource.clone();
            move |_| {
                if last_updated.get().is_some() {
                    if invalidated.get() {
                        invalidated.set_untracked(false);
                        resource.refetch();
                    }
                }
            }
        });

        Signal::derive(cx, move || {
            // On mount, ensure that the resource is not stale
            match (last_updated.get_untracked(), stale_time.get_untracked()) {
                (Some(last_updated), Some(stale_time)) => {
                    if time_until_stale(last_updated, stale_time).is_zero() {
                        resource.refetch();
                    }
                }
                _ => (),
            }

            // Happens when the resource is SSR'd.
            let read = resource.read(cx);
            if read.is_some() && last_updated.get_untracked().is_none() {
                last_updated.set(Some(get_instant()));
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
    // Render optimized version of `QueryState::read`
    // Not sure if this is needed.
    pub fn read_memo(&self, cx: Scope) -> Memo<Option<V>> {
        let signal = self.read(cx);
        create_memo(cx, move |_| signal.get())
    }
}

fn time_until_stale(last_updated: Instant, stale_time: Duration) -> Duration {
    let last_updated = last_updated.0.as_millis() as i64;
    let now = get_instant().0.as_millis() as i64;
    let stale_time = stale_time.as_millis() as i64;
    let result = (last_updated + stale_time) - now;
    let ensure_non_negative = result.max(0);
    Duration::from_millis(ensure_non_negative as u64)
}
