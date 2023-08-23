use leptos::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    future::Future,
    hash::Hash,
    rc::Rc,
};

use crate::{
    query::Query,
    use_query_client,
    util::{maybe_time_until_stale, time_until_stale, use_timeout},
    QueryData, QueryError, QueryState,
};

thread_local! {
    static SUPPRESS_QUERY_LOAD: Cell<bool> = Cell::new(false);
}

#[doc(hidden)]
pub fn suppress_query_load(suppress: bool) {
    SUPPRESS_QUERY_LOAD.with(|w| w.set(suppress));
}

// Create Executor function which will execute task in `spawn_local` and update state.
pub(crate) fn create_executor<K, V, E, Fu>(
    query: Signal<Query<K, V, E>>,
    fetcher: impl Fn(K) -> Fu + 'static,
) -> impl Fn() + Clone
where
    K: Clone + Hash + Eq + 'static,
    V: Clone + 'static,
    E: Clone + 'static,
    Fu: Future<Output = Result<V, E>> + 'static,
{
    let fetcher = Rc::new(fetcher);
    move || {
        let fetcher = fetcher.clone();
        SUPPRESS_QUERY_LOAD.with(|supressed| {
            if !supressed.get() {
                spawn_local(async move {
                    let query = query.get_untracked();
                    let data_state = query.state.get_untracked();
                    match data_state {
                        QueryState::Fetching(_) | QueryState::Loading | QueryState::Retrying(_) => {
                            ()
                        }
                        // First load.
                        QueryState::Created => {
                            query.state.set(QueryState::Loading);
                            let result = fetcher(query.key.clone()).await;
                            let updated_at = crate::Instant::now();
                            match result {
                                Ok(data) => {
                                    let data = QueryData { data, updated_at };
                                    query.state.set(QueryState::Loaded(data));
                                }
                                Err(error) => query.state.set(QueryState::Error(QueryError {
                                    error,
                                    error_count: 0,
                                    updated_at,
                                    prev_data: None,
                                })),
                            }
                        }
                        // Subsequent loads.
                        QueryState::Loaded(data) | QueryState::Invalid(data) => {
                            query.state.set(QueryState::Fetching(data.clone()));
                            let result = fetcher(query.key.clone()).await;
                            let updated_at = crate::Instant::now();
                            match result {
                                Ok(data) => {
                                    let data = QueryData { data, updated_at };
                                    query.state.set(QueryState::Loaded(data));
                                }
                                Err(error) => query.state.set(QueryState::Error(QueryError {
                                    error,
                                    error_count: 0,
                                    updated_at,
                                    prev_data: Some(data),
                                })),
                            }
                        }
                        QueryState::Error(prev_error) => {
                            query
                                .state
                                .set(QueryState::Retrying(prev_error.prev_data.clone()));
                            let result = fetcher(query.key.clone()).await;
                            let updated_at = crate::Instant::now();
                            match result {
                                Ok(data) => {
                                    let data = QueryData { data, updated_at };
                                    query.state.set(QueryState::Loaded(data));
                                }
                                Err(error) => query.state.set(QueryState::Error(QueryError {
                                    error,
                                    error_count: prev_error.error_count,
                                    updated_at,
                                    prev_data: prev_error.prev_data,
                                })),
                            }
                        }
                    }
                })
            }
        })
    }
}

// Start synchronization effects.
pub(crate) fn synchronize_state<K, V, E>(
    cx: Scope,
    query: Signal<Query<K, V, E>>,
    executor: impl Fn() + Clone + 'static,
) where
    K: Hash + Eq + Clone + 'static,
    V: Clone,
    E: Clone,
{
    ensure_not_stale(cx, query, executor.clone());
    ensure_not_invalid(cx, query, executor.clone());
    sync_refetch(cx, query, executor.clone());

    let query = Signal::derive(cx, move || Some(query.get()));
    synchronize_observer(cx, query);
}

pub(crate) fn synchronize_observer<K, V, E>(cx: Scope, query: Signal<Option<Query<K, V, E>>>)
where
    K: Hash + Eq + Clone + 'static,
    V: Clone,
    E: Clone,
{
    sync_observers(cx, query);
    ensure_cache_cleanup(cx, query);
}

/// On mount, ensure that the resource is not stale
fn ensure_not_stale<K: Clone, V: Clone, E: Clone>(
    cx: Scope,
    query: Signal<Query<K, V, E>>,
    executor: impl Fn() + Clone + 'static,
) {
    create_isomorphic_effect(cx, move |_| {
        let query = query.get();
        let stale_time = query.stale_time;

        if let (Some(updated_at), Some(stale_time)) = (
            query.state.get_untracked().updated_at(),
            stale_time.get_untracked(),
        ) {
            if time_until_stale(updated_at, stale_time).is_zero() {
                executor();
            }
        }
    })
}

