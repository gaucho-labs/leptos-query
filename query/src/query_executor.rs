use futures_channel::oneshot;
use leptos::*;
use std::{cell::Cell, future::Future, pin::pin, rc::Rc};

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
/// use leptos::*;
/// use leptos_query::*;
/// use leptos_axum::*;
///
/// fn make_routes()  {
///     // Disable query loading.
///     leptos_query::suppress_query_load(true);
///     // Introspect App Routes.
///     leptos_axum::generate_route_list(App);
///     // Enable query loading.
///     leptos_query::suppress_query_load(false);
/// }
///
/// #[component]
/// fn App() -> impl IntoView {
///     ()
/// }
///
///
///
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
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
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
                        None => {}
                        Some(cancellation) => {
                            match query.get_state() {
                                // Already loading.
                                QueryState::Loading | QueryState::Created => {
                                    query.set_state(QueryState::Loading);
                                    let fetch = pin!(fetcher(query.key.clone()));
                                    match execute_with_cancellation(fetch, cancellation).await {
                                        Ok(data) => {
                                            let data = QueryData::now(data);
                                            query.set_state(QueryState::Loaded(data));
                                        }
                                        Err(_) => {
                                            logging::error!("Failed to await!");
                                            query.set_state(QueryState::Created);
                                        }
                                    }
                                }
                                // Subsequent loads.
                                QueryState::Fetching(data)
                                | QueryState::Loaded(data)
                                | QueryState::Invalid(data) => {
                                    query.set_state(QueryState::Fetching(data));
                                    let fetch = pin!(fetcher(query.key.clone()));
                                    match execute_with_cancellation(fetch, cancellation).await {
                                        Ok(data) => {
                                            let data = QueryData::now(data);
                                            query.set_state(QueryState::Loaded(data));
                                        }
                                        Err(_) => {
                                            query.maybe_map_state(|state| {
                                                if let QueryState::Fetching(data) = state {
                                                    Ok(QueryState::Loaded(data))
                                                } else {
                                                    Err(state)
                                                }
                                            });
                                        }
                                    }
                                }
                            }
                            query.finalize_execution()
                        }
                    }
                })
            }
        })
    }
}

async fn execute_with_cancellation<V, Fu>(
    fut: Fu,
    cancellation: oneshot::Receiver<()>,
) -> Result<V, ()>
where
    Fu: Future<Output = V> + Unpin,
{
    cfg_if::cfg_if! {
        if #[cfg(any(feature = "hydrate", feature = "csr"))] {
            use futures::future::Either;

            let result = futures::future::select(fut, cancellation).await;

            match result {
                Either::Left((result, _)) => Ok(result),
                Either::Right((cancelled ,_)) => {
                    if let Err(_) = cancelled {
                        logging::debug_warn!("Query cancellation was incorrectly dropped.");
                    }

                    Err(())
                },
            }
        // No cancellation on server side.
        } else {
            let _ = cancellation;
            let result = fut.await;
            Ok(result)
        }
    }
}
