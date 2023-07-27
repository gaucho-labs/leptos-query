use leptos::*;
use std::{cell::Cell, future::Future, hash::Hash, rc::Rc, time::Duration};

use crate::{
    instant::{get_instant, Instant},
    query::Query,
    use_cache, use_query_client,
    util::{time_until_stale, use_timeout},
    QueryData, QueryState,
};

thread_local! {
    static SUPPRESS_QUERY_LOAD: Cell<bool> = Cell::new(false);
}

#[doc(hidden)]
pub fn suppress_query_load(suppress: bool) {
    SUPPRESS_QUERY_LOAD.with(|w| w.set(suppress));
}

// Create Executor function which will execute task in `spawn_local` and update state.
pub(crate) fn create_executor<K, V, Fu>(
    state: Signal<Query<K, V>>,
    query: impl Fn(K) -> Fu + 'static,
) -> impl Fn()
where
    K: Clone + Hash + Eq + PartialEq + 'static,
    V: Clone + 'static,
    Fu: Future<Output = V> + 'static,
{
    let query = Rc::new(query);
    move || {
        let query = query.clone();
        SUPPRESS_QUERY_LOAD.with(|supressed| {
            if !supressed.get() {
                spawn_local(async move {
                    let state = state.get_untracked();
                    let data_state = state.data.get_untracked();
                    match data_state {
                        QueryState::Fetching(_) => (),
                        QueryState::Loading => {
                            let data = query(state.key.clone()).await;
                            let updated_at = get_instant();
                            let data = QueryData { data, updated_at };
                            state.data.set(QueryState::Loaded(data))
                        }
                        QueryState::Loaded(data)
                        | QueryState::Stale(data)
                        | QueryState::Invalid(data) => {
                            state.data.set(QueryState::Fetching(data));
                            let data = query(state.key.clone()).await;
                            let updated_at = get_instant();
                            let data = QueryData { data, updated_at };
                            state.data.set(QueryState::Loaded(data))
                        }
                    }
                })
            }
        })
    }
}

// Start synchronization effects.
pub(crate) fn synchronize_state<K, V>(cx: Scope, query: Signal<Query<K, V>>, executor: Rc<dyn Fn()>)
where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone,
{
    ensure_not_stale(cx, query, executor.clone());
    ensure_not_invalid(cx, query, executor.clone());
    sync_refetch(cx, query, executor.clone());
    sync_observers(cx, query);
    ensure_cache_cleanup(cx, query);
}

fn ensure_not_stale<K: Clone, V: Clone>(
    cx: Scope,
    query: Signal<Query<K, V>>,
    executor: Rc<dyn Fn()>,
) {
    create_isomorphic_effect(cx, move |_| {
        let query = query.get();
        let stale_time = query.stale_time;

        // On mount, ensure that the resource is not stale
        if let (Some(updated_at), Some(stale_time)) = (
            query.data.get_untracked().updated_at(),
            stale_time.get_untracked(),
        ) {
            if time_until_stale(updated_at, stale_time).is_zero() {
                executor();
            }
        }

        // Start timeout for marking data as stale.
        use_timeout(cx, {
            let state = query.clone();
            move || match (state.data.get().updated_at(), stale_time.get()) {
                (Some(updated_at), Some(stale_time)) => {
                    let timeout = time_until_stale(updated_at, stale_time);
                    if timeout.is_zero() {
                        state.mark_stale();
                        None
                    } else {
                        set_timeout_with_handle(
                            {
                                let state = state.clone();
                                move || {
                                    state.mark_stale();
                                }
                            },
                            timeout,
                        )
                        .ok()
                    }
                }
                _ => None,
            }
        });
    })
}

fn ensure_not_invalid<K: Clone, V: Clone>(
    cx: Scope,
    state: Signal<Query<K, V>>,
    executor: Rc<dyn Fn()>,
) {
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();

        // Refetch query if Invalid.
        create_isomorphic_effect(cx, {
            let executor = executor.clone();
            {
                move |_| match state.data.get() {
                    QueryState::Invalid(_) => executor(),
                    _ => (),
                }
            }
        });
    })
}

fn sync_refetch<K, V>(cx: Scope, state: Signal<Query<K, V>>, executor: Rc<dyn Fn()>)
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    create_effect(cx, {
        let executor = executor.clone();
        move |_| {
            let executor = executor.clone();

            let state = state.get();
            let refetch_interval = state.refetch_interval;

            // Effect for refetching query on interval.
            use_timeout(cx, {
                let executor = executor.clone();
                move || match (state.data.get().updated_at(), refetch_interval.get()) {
                    (Some(updated_at), Some(refetch_interval)) => {
                        let executor = executor.clone();
                        let timeout = time_until_stale(updated_at, refetch_interval);
                        set_timeout_with_handle(
                            move || {
                                executor();
                            },
                            timeout,
                        )
                        .ok()
                    }
                    _ => None,
                }
            });
        }
    })
}

// Ensure that observers are kept track of.
fn sync_observers<K: Clone, V: Clone>(cx: Scope, state: Signal<Query<K, V>>) {
    type Observer = Rc<Cell<usize>>;
    let last_observer: Rc<Cell<Option<Observer>>> = Rc::new(Cell::new(None));

    on_cleanup(cx, {
        let last_observer = last_observer.clone();
        move || {
            if let Some(observer) = last_observer.take() {
                observer.set(observer.get() - 1);
            }
        }
    });

    // Ensure that observers are kept track of.
    create_isomorphic_effect(cx, move |observers: Option<Rc<Cell<usize>>>| {
        // Decrement previous observers.
        if let Some(observers) = observers {
            last_observer.set(None);
            observers.set(observers.get() - 1);
        }
        // Deal with latest observers.
        let observers = state.get().observers;
        last_observer.set(Some(observers.clone()));
        observers.set(observers.get() + 1);
        observers
    });
}

fn ensure_cache_cleanup<K, V>(cx: Scope, query: Signal<Query<K, V>>)
where
    K: Clone + Hash + Eq + PartialEq + 'static,
    V: Clone + 'static,
{
    let root_scope = use_query_client(cx).cx;
    create_isomorphic_effect(cx, move |_| {
        let state = query.get();
        let key = state.key.clone();
        let observers = state.observers.clone();
        cache_cleanup::<K, V>(
            root_scope,
            key,
            state.data.into(),
            state.cache_time.get(),
            observers,
        );
    });
}

// Will cleanup the cache corresponding to the key when the cache_time has elapsed, and the query has not been updated.
fn cache_cleanup<K, V>(
    cx: Scope,
    key: K,
    state: Signal<QueryState<V>>,
    cache_time: Option<Duration>,
    observers: Rc<Cell<usize>>,
) where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + 'static,
{
    use_timeout(cx, move || match (state.get().updated_at(), cache_time) {
        (Some(last_updated), Some(cache_time)) => {
            let timeout = time_until_stale(last_updated, cache_time);
            let key = key.clone();
            let observers = observers.clone();
            set_timeout_with_handle(
                move || {
                    let removed = use_cache::<K, V, Option<Query<K, V>>>(cx, move |(_, cache)| {
                        cache.remove(&key)
                    });
                    if let Some(query) = removed {
                        if observers.get() == 0 {
                            query.dispose();
                            drop(query)
                        }
                    };
                },
                timeout,
            )
            .ok()
        }
        _ => None,
    });
}
