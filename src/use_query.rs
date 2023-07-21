use crate::query_executor::create_executor;
use crate::query_result::QueryResult;
use crate::{use_cache, QueryClient, QueryOptions, QueryState, ResourceOption};
use leptos::*;
use std::collections::hash_map::Entry;
use std::future::Future;
use std::hash::Hash;
use std::time::Duration;

/// Provides a Query Client to the current scope.
pub fn provide_query_client(cx: Scope) {
    provide_context(cx, QueryClient::new(cx));
}

/// Creates a query. Useful for data fetching, caching, and synchronization with server state.
///
/// A Query provides:
/// - caching
/// - de-duplication
/// - invalidation
/// - background refetching
/// - refetch intervals
/// - memory management with cache lifetimes
///
///
/// Details:
/// - A query is unique per Key `K`.
/// - A query Key type `K` must only correspond to ONE UNIQUE Value `V` Type.
/// - Meaning a query Key type `K` cannot correspond to multiple Value `V` Types.
///
/// Example
/// ```
///
/// // Create a Newtype for MonkeyId.
/// #[derive(Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
/// struct MonkeyId(String);
///
///
/// // Monkey fetcher.
/// async fn get_monkey(id: MonkeyId) -> Monkey {
/// ...
/// }
///
/// // Query for a Monkey.
/// fn use_monkey_query(cx: Scope, id: MonkeyId) -> QueryResult<Monkey> {
///     leptos_query::use_query(
///         cx,
///         id,
///         get_monkey,
///         QueryOptions {
///             default_value: None,
///             refetch_interval: None,
///             resource_option: ResourceOption::NonBlocking,
///             stale_time: Some(Duration::from_secs(5)),
///             cache_time: Some(Duration::from_secs(30)),
///         },
///     )
/// }
///
/// ```
///
pub fn use_query<K, V, Fu>(
    cx: Scope,
    key: impl Fn() -> K + 'static,
    query: impl Fn(K) -> Fu + 'static,
    options: QueryOptions<V>,
) -> QueryResult<V>
where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone
        + Serializable
        + 'static
        + server_fn::serde::de::DeserializeOwned
        + server_fn::serde::Serialize,
    Fu: Future<Output = V> + 'static,
{
    let key = create_memo(cx, move |_| key());

    // Find relevant state.
    let state = create_memo(cx, {
        let options = options.clone();
        move |_| {
            use_cache(cx, {
                let options = options.clone();
                let key = key.get();
                move |(root_scope, cache)| {
                    let entry = cache.entry(key.clone());

                    let state = match entry {
                        Entry::Occupied(entry) => {
                            let entry = entry.into_mut();
                            // Enable nested options.
                            entry.set_options(cx, options);
                            entry
                        }
                        Entry::Vacant(entry) => {
                            let state = QueryState::new(root_scope, key, options);
                            entry.insert(state.clone())
                        }
                    };
                    state.clone()
                }
            })
        }
    });

    let fetcher = move |state: QueryState<K, V>| {
        async move {
            if state.value.get_untracked().is_none() || state.is_loading_untracked() {
                // Suspend indefinitely and wait for interruption.
                sleep(LONG_TIME).await;
                None
            } else {
                state.value.get_untracked()
            }
        }
    };

    let resource = {
        match options.resource_option {
            ResourceOption::NonBlocking => create_resource(cx, move || state.get(), fetcher),
            ResourceOption::Blocking => create_blocking_resource(cx, move || state.get(), fetcher),
        }
    };

    let callback = move || resource.refetch();
    let refetch = create_executor(cx, state, query, callback);

    create_isomorphic_effect(cx, {
        let refetch = refetch.clone();
        move |_| {
            let state = state.get();
            let value = state.value.get();
            // Fetch for first time if necessary.
            if state.needs_init() {
                refetch();
            // Update resource with latest value.
            } else if value.is_some() {
                resource.set(value);
            }
        }
    });

    // When key changes.
    let data = Signal::derive(cx, move || resource.read(cx).flatten());

    let is_loading = Signal::derive(cx, move || {
        let state = state.get();

        (resource.loading().get() || state.fetching.get()) && state.value.get().is_none()
    });

    QueryResult::new(cx, state, data, is_loading, refetch)
}

const LONG_TIME: Duration = Duration::from_secs(60 * 60 * 24);

async fn sleep(duration: Duration) {
    use cfg_if::cfg_if;
    cfg_if! {
        if #[cfg(all(target_arch = "wasm32", any(feature = "hydrate")))] {
            gloo_timers::future::sleep(duration).await;
        }
         else if #[cfg(feature = "ssr")] {
            tokio::time::sleep(duration).await;
        }  else {
            debug_warn!("You are missing a Cargo feature for leptos_query. Please use one of 'ssr' or 'hydrate'")
        }
    }
}
