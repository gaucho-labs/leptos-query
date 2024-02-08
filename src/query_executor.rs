use leptos::*;
use std::{cell::Cell, future::Future, hash::Hash, rc::Rc};

use crate::{query::Query, QueryData, QueryState};

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
    V: std::fmt::Debug + Clone + 'static,
    Fu: Future<Output = V> + 'static,
{
    let fetcher = Rc::new(fetcher);
    move || {
        let fetcher = fetcher.clone();
        SUPPRESS_QUERY_LOAD.with(|supressed| {
            if !supressed.get() {
                spawn_local(async move {
                    let query = query.get_untracked();

                    match query.new_execution() {
                        None => {
                            logging::log!("Query already loading. Skipping.");
                        }
                        Some(execution) => {
                            match query.get_state() {
                                // Already loading.
                                QueryState::Loading | QueryState::Created => {
                                    query.set_state(QueryState::Loading);
                                    let data = fetcher(query.key.clone()).await;
                                    if !query.is_cancelled(execution) {
                                        let data = QueryData::now(data);
                                        query.set_state(QueryState::Loaded(data));
                                    } else {
                                        query.set_state(QueryState::Created);
                                    }
                                }
                                // Subsequent loads.
                                QueryState::Fetching(data)
                                | QueryState::Loaded(data)
                                | QueryState::Invalid(data) => {
                                    query.set_state(QueryState::Fetching(data));

                                    let new_data = fetcher(query.key.clone()).await;

                                    if !query.is_cancelled(execution) {
                                        let new_data = QueryData::now(new_data);
                                        query.set_state(QueryState::Loaded(new_data));
                                    } else {
                                        // If no other execution is active, then set state to Loaded.
                                        query.maybe_map_state(|state| match state {
                                            QueryState::Fetching(data) if !query.is_executing() => {
                                                Ok(QueryState::Loaded(data))
                                            }
                                            // Don't do anything to state.
                                            already_loaded @ (QueryState::Fetching(_)
                                            | QueryState::Loaded(_)
                                            | QueryState::Invalid(_)) => Err(already_loaded),
                                            QueryState::Loading | QueryState::Created => {
                                                unreachable!("Invalid State")
                                            }
                                        });
                                    }
                                }
                            }
                            query.finish_exec(execution)
                        }
                    }
                })
            }
        })
    }
}

// Refetch data once marked as invalid.

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
