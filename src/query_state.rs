use leptos::*;
use std::{cell::Cell, rc::Rc, time::Duration};

use crate::{ensure_valid_stale_time, instant::Instant, QueryOptions};

#[derive(Clone)]
pub(crate) struct QueryState<K, V>
where
    K: 'static,
    V: 'static,
{
    pub(crate) key: K,
    pub(crate) observers: Rc<Cell<usize>>,
    pub(crate) needs_refetch: Rc<Cell<bool>>,
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
    pub(crate) fn new(cx: Scope, key: K, options: QueryOptions<V>) -> Self {
        let stale_time = ensure_valid_stale_time(&options.stale_time, &options.cache_time);
        let stale_time = create_rw_signal(cx, stale_time);
        let refetch_interval = create_rw_signal(cx, options.refetch_interval);
        let value = create_rw_signal(cx, None);
        let updated_at = create_rw_signal(cx, None);
        let invalidated = create_rw_signal(cx, false);
        let fetching = create_rw_signal(cx, false);

        QueryState {
            key,
            observers: Rc::new(Cell::new(0)),
            needs_refetch: Rc::new(Cell::new(true)),
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

    /// If the query is being fetched for the first time.
    /// IMPORTANT: If the query is never [read](QueryState::read), this will always return false.
    pub(crate) fn is_loading(&self, cx: Scope) -> Signal<bool> {
        let updated_at = self.updated_at;
        let fetching = self.fetching;

        Signal::derive(cx, move || updated_at.get().is_none() && fetching.get())
    }

    pub(crate) fn is_loading_untracked(&self) -> bool {
        self.updated_at.get_untracked().is_none() && self.fetching.get_untracked()
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
