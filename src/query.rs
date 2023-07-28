use leptos::*;
use std::{cell::Cell, rc::Rc, time::Duration};

use crate::{ensure_valid_stale_time, QueryOptions, QueryState};

#[derive(Clone)]
pub(crate) struct Query<K, V>
where
    K: 'static,
    V: 'static,
{
    pub(crate) key: K,
    // State.
    pub(crate) observers: Rc<Cell<usize>>,
    pub(crate) state: RwSignal<QueryState<V>>,
    // Config.
    pub(crate) stale_time: RwSignal<Option<Duration>>,
    pub(crate) cache_time: RwSignal<Option<Duration>>,
    pub(crate) refetch_interval: RwSignal<Option<Duration>>,
}

impl<K: PartialEq, V> PartialEq for Query<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K: PartialEq, V> Eq for Query<K, V> {}

impl<K, V> Query<K, V>
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    pub(crate) fn new(cx: Scope, key: K) -> Self {
        let stale_time = create_rw_signal(cx, None);
        let cache_time = create_rw_signal(cx, None);
        let refetch_interval = create_rw_signal(cx, None);

        let state = create_rw_signal(cx, QueryState::Created);

        Query {
            key,
            observers: Rc::new(Cell::new(0)),
            state,
            stale_time,
            cache_time,
            refetch_interval,
        }
    }
}

impl<K, V> Query<K, V>
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    /// Marks the resource as invalid, which will cause it to be refetched on next read.
    pub(crate) fn mark_invalid(&self) {
        if let QueryState::Loaded(data) = self.state.get_untracked() {
            self.state.set(QueryState::Invalid(data))
        }
    }

    pub(crate) fn overwrite_options(&self, options: QueryOptions<V>) {
        let stale_time = ensure_valid_stale_time(&options.stale_time, &options.cache_time);

        self.stale_time.set(stale_time);
        self.cache_time.set(options.cache_time);
        self.refetch_interval.set(options.refetch_interval);
    }

    // Enables having different stale times & refetch intervals for the same query.
    // The lowest stale time & refetch interval will be used.
    // When the scope is dropped, the stale time & refetch interval will be reset to the previous value (if they existed).
    // Cache time behaves differently. It will only use the minimum cache time found.
    pub(crate) fn update_options(&self, cx: Scope, options: QueryOptions<V>) {
        // Use the minimum cache time.
        match (self.cache_time.get_untracked(), options.cache_time) {
            (Some(current), Some(new)) if new < current => self.cache_time.set(Some(new)),
            (None, Some(new)) => self.cache_time.set(Some(new)),
            _ => (),
        }

        let curr_stale = self.stale_time.get_untracked();
        let curr_refetch_interval = self.refetch_interval.get_untracked();

        let (prev_stale, new_stale) = match (curr_stale, options.stale_time) {
            (Some(current), Some(new)) if new < current => (Some(current), Some(new)),
            (None, Some(new)) => (None, Some(new)),
            _ => (None, None),
        };

        let (prev_refetch, new_refetch) = match (curr_refetch_interval, options.refetch_interval) {
            (Some(current), Some(new)) if new < current => (Some(current), Some(new)),
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
}

impl<K, V> Query<K, V> {
    pub(crate) fn dispose(&self) {
        self.state.dispose();
        self.stale_time.dispose();
        self.refetch_interval.dispose();
        self.cache_time.dispose();
    }
}
