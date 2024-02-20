use crate::QueryState;
use leptos::*;

/// Reactive query result.
#[derive(Clone)]
pub struct QueryResult<V, R>
where
    V: 'static,
    R: RefetchFn,
{
    /// The current value of the query. None if it has not been fetched yet.
    /// Should be called inside of a [`Transition`](leptos::Transition) or [`Suspense`](leptos::Suspense) component.
    pub data: Signal<Option<V>>,
    /// The current state of the data.
    pub state: Signal<QueryState<V>>,
    /// If the query is fetching for the first time.
    pub is_loading: Signal<bool>,
    /// If the query is actively fetching.
    pub is_fetching: Signal<bool>,
    /// If the query data has been marked as invalid.
    pub is_invalid: Signal<bool>,

    /// Refetch the query.
    pub refetch: R,
}

/// Convenience Trait alias for a Query Result's refetch function.
pub trait RefetchFn: Fn() + Clone {}
impl<R: Fn() + Clone> RefetchFn for R {}
