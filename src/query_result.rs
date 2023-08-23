use std::time::Duration;

use crate::{
    query::Query,
    util::{maybe_time_until_stale, use_timeout},
    QueryState,
};
use leptos::*;

/// Reactive query result.
#[derive(Clone)]
pub struct QueryResult<V, E, R>
where
    V: 'static,
    E: 'static,
    R: RefetchFn,
{
    /// The current value of the query. None if it has not been fetched yet.
    /// Should be called inside of a [`Transition`](leptos::Transition) or [`Suspense`](leptos::Suspense) component.
    pub data: Signal<Option<V>>,
    /// The current state of the data.
    pub state: Signal<QueryState<V, E>>,

    /// If the query is fetching for the first time.
    pub is_loading: Signal<bool>,
    /// If the query is actively fetching.
    pub is_fetching: Signal<bool>,
    /// If the query data is considered stale.
    pub is_stale: Signal<bool>,
    /// If the query data has been marked as invalid.
    pub is_invalid: Signal<bool>,

    /// Refetch the query.
    pub refetch: R,
}

/// Convenience Trait alias for a Query Result's refetch function.
pub trait RefetchFn: Fn() + Clone {}
impl<R: Fn() + Clone> RefetchFn for R {}

pub(crate) fn create_query_result<K: Clone, E: Clone, V: Clone>(
    cx: Scope,
    query: Signal<Query<K, V, E>>,
    data: Signal<Option<V>>,
    executor: impl Fn() + Clone,
) -> QueryResult<V, E, impl RefetchFn> {
    let state = Signal::derive(cx, move || query.get().state.get());

    let is_loading = Signal::derive(cx, move || matches!(state.get(), QueryState::Loading));
    let is_fetching = Signal::derive(cx, move || {
        matches!(state.get(), QueryState::Loading | QueryState::Fetching(_))
    });
    let is_invalid = Signal::derive(cx, move || matches!(state.get(), QueryState::Invalid(_)));

    // Make stale time.
    let stale_time = Signal::derive(cx, move || query.get().stale_time.get());
    let is_stale = make_is_stale(cx, state, stale_time);

    QueryResult {
        data,
        state,
        is_loading,
        is_fetching,
        is_stale,
        is_invalid,
        refetch: executor,
    }
}

fn make_is_stale<V: Clone, E: Clone>(
    cx: Scope,
    state: Signal<QueryState<V, E>>,
    stale_time: Signal<Option<Duration>>,
) -> Signal<bool> {
    let (stale, set_stale) = create_signal(cx, false);

    let _ = use_timeout(cx, move || {
        match maybe_time_until_stale(state.get().updated_at(), stale_time.get()) {
            Some(Duration::ZERO) => {
                set_stale.set(true);
                None
            }
            Some(timeout) => {
                set_stale.set(false);
                set_timeout_with_handle(
                    move || {
                        set_stale.set(true);
                    },
                    timeout,
                )
                .ok()
            }
            None => None,
        }
    });

    stale.into()
}
