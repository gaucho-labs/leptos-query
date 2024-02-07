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
    QueryData, QueryState,
};

thread_local! {
    static SUPPRESS_QUERY_LOAD: Cell<bool> = Cell::new(false);
}

/// Disable or enable query loading.
///
/// Useful for disabling query loads during App introspection, such as SSR Router integrations for Actix/Axum.
///
/// Example for `generate_route_list`
/// ```
/// // Disable query loading.
/// leptos_query::suppress_query_load(true);
/// // Introspect App Routes.
/// leptos_axum::generate_route_list(|| view! { <App/> }).await;
/// // Enable query loading.
/// leptos_query::suppress_query_load(false);
/// ```

pub fn suppress_query_load(suppress: bool) {
    SUPPRESS_QUERY_LOAD.with(|w| w.set(suppress));
}

// Create Executor function which will execute task in `spawn_local` and update state.
pub(crate) fn create_executor<K, V, Fu>(
    query: Signal<Query<K, V>>,
    fetcher: impl Fn(K) -> Fu + 'static,
) -> impl Fn() + Clone
where
    K: Clone + Hash + Eq + 'static,
    V: Clone + 'static,
    Fu: Future<Output = V> + 'static,
{
    let fetcher = Rc::new(fetcher);
    move || {
        let fetcher = fetcher.clone();
        SUPPRESS_QUERY_LOAD.with(|supressed| {
            if !supressed.get() {
                spawn_local(async move {
                    let query = query.get_untracked();
                    let data_state = query.get_state();
                    match data_state {
                        // Already loading.
                        QueryState::Fetching(_) | QueryState::Loading => {}
                        // First load.
                        QueryState::Created => {
                            query.set_state(QueryState::Loading);
                            let data = fetcher(query.key.clone()).await;
                            let data = QueryData::now(data);
                            query.set_state(QueryState::Loaded(data));
                        }
                        // Subsequent loads.
                        QueryState::Loaded(data) | QueryState::Invalid(data) => {
                            logging::log!("Refetching");
                            query.set_state(QueryState::Fetching(data));

                            let new_data = fetcher(query.key.clone()).await;

                            // Check if query was cancelled.
                            if query.with_state(|state| matches!(state, QueryState::Fetching(_))) {
                                let data = QueryData::now(new_data);
                                query.set_state(QueryState::Loaded(data));
                            }
                        }
                    }
                })
            }
        })
    }
}

// Start synchronization effects.
// pub(crate) fn synchronize_state<K, V>(
//     query: Signal<Query<K, V>>,
//     executor: impl Fn() + Clone + 'static,
// ) where
//     K: Hash + Eq + Clone + 'static,
//     V: Clone,
// {
//     ensure_not_stale(query, executor.clone());
//     ensure_not_invalid(query, executor.clone());
//     sync_refetch(query, executor.clone());
//     let query = Signal::derive(move || Some(query.get()));
//     synchronize_observer(query);
// }

// pub(crate) fn synchronize_observer<K, V>(query: Signal<Option<Query<K, V>>>)
// where
//     K: Hash + Eq + Clone + 'static,
//     V: Clone,
// {
//     sync_observers(query);
//     ensure_cache_cleanup(query);
// }

// On mount, ensure that the resource is not stale
// fn ensure_not_stale<K: Clone, V: Clone>(
//     query: Signal<Query<K, V>>,
//     executor: impl Fn() + Clone + 'static,
// ) {
//     create_isomorphic_effect(move |_| {
//         let query = query.get();
//         let stale_time = query.stale_time;

//         if let (Some(updated_at), Some(stale_time)) = (
//             query.state.get_untracked().updated_at(),
//             stale_time.get_untracked(),
//         ) {
//             if time_until_stale(updated_at, stale_time).is_zero() {
//                 executor();
//             }
//         }
//     });
// }

// Refetch data once marked as invalid.
// fn ensure_not_invalid<K: Clone, V: Clone>(
//     query: Signal<Query<K, V>>,
//     executor: impl Fn() + 'static,
// ) {
//     create_effect(move |_| {
//         let query = query.get();
//         // Refetch query if Invalid.
//         if query
//             .state
//             .with(|state| matches!(state, QueryState::Invalid(_)))
//         {
//             executor()
//         }
//     });
// }

// Effect for refetching query on interval, if present.
// fn sync_refetch<K, V>(query: Signal<Query<K, V>>, executor: impl Fn() + Clone + 'static)
// where
//     K: Clone + 'static,
//     V: Clone + 'static,
// {
//     let _ = use_timeout(move || {
//         let query = query.get();
//         let updated_at = query.state.get().updated_at();
//         let refetch_interval = query.refetch_interval.get();
//         match (updated_at, refetch_interval) {
//             (Some(updated_at), Some(refetch_interval)) => {
//                 let executor = executor.clone();
//                 let timeout = time_until_stale(updated_at, refetch_interval);
//                 set_timeout_with_handle(
//                     move || {
//                         executor();
//                     },
//                     timeout,
//                 )
//                 .ok()
//             }
//             _ => None,
//         }
//     });
// }

