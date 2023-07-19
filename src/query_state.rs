use leptos::*;
use std::{cell::Cell, rc::Rc, time::Duration};

use crate::{
    ensure_valid_stale_time,
    instant::Instant,
    util::{time_until_stale, use_timeout},
    QueryOptions,
};

#[derive(Clone)]
pub(crate) struct QueryState<K, V>
where
    K: 'static,
    V: 'static,
{
    pub(crate) key: K,
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
    pub(crate) fn new(cx: Scope, key: K, options: QueryOptions<V>) -> Self {
        let stale_time = ensure_valid_stale_time(&options.stale_time, &options.cache_time);

        QueryState {
            key,
            observers: Rc::new(Cell::new(0)),
            value: create_rw_signal(cx, None),
            stale_time: create_rw_signal(cx, stale_time),
            refetch_interval: create_rw_signal(cx, options.refetch_interval),
            updated_at: create_rw_signal(cx, None),
            invalidated: create_rw_signal(cx, false),
            fetching: create_rw_signal(cx, false),
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
