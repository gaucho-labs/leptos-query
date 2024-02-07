use leptos::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use crate::{query_observer::QueryObserver, QueryState};

#[derive(Clone)]
pub(crate) struct Query<K, V>
where
    K: 'static,
    V: 'static,
{
    pub(crate) key: K,
    observers: Rc<RefCell<Vec<QueryObserver<V>>>>,
    state: Rc<Cell<QueryState<V>>>,
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
            observers: Rc::new(RefCell::new(Vec::new())),
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
        self.state.set(state.clone());
        let observers = self.observers.borrow();
        for observer in observers.iter() {
            observer.update(state.clone())
        }
    }

    pub(crate) fn with_state<T>(&self, predicate: impl FnOnce(&QueryState<V>) -> T) -> T {
        let state = self.state.replace(QueryState::Created);
        let result = predicate(&state);
        self.state.set(state);
        result
    }

    pub(crate) fn map_state(&self, update_fn: impl FnOnce(QueryState<V>) -> QueryState<V>) {
        let state = self.state.replace(QueryState::Created);
        let new_state = update_fn(state);
        self.set_state(new_state);
    }

    pub(crate) fn maybe_map_state(
        &self,
        update_fn: impl FnOnce(QueryState<V>) -> Result<QueryState<V>, QueryState<V>>,
    ) {
        let current_state = self.state.replace(QueryState::Created);

        match update_fn(current_state) {
            Ok(new_state) => self.set_state(new_state),
            Err(old_state) => self.state.set(old_state),
        }
    }

    pub(crate) fn register_observer(&self) -> QueryObserver<V> {
        let mut observers = self.observers.try_borrow_mut().expect("register_observer");
        let observer_id = observers.last().map(|o| o.get_id()).unwrap_or_default();
        let observer = QueryObserver::new(observer_id, self.get_state());
        logging::log!("Updating observers {}", observers.len() + 1);
        observers.push(observer.clone());
        // drop(observers);
        // logging::log!("Dropping observers");
        observer
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
