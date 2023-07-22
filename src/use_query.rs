use crate::instant::get_instant;
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
/// use leptos::*;
/// use leptos_query::*;
/// use std::time::Duration;
/// use serde::*;
///
/// // Data type.
/// #[derive(Clone, Deserialize, Serialize)]
/// struct Monkey {
///     name: String,
/// }
///
/// // Create a Newtype for MonkeyId.
/// #[derive(Clone, PartialEq, Eq, Hash)]
/// struct MonkeyId(String);
///
/// // Monkey fetcher.
/// async fn get_monkey(id: MonkeyId) -> Monkey {
///     todo!()
/// }
///
/// // Query for a Monkey.
/// fn use_monkey_query(cx: Scope, id: impl Fn() -> MonkeyId + 'static) -> QueryResult<Monkey> {
///     leptos_query::use_query(
///         cx,
///         id,
///         get_monkey,
///         QueryOptions {
///             default_value: None,
///             refetch_interval: None,
///             resource_option: ResourceOption::NonBlocking,
///             stale_time: Some(Duration::from_secs(5)),
///             cache_time: Some(Duration::from_secs(60)),
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
    V: Clone + Serializable + 'static,
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
            if state.fetching.get_untracked() || state.value.get_untracked().is_none() {
                // Suspend indefinitely and wait for interruption.
                sleep(LONG_TIME).await;
                ResourceData(None)
            } else {
                ResourceData(state.value.get_untracked())
            }
        }
    };

    let resource: Resource<QueryState<K, V>, ResourceData<V>> = {
        let default = options.default_value;
        match options.resource_option {
            ResourceOption::NonBlocking => create_resource_with_initial_value(
                cx,
                move || state.get(),
                fetcher,
                if default.is_some() {
                    Some(ResourceData(default))
                } else {
                    None
                },
            ),
            ResourceOption::Blocking => create_blocking_resource(cx, move || state.get(), fetcher),
        }
    };

    // Ensure always latest value.
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();
        let value = state.value.get();
        if value.is_some() {
            // Interrupt suspense.
            if resource.loading().get_untracked() {
                resource.set(ResourceData(value));
            } else {
                resource.refetch();
            }
        }
    });

    let executor = create_executor(cx, state, query);

    // Ensure key changes are considered.
    create_isomorphic_effect(cx, {
        let executor = executor.clone();
        move |prev_state: Option<QueryState<K, V>>| {
            let state = state.get();
            if let Some(prev_state) = prev_state {
                if prev_state != state && state.needs_init() {
                    executor()
                }
            }
            state
        }
    });

    let data = Signal::derive(cx, {
        let executor = executor.clone();
        move || {
            let read = resource.read(cx).map(|r| r.0).flatten();
            let state = state.get_untracked();
            let updated_at = state.updated_at;

            // First Read.
            // Putting this in an effect will cause it to always refetch needlessly on the client after SSR.
            if read.is_none() && state.needs_init() {
                executor()
            // SSR edge case.
            // Given hydrate can happen before resource resolves, signals on the client can be out of sync with resource.
            } else if read.is_some() {
                if updated_at.get_untracked().is_none() {
                    updated_at.set(Some(get_instant()));
                }
                if state.value.get_untracked().is_none() {
                    state.value.set(read.clone());
                }
                if state.fetching.get_untracked() {
                    state.fetching.set(false);
                }
            }
            read
        }
    });

    // Doing this outside of QueryResult because of the need for `resource`.
    let is_loading = Signal::derive(cx, move || {
        let state = state.get();

        // Need to consider both because of SSR resource <-> signal mismatches.
        (resource.loading().get() || state.fetching.get()) && state.value.get().is_none()
    });

    QueryResult::new(cx, state, data, is_loading, executor)
}

const LONG_TIME: Duration = Duration::from_secs(60 * 60 * 24);

async fn sleep(duration: Duration) {
    use cfg_if::cfg_if;
    cfg_if! {
        if #[cfg(all(target_arch = "wasm32", any(feature = "hydrate")))] {
            gloo_timers::future::sleep(duration).await;
        } else if #[cfg(feature = "ssr")] {
            tokio::time::sleep(duration).await;
        } else {
            let _ = duration;
            debug_warn!("You are missing a Cargo feature for leptos_query. Please use one of 'ssr' or 'hydrate'")
        }
    }
}

/// Wrapper type to enable using `Serializable`
#[derive(Clone, Debug)]
struct ResourceData<V>(Option<V>);

impl<V> Serializable for ResourceData<V>
where
    V: Serializable,
{
    fn ser(&self) -> Result<String, SerializationError> {
        if let Some(ref value) = self.0 {
            value.ser()
        } else {
            Ok("null".to_string())
        }
    }

    fn de(bytes: &str) -> Result<Self, SerializationError> {
        match bytes {
            "" | "null" => Ok(ResourceData(None)),
            v => <V>::de(v).map(Some).map(ResourceData),
        }
    }
}
