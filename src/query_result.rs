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
    pub(crate) fn new<K: Clone>(
        cx: Scope,
        state: Memo<QueryState<K, V>>,
        data: Signal<Option<V>>,
        is_loading: Signal<bool>,
        refetch: Rc<dyn Fn() -> ()>,
    ) -> QueryResult<V> {
        let is_stale = make_stale_signal(cx, state);
        let is_refetching = Signal::derive(cx, move || state.get().fetching.get());
        let updated_at = Signal::derive(cx, move || state.get().updated_at.get());
        let refetch = move |_: ()| refetch();

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

fn make_stale_signal<K: Clone, V: Clone>(cx: Scope, state: Memo<QueryState<K, V>>) -> Signal<bool> {
    let (stale, set_stale) = create_signal(cx, false);
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();
        let updated_at = state.updated_at;
        let stale_time = state.stale_time;

        use_timeout(cx, move || match (updated_at.get(), stale_time.get()) {
            (Some(updated_at), Some(stale_time)) => {
                let timeout = time_until_stale(updated_at, stale_time);
                if timeout.is_zero() {
                    set_stale.set(true);
                    None
                } else {
                    set_stale.set(false);
                    set_timeout_with_handle(
                        move || {
                            set_stale.set(true);
                        },
                        timeout,
                    )
                    .ok()
                }
            }
            _ => None,
        })
    });

    stale.into()
}
