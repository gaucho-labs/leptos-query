use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use futures_channel::oneshot;
use leptos::*;
use slotmap::{new_key_type, SlotMap};

use crate::{garbage_collector::GarbageCollector, QueryState};

#[derive(Clone)]
pub(crate) struct Query<K, V>
where
    V: 'static,
{
    pub(crate) key: K,

    // Cancellation
    current_request: Rc<Cell<Option<oneshot::Sender<()>>>>,

    // State
    state: Rc<Cell<QueryState<V>>>,
    active_observer_count: RwSignal<usize>,

    // Synchronization
    observers: Rc<RefCell<SlotMap<ObserverKey, QueryObserver<V>>>>,
    garbage_collector: Rc<GarbageCollector<K, V>>,

    // Config
    gc_time: RwSignal<Option<Duration>>,
    stale_time: RwSignal<Option<Duration>>,
    refetch_interval: RwSignal<Option<Duration>>,
}

new_key_type! {
    pub(crate) struct ObserverKey;
}

impl<K: PartialEq, V> PartialEq for Query<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K: PartialEq, V> Eq for Query<K, V> {}

struct QueryObserver<V: 'static> {
    state: RwSignal<QueryState<V>>,
    kind: QueryObserverKind,
}

#[derive(Clone, Copy)]
pub(crate) enum QueryObserverKind {
    Active,
    Passive,
}

impl<K, V> Query<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub(crate) fn new(key: K) -> Self {
        let gc_time = RwSignal::new(None);
        Query {
            key: key.clone(),

            current_request: Rc::new(Cell::new(None)),

            observers: Rc::new(RefCell::new(SlotMap::with_key())),
            state: Rc::new(Cell::new(QueryState::Created)),
            garbage_collector: Rc::new(GarbageCollector::new(key, gc_time.clone().into())),
            active_observer_count: RwSignal::new(0),

            gc_time,
            refetch_interval: RwSignal::new(None),
            stale_time: RwSignal::new(None),
        }
    }
}

impl<K, V> Query<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub(crate) fn set_state(&self, state: QueryState<V>) {
        let observers = self.observers.borrow();
        for observer in observers.values() {
            observer.state.set(state.clone())
        }
        if let Some(updated_at) = state.updated_at() {
            self.garbage_collector.new_update(updated_at);
        }

