use std::rc::Rc;

use crate::{
    instant::Instant,
    query_state::QueryState,
    util::{time_until_stale, use_timeout},
};
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
    /// If the query is currently fetching.
    pub is_fetching: Signal<bool>,
    /// If the query is considered stale.
    pub is_stale: Signal<bool>,
    /// The last time the query was updated. None if it has not been fetched yet.
    pub updated_at: Signal<Option<Instant>>,
    /// If the query should refetch on next usage.
    pub invalidated: Signal<bool>,
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
    pub(crate) fn from_resource<K: Clone>(
        cx: Scope,
        state: Signal<QueryState<K, V>>,
        data: Signal<Option<V>>,
        is_loading: Signal<bool>,
        executor: Rc<dyn Fn()>,
    ) -> QueryResult<V> {
        let is_stale = make_stale_signal(cx, state);
        let is_fetching = Signal::derive(cx, move || state.get().fetching.get());
        let updated_at = Signal::derive(cx, move || state.get().updated_at.get());
        let invalidated = Signal::derive(cx, move || state.get().invalidated.get());
        let refetch = move |_: ()| executor();

        QueryResult {
            data,
            is_loading,
            is_stale,
            is_fetching,
            updated_at,
            invalidated,
            refetch: refetch.mapped_signal_setter(cx),
        }
    }

    pub(crate) fn from_state<K: Clone>(
        cx: Scope,
        state: Signal<QueryState<K, V>>,
        executor: Rc<dyn Fn()>,
    ) -> QueryResult<V> {
        let data = Signal::derive(cx, move || state.get().value.get());
        let is_stale = make_stale_signal(cx, state);
        let is_fetching = Signal::derive(cx, move || state.get().fetching.get());
        let is_loading = Signal::derive(cx, move || {
            let state = state.get();
            state.value.get().is_none() && state.fetching.get()
        });
        let updated_at = Signal::derive(cx, move || state.get().updated_at.get());
        let invalidated = Signal::derive(cx, move || state.get().invalidated.get());
        let refetch = move |_: ()| executor();

        QueryResult {
            data,
            is_loading,
            is_stale,
            is_fetching,
            updated_at,
            invalidated,
            refetch: refetch.mapped_signal_setter(cx),
        }
    }
}

impl<V: Copy> Copy for QueryResult<V> where V: 'static {}

fn make_stale_signal<K: Clone, V: Clone>(
    cx: Scope,
    state: Signal<QueryState<K, V>>,
) -> Signal<bool> {
    let stale = create_rw_signal(cx, false);
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();
        sync_stale_signal(cx, state, stale)
    });

    stale.into()
}

pub(crate) fn sync_stale_signal<K: Clone, V: Clone>(
    cx: Scope,
    state: QueryState<K, V>,
    stale: RwSignal<bool>,
) {
    let updated_at = state.updated_at;
    let stale_time = state.stale_time;

    use_timeout(cx, move || match (updated_at.get(), stale_time.get()) {
        (Some(updated_at), Some(stale_time)) => {
            let timeout = time_until_stale(updated_at, stale_time);
            if timeout.is_zero() {
                stale.set(true);
                None
            } else {
                stale.set(false);
                set_timeout_with_handle(
                    move || {
                        stale.set(true);
                    },
                    timeout,
                )
                .ok()
            }
        }
        _ => None,
    })
}
