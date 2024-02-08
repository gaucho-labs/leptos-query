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
    K: 'static,
    V: 'static,
{
    pub(crate) key: K,

    // Cancellation.
    current_request: Rc<Cell<Option<oneshot::Sender<()>>>>,

    // Synchronization.
    observers: Rc<RefCell<SlotMap<ObserverKey, RwSignal<QueryState<V>>>>>,
    state: Rc<Cell<QueryState<V>>>,
    garbage_collector: Rc<GarbageCollector<K, V>>,

    // Config.
    gc_time: RwSignal<Option<Duration>>,
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

impl<K, V> Query<K, V>
where
    K: Clone + Eq + std::hash::Hash,
{
    pub(crate) fn new(key: K) -> Self {
        let gc_time = RwSignal::new(None);
        Query {
            key: key.clone(),

            current_request: Rc::new(Cell::new(None)),

            observers: Rc::new(RefCell::new(SlotMap::with_key())),
            state: Rc::new(Cell::new(QueryState::Created)),
            garbage_collector: Rc::new(GarbageCollector::new(key, gc_time.clone().into())),

            gc_time,
            refetch_interval: RwSignal::new(None),
        }
    }
}

impl<K, V> Query<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    pub(crate) fn set_state(&self, state: QueryState<V>) {
        let observers = self.observers.borrow();
        for observer in observers.values() {
            observer.set(state.clone())
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
                Ok(QueryState::Invalid(data))
            } else {
                Err(state)
            }
        });
        updated
    }

    pub(crate) fn register_observer(&self) -> (ReadSignal<QueryState<V>>, impl Fn() + Clone) {
        let current_state = self.get_state();
        let state_signal = RwSignal::new(current_state);
        let observer_id = self.observers.borrow_mut().insert(state_signal);

        let remove_observer = {
            let collector = self.garbage_collector.clone();
            let observers = self.observers.clone();
            move || {
                let mut observers = observers.borrow_mut();
                observers.remove(observer_id);
                if observers.is_empty() {
                    collector.enable_gc();
                }
            }
        };

        self.garbage_collector.disable_gc();

        (state_signal.read_only(), remove_observer)
    }
}

impl<K, V> Query<K, V>
where
    K: Clone,
    V: Clone,
{
    pub(crate) fn get_state(&self) -> QueryState<V> {
        let state = self.state.take();
        let state_clone = state.clone();
        self.state.set(state);
        state_clone
    }

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
                logging::error!("Failed to cancel request");
            }
            cancellation.is_ok()
        } else {
            false
        }
    }

    pub(crate) fn get_refetch_interval(&self) -> Signal<Option<Duration>> {
        self.refetch_interval.into()
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

        let curr_refetch_interval = self.refetch_interval.get_untracked();

        let (prev_refetch, new_refetch) = match (curr_refetch_interval, options.refetch_interval) {
            (Some(current), Some(new)) if new < current => (Some(current), Some(new)),
            (None, Some(new)) => (None, Some(new)),
            _ => (None, None),
        };

        if let Some(new_refetch) = new_refetch {
            self.refetch_interval.set(Some(new_refetch));
        }

        // Reset stale time and refetch interval to previous values when scope is dropped.
        let refetch_interval = self.refetch_interval;
        on_cleanup(move || {
            if let Some(prev_refetch) = prev_refetch {
                refetch_interval.set(Some(prev_refetch));
            }
        })
    }
}

impl<K, V> Query<K, V> {
    pub(crate) fn dispose(&self) {
        logging::log!("disposing of query");
        debug_assert!(self.observers.borrow().is_empty(), "Query has observers");
        self.gc_time.dispose();
        self.refetch_interval.dispose();
    }
}
