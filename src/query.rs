use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use leptos::*;
use slotmap::{new_key_type, SlotMap};

use crate::QueryState;

#[derive(Clone)]
pub(crate) struct Query<K, V>
where
    K: 'static,
    V: 'static,
{
    pub(crate) key: K,

    // pub(crate) fetching: Rc<Cell<bool>>,
    version: Rc<Cell<usize>>,
    cancelled_execs: Rc<RefCell<Vec<usize>>>,
    active_execs: Rc<RefCell<Vec<usize>>>,

    observers: Rc<RefCell<SlotMap<ObserverKey, RwSignal<QueryState<V>>>>>,
    state: Rc<Cell<QueryState<V>>>,
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

impl<K, V> Query<K, V> {
    pub(crate) fn new(key: K) -> Self {
        Query {
            key,
            version: Rc::new(Cell::new(0)),
            active_execs: Rc::new(RefCell::new(Vec::new())),
            cancelled_execs: Rc::new(RefCell::new(Vec::new())),
            observers: Rc::new(RefCell::new(SlotMap::with_key())),
            state: Rc::new(Cell::new(QueryState::Created)),
        }
    }
}

impl<K, V> Query<K, V>
where
    K: Clone,
    V: Clone,
{
    pub(crate) fn get_state(&self) -> QueryState<V> {
        let state = self.state.replace(QueryState::Created);
        let state_clone = state.clone();
        self.state.set(state);
        state_clone
    }

    pub(crate) fn set_state(&self, state: QueryState<V>) {
        let observers = self.observers.borrow();
        for observer in observers.values() {
            observer.set(state.clone())
        }
        self.state.set(state);
    }

    pub(crate) fn with_state<T>(&self, func: impl FnOnce(&QueryState<V>) -> T) -> T {
        let state = self.state.replace(QueryState::Created);
        let result = func(&state);
        self.state.set(state);
        result
    }

    /**
     * Only scenario where two requests can exist at the same time is the first is ancelled.
     */
    pub(crate) fn new_execution(&self) -> Option<usize> {
        let is_executing = {
            let active = self.active_execs.borrow();
            let cancelled = self.cancelled_execs.borrow();
            self.debug_cancellation();

            active.len() > cancelled.len()
        };
        if is_executing {
            None
        } else {
            let exec_count = self.version.get() + 1;
            self.version.set(exec_count);
            self.active_execs.borrow_mut().push(exec_count);
            Some(exec_count)
        }
    }

    pub(crate) fn is_executing(&self) -> bool {
        let active = self.active_execs.borrow();
        let cancelled = self.cancelled_execs.borrow();
        self.debug_cancellation();
        active.len() > cancelled.len()
    }

    pub(crate) fn finish_exec(&self, execution_id: usize) {
        self.active_execs
            .borrow_mut()
            .retain(|id| *id != execution_id);
        self.cancelled_execs
            .borrow_mut()
            .retain(|id| *id != execution_id);
    }

    pub(crate) fn is_cancelled(&self, execution_version: usize) -> bool {
        self.cancelled_execs.borrow().contains(&execution_version)
    }

    pub(crate) fn cancel(&self) -> bool {
        self.debug_cancellation();

        let latest_executing = self.active_execs.borrow().last().cloned();
        let mut cancelled = self.cancelled_execs.borrow_mut();

        if cancelled.last() != latest_executing.as_ref() {
            cancelled.push(latest_executing.unwrap());
            true
        } else {
            false
        }
    }

    fn debug_cancellation(&self) {
        let active = self.active_execs.borrow();
        let cancelled = self.cancelled_execs.borrow();
        debug_assert!(
            active.len() >= cancelled.len(),
            "More cancelled than active executions"
        );
        debug_assert!(
            active.len() - cancelled.len() <= 1,
            "More than one active non-cancelled execution"
        );
    }

    pub(crate) fn update_state(&self, update_fn: impl FnOnce(&mut QueryState<V>)) {
        let mut state = self.state.replace(QueryState::Created);
        update_fn(&mut state);
        self.set_state(state);
    }

    pub(crate) fn maybe_map_state(
        &self,
        update_fn: impl FnOnce(QueryState<V>) -> Result<QueryState<V>, QueryState<V>>,
    ) -> bool {
        let current_state = self.state.replace(QueryState::Created);

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

    pub(crate) fn register_observer(&self) -> (ObserverKey, ReadSignal<QueryState<V>>) {
        logging::log!("register_observer: {}", self.observers.borrow().len());

        let current_state = self.get_state();
        let state_signal = RwSignal::new(current_state);
        let observer_id = self.observers.borrow_mut().insert(state_signal);

        (observer_id, state_signal.read_only())
    }

    pub(crate) fn remove_observer(&self, observer_id: ObserverKey) {
        logging::log!("remove_observer: {}", self.observers.borrow().len());
        let removed = self.observers.borrow_mut().remove(observer_id);

        if let Some(signal) = removed {
            logging::log!("remove_observer ID {:?}", observer_id);
            signal.dispose();
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

    // pub(crate) fn overwrite_options(&self, options: QueryOptions<V>) {
    //     let stale_time = ensure_valid_stale_time(&options.stale_time, &options.cache_time);

    //     self.stale_time.set(stale_time);
    //     self.cache_time.set(options.cache_time);
    //     self.refetch_interval.set(options.refetch_interval);
    // }

    // Enables having different stale times & refetch intervals for the same query.
    // The lowest stale time & refetch interval will be used.
    // When the scope is dropped, the stale time & refetch interval will be reset to the previous value (if they existed).
    // Cache time behaves differently. It will only use the minimum cache time found.
    // pub(crate) fn update_options(&self, options: QueryOptions<V>) {
    //     // Use the minimum cache time.
    //     match (self.cache_time.get_untracked(), options.cache_time) {
    //         (Some(current), Some(new)) if new < current => self.cache_time.set(Some(new)),
    //         (None, Some(new)) => self.cache_time.set(Some(new)),
    //         _ => (),
    //     }

    //     let curr_stale = self.stale_time.get_untracked();
    //     let curr_refetch_interval = self.refetch_interval.get_untracked();

    //     let (prev_stale, new_stale) = match (curr_stale, options.stale_time) {
    //         (Some(current), Some(new)) if new < current => (Some(current), Some(new)),
    //         (None, Some(new)) => (None, Some(new)),
    //         _ => (None, None),
    //     };

    //     let (prev_refetch, new_refetch) = match (curr_refetch_interval, options.refetch_interval) {
    //         (Some(current), Some(new)) if new < current => (Some(current), Some(new)),
    //         (None, Some(new)) => (None, Some(new)),
    //         _ => (None, None),
    //     };

    //     if let Some(new_stale) = new_stale {
    //         self.stale_time.set(Some(new_stale));
    //     }

    //     if let Some(new_refetch) = new_refetch {
    //         self.refetch_interval.set(Some(new_refetch));
    //     }

    //     // Reset stale time and refetch interval to previous values when scope is dropped.
    //     let stale_time = self.stale_time;
    //     let refetch_interval = self.refetch_interval;
    //     on_cleanup(move || {
    //         if let Some(prev_stale) = prev_stale {
    //             stale_time.set(Some(prev_stale));
    //         }

    //         if let Some(prev_refetch) = prev_refetch {
    //             refetch_interval.set(Some(prev_refetch));
    //         }
    //     })
    // }
}

impl<K, V> Query<K, V> {
    pub(crate) fn dispose(&self) {
        // self.state.dispose();
        // self.stale_time.dispose();
        // self.refetch_interval.dispose();
        // self.cache_time.dispose();
    }
}