        self.state.set(state);
    }

    pub(crate) fn update_state(&self, update_fn: impl FnOnce(&mut QueryState<V>)) {
        let mut state = self.state.take();
        update_fn(&mut state);
        self.set_state(state);
    }

    /// Be careful with this function. Used to avoid cloning.
    /// If update returns Ok(_) the state will be updated and subscribers will be notified.
    /// If update returns Err(_) the state will not be updated and subscribers will not be notified.
    /// Err(_) should always contain the previous state.
    pub(crate) fn maybe_map_state(
        &self,
        update_fn: impl FnOnce(QueryState<V>) -> Result<QueryState<V>, QueryState<V>>,
    ) -> bool {
        let current_state = self.state.take();

        match update_fn(current_state) {
            Ok(new_state) => {
                self.set_state(new_state);
                true
            }
            Err(old_state) => {
                self.state.set(old_state);
                false
            }
        }
    }

    /// Marks the resource as invalid, which will cause it to be refetched on next read.
    pub(crate) fn mark_invalid(&self) -> bool {
        let mut updated = false;
        self.maybe_map_state(|state| {
            if let QueryState::Loaded(data) = state {
                updated = true;
                logging::log!("MARKING INVALID {:?}", self.key);
                Ok(QueryState::Invalid(data))
            } else {
                Err(state)
            }
        });
        updated
    }

    pub(crate) fn register_observer(
        &self,
        kind: QueryObserverKind,
    ) -> (ReadSignal<QueryState<V>>, impl Fn() + Clone) {
        let current_state = self.get_state();
        let state_signal = RwSignal::new(current_state);
        let observer = QueryObserver {
            state: state_signal,
            kind,
        };
        let observer_id = self.observers.borrow_mut().insert(observer);
        let observer_count = self.active_observer_count.clone();
        if let QueryObserverKind::Active = kind {
            observer_count.set(observer_count.get_untracked() + 1);
        }

        let remove_observer = {
            let collector = self.garbage_collector.clone();
            let observers = self.observers.clone();
            move || {
                let mut observers = observers.borrow_mut();
                let removed = observers.remove(observer_id);

                if let Some(QueryObserver {
                    kind: QueryObserverKind::Active,
                    ..
                }) = removed
                {
                    observer_count.set(observer_count.get_untracked() - 1);
                }

                if observers
                    .values()
                    .map(|o| o.kind)
                    .all(|k| matches!(k, QueryObserverKind::Passive))
                {
                    collector.enable_gc();
                }
            }
        };

        self.garbage_collector.disable_gc();

        (state_signal.read_only(), remove_observer)
    }

    pub(crate) fn get_active_observer_count(&self) -> Signal<usize> {
        self.active_observer_count.into()
    }

    pub(crate) fn get_state(&self) -> QueryState<V> {
        let state = self.state.take();
        let state_clone = state.clone();
        self.state.set(state);
        state_clone
    }

    pub(crate) fn get_refetch_interval(&self) -> Signal<Option<Duration>> {
        self.refetch_interval.into()
    }

    pub(crate) fn get_stale_time(&self) -> Signal<Option<Duration>> {
        self.stale_time.into()
    }

    // Useful to avoid clones.
    pub(crate) fn with_state<T>(&self, func: impl FnOnce(&QueryState<V>) -> T) -> T {
        let state = self.state.take();
        let result = func(&state);
        self.state.set(state);
        result
    }

    /**
     * Execution and Cancellation.
     */

    // Only scenario where two requests can exist at the same time is the first is cancelled.
    pub(crate) fn new_execution(&self) -> Option<oneshot::Receiver<()>> {
        let current_request = self.current_request.take();
        if current_request.is_none() {
            let (sender, receiver) = oneshot::channel();
            self.current_request.set(Some(sender));
            Some(receiver)
        } else {
            self.current_request.set(current_request);
            None
        }
    }

    pub(crate) fn finalize_execution(&self) {
        self.current_request.set(None);
    }

    pub(crate) fn cancel(&self) -> bool {
        if let Some(current_request) = self.current_request.take() {
            let cancellation = current_request.send(());
            if cancellation.is_err() {
                logging::error!("Failed to cancel request {:?}", self.key);
            }
            cancellation.is_ok()
        } else {
            false
        }
    }

    // Enables having different stale times & refetch intervals for the same query.
    // The lowest stale time & refetch interval will be used.
    // When the scope is dropped, the stale time & refetch interval will be reset to the previous value (if they existed).
    // Cache time behaves differently. It will only use the minimum cache time found.
    pub(crate) fn update_options(&self, options: crate::QueryOptions<V>) {
        // Use the minimum cache time.
        match (self.gc_time.get_untracked(), options.gc_time) {
            (Some(current), Some(new)) if new < current => self.gc_time.set(Some(new)),
            (None, Some(new)) => self.gc_time.set(Some(new)),
            _ => (),
        }

        // Handle refetch interval.
        {
            let curr_refetch_interval = self.refetch_interval.get_untracked();
            let (prev_refetch, new_refetch) =
                match (curr_refetch_interval, options.refetch_interval) {
                    (Some(current), Some(new)) if new < current => (Some(current), Some(new)),
                    (None, Some(new)) => (None, Some(new)),
                    _ => (None, None),
                };

            if let Some(new_refetch) = new_refetch {
                self.refetch_interval.set(Some(new_refetch));
            }

            let refetch_interval = self.refetch_interval;
            on_cleanup(move || {
                if let Some(prev_refetch) = prev_refetch {
                    refetch_interval.set(Some(prev_refetch));
                }
            });
        }
        // Handle stale time.
        {
            let curr_stale_time = self.stale_time.get_untracked();
            let (prev_stale, new_stale) = match (curr_stale_time, options.stale_time) {
                (Some(current), Some(new)) if new < current => (Some(current), Some(new)),
                (None, Some(new)) => (None, Some(new)),
                _ => (None, None),
            };

            if let Some(stale_time) = new_stale {
                self.stale_time.set(Some(stale_time))
            }

            let stale_time = self.stale_time;
            on_cleanup(move || {
                if let Some(prev_stale) = prev_stale {
                    stale_time.set(Some(prev_stale));
                }
            })
        }
    }
}

impl<K, V> Query<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub(crate) fn dispose(&self) {
        debug_assert!(
            {
                self.observers
                    .borrow()
                    .values()
                    .map(|o| o.kind)
                    .all(|k| matches!(k, QueryObserverKind::Passive))
            },
            "Query has active observers"
        );
        self.gc_time.dispose();
        self.refetch_interval.dispose();
    }
}