/// Refetch data once marked as invalid.
fn ensure_not_invalid<K: Clone, V: Clone, E: Clone>(
    cx: Scope,
    state: Signal<Query<K, V, E>>,
    executor: impl Fn() + 'static,
) {
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();
        // Refetch query if Invalid.
        if let QueryState::Invalid(_) = state.state.get() {
            executor()
        }
    });
}

/// Effect for refetching query on interval, if present.
fn sync_refetch<K, V, E>(
    cx: Scope,
    query: Signal<Query<K, V, E>>,
    executor: impl Fn() + Clone + 'static,
) where
    K: Clone + 'static,
    V: Clone + 'static,
    E: Clone + 'static,
{
    let _ = use_timeout(cx, move || {
        let query = query.get();
        let updated_at = query.state.get().updated_at();
        let refetch_interval = query.refetch_interval.get();
        match (updated_at, refetch_interval) {
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

// Ensure that observers are kept track of.
fn sync_observers<K: Clone, V: Clone, E: Clone>(cx: Scope, query: Signal<Option<Query<K, V, E>>>) {
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
    create_isomorphic_effect(cx, move |observers: Option<Option<Rc<Cell<usize>>>>| {
        // Decrement previous observers.
        if let Some(observers) = observers.flatten() {
            last_observer.set(None);
            observers.set(observers.get() - 1);
        }
        // Deal with latest observers.
        if let Some(query) = query.get() {
            let observers = query.observers;
            last_observer.set(Some(observers.clone()));
            observers.set(observers.get() + 1);
            Some(observers)
        } else {
            None
        }
    });
}

/// This is a very finicky function. Be cautious with edits.
pub(crate) fn ensure_cache_cleanup<K, V, E>(cx: Scope, query: Signal<Option<Query<K, V, E>>>)
where
    K: Clone + Hash + Eq + 'static,
    V: Clone + 'static,
    E: Clone + 'static,
{
    let root_scope = use_query_client(cx).cx;

    let child_disposed = Rc::new(Cell::new(false));
    on_cleanup(cx, {
        let child_disposed = child_disposed.clone();
        move || child_disposed.set(true)
    });

    // Keep track of existing timeouts for keys.
    let timeout_map = Rc::new(RefCell::new(HashMap::<K, Box<dyn Fn()>>::new()));

    // Functions that should be run on scope cleanup.
    let cleanup_map = Rc::new(RefCell::new(HashMap::<K, Box<dyn FnOnce()>>::new()));
    on_cleanup(cx, {
        let key_to_on_cleanup = cleanup_map.clone();
        move || {
            let mut map = key_to_on_cleanup.borrow_mut();
            map.drain().for_each(|(_, cleanup)| cleanup());
        }
    });

    // Create outer effect with child scope, and create timeout on root scope.
    create_effect(cx, move |_| {
        // These signals can't go inside use_timeout because they will be disposed of before the timeout executes.
        if let Some(query) = query.get() {
            let updated_at = query.state.get().updated_at();
            let cache_time = query.cache_time.get();

            // Remove key from cleanup map.
            {
                let mut cleanup_map = cleanup_map.borrow_mut();
                cleanup_map.remove(&query.key);
                drop(cleanup_map);
            }

            // Clear previous timeout for key.
            let mut timeout_map = timeout_map.borrow_mut();
            if let Some(clear) = timeout_map.remove(&query.key) {
                clear()
            }

            let child_disposed = child_disposed.clone();
            let cleanup_map = cleanup_map.clone();

            // use_timeout ensures no leaky timeouts. Old timeout is always cleared.
            let clear_timeout = use_timeout(root_scope, {
                let query = query.clone();
                move || {
                    if let Some(timeout) = maybe_time_until_stale(updated_at, cache_time) {
                        let child_disposed = child_disposed.clone();
                        let cleanup_map = cleanup_map.clone();
                        let query = query.clone();

                        set_timeout_with_handle(
                            move || {
                                // Remove from cache & dispose.
                                let dispose = {
                                    let query = query.clone();
                                    move || {
                                        let removed = use_query_client(root_scope)
                                            .evict_and_notify::<K, V, E>(&query.key);
                                        if let Some(query) = removed {
                                            if query.observers.get() == 0 {
                                                query.dispose();
                                                drop(query)
                                            }
                                        }
                                    }
                                };

                                // Check if scope has been disposed, or there are no observers.
                                if child_disposed.get() || query.observers.get() == 0 {
                                    // Dispose immediately.
                                    dispose();
                                } else {
                                    // Add cleanup function.
                                    let mut map = cleanup_map.borrow_mut();
                                    map.insert(query.key.clone(), Box::new(dispose));
                                }
                            },
                            timeout,
                        )
                        .ok()
                    } else {
                        None
                    }
                }
            });

            timeout_map.insert(query.key.clone(), Box::new(clear_timeout));
        }
    });
}
