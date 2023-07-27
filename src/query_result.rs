use std::{rc::Rc, time::Duration};

use crate::{query::Query, util::time_until_stale, QueryState};
use leptos::*;

/// Reactive query result.
#[derive(Clone)]
pub struct QueryResult<V>
where
    V: 'static,
{
    cx: Scope,
    stale_time: Signal<Option<Duration>>,
    /// The current value of the query. None if it has not been fetched yet.
    pub data: Signal<Option<V>>,
    /// The current state of the data.
    pub state: Signal<QueryState<V>>,
    /// Refetch the query.
    pub refetch: SignalSetter<()>,
}

// TODO: Should this be signal or memo?

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
        state: Signal<Query<K, V>>,
        data: Signal<Option<V>>,
        executor: Rc<dyn Fn()>,
    ) -> QueryResult<V> {
        let refetch = { move |_: ()| executor() }.mapped_signal_setter(cx);

        QueryResult {
            cx,
            stale_time: Signal::derive(cx, move || state.get().stale_time.get()),
            data,
            state: Signal::derive(cx, move || state.get().data.get()),
            refetch,
        }
    }

    pub fn is_loading(&self) -> Signal<bool> {
        let state = self.state;
        Signal::derive(self.cx, move || match state.get() {
            QueryState::Loading => true,
            _ => false,
        })
    }

    pub fn is_stale(&self) -> Signal<bool> {
        let state = self.state;
        let stale_time = self.stale_time;
        Signal::derive(self.cx, move || {
            if let (Some(updated_at), Some(stale_time)) =
                (state.get().updated_at(), stale_time.get())
            {
                if time_until_stale(updated_at, stale_time).is_zero() {
                    return true;
                }
            }
            false
        })
    }

    pub fn is_fetching(&self) -> Signal<bool> {
        let state = self.state;
        Signal::derive(self.cx, move || match state.get() {
            QueryState::Loading | QueryState::Fetching(_) => true,
            _ => false,
        })
    }

    pub fn invalidated(&self) -> Signal<bool> {
        let state = self.state;
        Signal::derive(self.cx, move || match state.get() {
            QueryState::Invalid(_) => true,
            _ => false,
        })
    }

    // pub(crate) fn from_state<K: Clone>(
    //     cx: Scope,
    //     state: Signal<QueryState<K, V>>,
    //     executor: Rc<dyn Fn()>,
    // ) -> QueryResult<V> {
    //     let data = Signal::derive(cx, move || state.get().data.get());
    //     let is_stale = make_stale_signal(cx, state);
    //     let is_fetching = Signal::derive(cx, move || state.get().fetching.get());
    //     let is_loading = Signal::derive(cx, move || {
    //         let state = state.get();
    //         state.data.get().is_none() && state.fetching.get()
    //     });
    //     let updated_at = Signal::derive(cx, move || state.get().updated_at.get());
    //     let invalidated = Signal::derive(cx, move || state.get().invalidated.get());
    //     let refetch = move |_: ()| executor();

    //     QueryResult {
    //         data,
    //         is_loading,
    //         is_stale,
    //         is_fetching,
    //         updated_at,
    //         invalidated,
    //         refetch: refetch.mapped_signal_setter(cx),
    //     }
    // }
}

impl<V> Copy for QueryResult<V> where V: Clone + 'static {}
