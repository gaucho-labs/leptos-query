use std::{rc::Rc, time::Duration};

use crate::{
    query::Query,
    util::{maybe_time_until_stale, use_timeout},
    QueryState,
};
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

impl<V> Copy for QueryResult<V> where V: Clone + 'static {}

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
        query: Signal<Query<K, V>>,
        data: Signal<Option<V>>,
        executor: Rc<dyn Fn()>,
    ) -> QueryResult<V> {
        let refetch = { move |_: ()| executor() }.mapped_signal_setter(cx);

        QueryResult {
            cx,
            stale_time: Signal::derive(cx, move || query.get().stale_time.get()),
            data,
            state: Signal::derive(cx, move || query.get().state.get()),
            refetch,
        }
    }

    pub fn is_loading(&self) -> Signal<bool> {
        let state = self.state;
        Signal::derive(self.cx, move || matches!(state.get(), QueryState::Loading))
    }

    pub fn is_stale(&self) -> Signal<bool> {
        let state = self.state;
        let stale_time = self.stale_time;
        let (stale, set_stale) = create_signal(self.cx, false);

        let _ = use_timeout(self.cx, {
            move || {
                if let Some(timeout) =
                    maybe_time_until_stale(state.get().updated_at(), stale_time.get())
                {
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
                } else {
                    None
                }
            }
        });

        stale.into()
    }

    pub fn is_fetching(&self) -> Signal<bool> {
        let state = self.state;
        Signal::derive(self.cx, move || {
            matches!(state.get(), QueryState::Loading | QueryState::Fetching(_))
        })
    }

    pub fn invalidated(&self) -> Signal<bool> {
        let state = self.state;
        Signal::derive(self.cx, move || {
            matches!(state.get(), QueryState::Invalid(_))
        })
    }
}
