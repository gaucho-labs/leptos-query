use crate::{instant::Instant, query_state::QueryState};
use leptos::*;

/// Reactive query result.
#[derive(Clone)]
pub struct QueryResult<V>
where
    V: 'static,
{
    /// The current value of the query. None if it has not been fetched yet.
    pub data: Signal<Option<V>>,
    /// If the query is fetching for the first time.
    pub is_loading: Signal<bool>,
    /// If the query is considered stale.
    pub is_stale: Signal<bool>,
    /// If the query is currently refetching.
    pub is_refetching: Signal<bool>,
    /// The last time the query was updated. None if it has not been fetched yet.
    pub updated_at: Signal<Option<Instant>>,
    /// Refetch the query.
    pub refetch: SignalSetter<()>,
}
impl<V> QueryResult<V> {
    /// Refetch the query.
    pub fn refetch(&self) {
        self.refetch.set(())
    }
}

impl<V> QueryResult<V>
where
    V: Clone,
{
    pub(crate) fn from_state_signal<K: Clone>(
        cx: Scope,
        state: Signal<QueryState<K, V>>,
    ) -> QueryResult<V> {
        let data = Signal::derive(cx, move || {
            let state = state.get();
            let read = state.read(cx);
            read
        });
        let is_loading = Signal::derive(cx, move || state.get().is_loading(cx).get());
        let is_stale = Signal::derive(cx, move || state.get().is_stale(cx).get());
        let is_refetching = Signal::derive(cx, move || state.get().fetching.get());
        let updated_at = Signal::derive(cx, move || state.get().updated_at.get());
        let refetch = move |_: ()| state.get().refetch();

        QueryResult {
            data,
            is_loading,
            is_stale,
            is_refetching,
            updated_at,
            refetch: refetch.mapped_signal_setter(cx),
        }
    }

    pub(crate) fn from_state<K: Clone>(cx: Scope, state: QueryState<K, V>) -> QueryResult<V> {
        let is_loading = state.is_loading(cx);
        let is_stale = state.is_stale(cx);
        let is_refetching = state.fetching.into();
        let updated_at = state.updated_at.into();
        let refetch = {
            let state = state.clone();
            move |_: ()| state.refetch()
        };
        let data = Signal::derive(cx, move || state.read(cx));

        QueryResult {
            data,
            is_loading,
            is_stale,
            is_refetching,
            updated_at,
            refetch: refetch.mapped_signal_setter(cx),
        }
    }
}

impl<V: Copy> Copy for QueryResult<V> where V: 'static {}