// Ensure that observers are kept track of.
// fn sync_observers<K: Clone, V: Clone>(query: Signal<Option<Query<K, V>>>) {
//     type Observer = Rc<Cell<usize>>;
//     let last_observer: Rc<Cell<Option<Observer>>> = Rc::new(Cell::new(None));

//     on_cleanup({
//         let last_observer = last_observer.clone();
//         move || {
//             if let Some(observer) = last_observer.take() {
//                 observer.set(observer.get() - 1);
//             }
//         }
//     });

//     // Ensure that observers are kept track of.
//     create_isomorphic_effect(move |observers: Option<Option<Rc<Cell<usize>>>>| {
//         // Decrement previous observers.
//         if let Some(observers) = observers.flatten() {
//             last_observer.set(None);
//             observers.set(observers.get() - 1);
//         }
//         // Deal with latest observers.
//         if let Some(query) = query.get() {
//             let observers = query.observers;
//             last_observer.set(Some(observers.clone()));
//             observers.set(observers.get() + 1);
//             Some(observers)
//         } else {
//             None
//         }
//     });
// }

// This is a very finicky function. Be cautious with edits.
// pub(crate) fn ensure_cache_cleanup<K, V>(query: Signal<Option<Query<K, V>>>)
// where
//     K: Clone + Hash + Eq + 'static,
//     V: Clone + 'static,
// {
//     let owner = use_query_client().owner;

//     let child_disposed = Rc::new(Cell::new(false));
//     on_cleanup({
//         let child_disposed = child_disposed.clone();
//         move || child_disposed.set(true)
//     });

//     // Keep track of existing timeouts for keys.
//     let timeout_map = Rc::new(RefCell::new(HashMap::<K, Box<dyn Fn()>>::new()));

//     // Functions that should be run on scope cleanup.
//     let cleanup_map = Rc::new(RefCell::new(HashMap::<K, Box<dyn FnOnce()>>::new()));
//     on_cleanup({
//         let key_to_on_cleanup = cleanup_map.clone();
//         move || {
//             let mut map = key_to_on_cleanup.borrow_mut();
//             map.drain().for_each(|(_, cleanup)| cleanup());
//         }
//     });

//     // Create outer effect with child scope, and create timeout on root scope.
//     create_effect(move |_| {
//         // These signals can't go inside use_timeout because they will be disposed of before the timeout executes.
//         if let Some(query) = query.get() {
//             let updated_at = query.state.get().updated_at();
//             let cache_time = query.cache_time.get();

//             // Remove key from cleanup map.
//             {
//                 let mut cleanup_map = cleanup_map.borrow_mut();
//                 cleanup_map.remove(&query.key);
//                 drop(cleanup_map);
//             }

//             // Clear previous timeout for key.
//             let mut timeout_map = timeout_map.borrow_mut();
//             if let Some(clear) = timeout_map.remove(&query.key) {
//                 clear()
//             }

//             let child_disposed = child_disposed.clone();
//             let cleanup_map = cleanup_map.clone();

//             // use_timeout ensures no leaky timeouts. Old timeout is always cleared.
//             let clear_timeout = with_owner(owner, {
//                 let query = query.clone();
//                 move || {
//                     use_timeout({
//                         let query = query.clone();
//                         move || {
//                             if let Some(timeout) = maybe_time_until_stale(updated_at, cache_time) {
//                                 let child_disposed = child_disposed.clone();
//                                 let cleanup_map = cleanup_map.clone();
//                                 let query = query.clone();

//                                 set_timeout_with_handle(
//                                     move || {
//                                         // Remove from cache & dispose.
//                                         let dispose = {
//                                             let query = query.clone();
//                                             move || {
//                                                 let removed = use_query_client()
//                                                     .evict_and_notify::<K, V>(&query.key);
//                                                 if let Some(query) = removed {
//                                                     if query.observers.get() == 0 {
//                                                         query.dispose();
//                                                         drop(query)
//                                                     }
//                                                 }
//                                             }
//                                         };

//                                         // Check if scope has been disposed. If it has, then dispose immediately. Otherwise add cleanup function.
//                                         if child_disposed.get() {
//                                             dispose();
//                                         } else {
//                                             let mut map = cleanup_map.borrow_mut();
//                                             map.insert(query.key.clone(), Box::new(dispose));
//                                         }
//                                     },
//                                     timeout,
//                                 )
//                                 .ok()
//                             } else {
//                                 None
//                             }
//                         }
//                     })
//                 }
//             });

//             timeout_map.insert(query.key.clone(), Box::new(clear_timeout));
//         }
//     });
// }
