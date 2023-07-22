use leptos::*;

use crate::*;

/// Read only Query Data.
/// Used when introspecting the query cache.
#[derive(Clone)]
pub struct QueryData<V>
where
    V: Clone + 'static,
{
    /// The current value of the query. None if it has not been fetched yet.
    pub data: Signal<Option<V>>,
    /// If the query is fetching for the first time.
    pub is_loading: Signal<bool>,
    /// If the query is currently fetching.
    pub is_fetching: Signal<bool>,
    /// If the query is considered stale.
    pub is_stale: Signal<bool>,
    /// The last time the query was updated. None if it has not been fetched yet.
    pub updated_at: Signal<Option<Instant>>,
    /// If the query should refetch on next usage.
    pub invalidated: Signal<bool>,
}

impl<V> QueryData<V>
where
    V: Clone + 'static,
{
    pub(crate) fn from_state<K: Clone>(cx: Scope, state: QueryState<K, V>) -> Self {
        let is_stale = create_rw_signal(cx, false);
        let data = state.value.into();
        let is_loading = state.fetching.into();
        let is_fetching = state.fetching.into();
        let is_stale = is_stale;
        let updated_at = state.updated_at.into();
        let invalidated = state.invalidated.into();

        cleanup_observers(cx, &state);
        sync_stale_signal(cx, state, is_stale);

        Self {
            data,
            is_loading,
            is_fetching,
            is_stale: is_stale.into(),
            updated_at,
            invalidated,
        }
    }
}

fn cleanup_observers<K, V>(cx: Scope, state: &QueryState<K, V>) {
    let observers = state.observers.clone();
    observers.set(observers.get() + 1);

    on_cleanup(cx, {
        let observers = observers.clone();
        move || {
            observers.set(observers.get() - 1);
        }
    });
}
