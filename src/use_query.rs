use crate::query_executor::{create_executor, synchronize_state};
use crate::query_result::QueryResult;
use crate::{
    create_query_result, use_query_client, Query, QueryData, QueryError, QueryOptions, QueryState,
    RefetchFn, ResourceOption,
};
use leptos::*;
use std::future::Future;
use std::hash::Hash;
use std::time::Duration;

/// Creates a query. Useful for data fetching, caching, and synchronization with server state.
///
/// A Query provides:
/// - Caching
/// - De-duplication
/// - Invalidation
/// - Background refetching
/// - Refetch intervals
/// - Memory management with cache lifetimes
///
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
/// // Monkey fetcher.
/// async fn get_monkey(id: String) -> Monkey {
///     todo!()
/// }
///
/// // Query for a Monkey.
/// fn use_monkey_query(cx: Scope, id: impl Fn() -> String + 'static) -> QueryResult<Monkey, impl RefetchFn> {
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
    fetcher: impl Fn(K) -> Fu + 'static,
    options: QueryOptions<V, Never>,
) -> QueryResult<V, Never, impl RefetchFn>
where
    K: Hash + Eq + Clone + 'static,
    V: Clone + Serializable + 'static,
    Fu: Future<Output = V> + 'static,
{
    let fetcher = std::rc::Rc::new(fetcher);
    let fetcher = move |key: K| {
        let fetcher = fetcher.clone();
        async move { Ok(fetcher(key).await) as Result<V, Never> }
    };

    use_query_with_retry(cx, key, fetcher, options)
}

pub fn use_query_with_retry<K, V, E, Fu>(
    cx: Scope,
    key: impl Fn() -> K + 'static,
    fetcher: impl Fn(K) -> Fu + 'static,
    options: QueryOptions<V, E>,
) -> QueryResult<V, E, impl Fn() + Clone>
where
    K: Hash + Eq + Clone + 'static,
    V: Clone + Serializable + 'static,
    E: Clone + Serializable + 'static,
    Fu: Future<Output = Result<V, E>> + 'static,
{
    // Find relevant state.
    let query = use_query_client(cx).get_query_signal::<K, V, E>(cx, key);

    // Update options.
    create_isomorphic_effect(cx, {
        let options = options.clone();
        move |_| {
            let (query, new) = query.get();
            if new {
                query.overwrite_options(options.clone())
            } else {
                query.update_options(cx, options.clone())
            }
        }
    });

    let query = Signal::derive(cx, move || query.get().0);

    let resource_fetcher = move |query: Query<K, V, E>| {
        async move {
            match query.state.get_untracked().result() {
                // Immediately provide cached value.
                Some(data) => ResourceData(Some(data)),

                // Suspend indefinitely and wait for interruption.
                None => {
                    sleep(LONG_TIME).await;
                    ResourceData(None)
                }
            }
        }
    };

    let QueryOptions {
        default_value,
        retry,
        ..
    } = options;

    let resource: Resource<Query<K, V, E>, ResourceData<V, E>> = {
        match options.resource_option {
            ResourceOption::NonBlocking => create_resource_with_initial_value(
                cx,
                move || query.get(),
                resource_fetcher,
                default_value.map(|default| ResourceData(Some(Ok(default)))),
            ),
            ResourceOption::Blocking => {
                create_blocking_resource(cx, move || query.get(), resource_fetcher)
            }
        }
    };

    // Ensure always latest value.
    create_isomorphic_effect(cx, move |_| {
        let state = query.get().state.get();
        if let QueryState::Loaded(data) = state {
            // Interrupt Suspense.
            if resource.loading().get_untracked() {
                resource.set(ResourceData(Some(Ok(data.data))));
            } else {
                resource.refetch();
            }
        }
    });

    let executor = create_executor(query, fetcher);

    synchronize_state(cx, query, executor.clone(), retry);

    // Ensure key changes are considered.
    create_isomorphic_effect(cx, {
        let executor = executor.clone();
        move |prev_query: Option<Query<K, V, E>>| {
            let query = query.get();
            if let Some(prev_query) = prev_query {
                if prev_query != query {
                    if let QueryState::Created = query.state.get_untracked() {
                        executor()
                    }
                }
            }
            query
        }
    });

    let data = Signal::derive(cx, {
        let executor = executor.clone();
        move || {
            let read = resource.read(cx).and_then(|r| r.0);
            let query = query.get_untracked();

            // First Read.
            // Putting this in an effect will cause it to always refetch needlessly on the client after SSR.
            if read.is_none() && matches!(query.state.get_untracked(), QueryState::Created) {
                executor()
            // SSR edge case.
            // Given hydrate can happen before resource resolves, signals on the client can be out of sync with resource.
            } else if let Some(Ok(ref data)) = read {
                if let QueryState::Created = query.state.get_untracked() {
                    let updated_at = crate::Instant::now();
                    let data = QueryData {
                        data: data.clone(),
                        updated_at,
                    };
                    query.state.set(QueryState::Loaded(data))
                }
            }
            read
        }
    });

    create_query_result(cx, query, data, executor)
}

const LONG_TIME: Duration = Duration::from_secs(60 * 60 * 24);

async fn sleep(duration: Duration) {
    use cfg_if::cfg_if;
    cfg_if! {
        if #[cfg(feature = "hydrate")] {
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
struct ResourceData<V, E>(Option<Result<V, E>>);

impl<V, E> Serializable for ResourceData<V, E>
where
    V: Serializable,
    E: Serializable,
{
    fn ser(&self) -> Result<String, SerializationError> {
        match self.0 {
            Some(Ok(ref value)) => value.ser(),
            Some(Err(ref error)) => error.ser(),
            None => Ok("null".to_string()),
        }
    }

    fn de(bytes: &str) -> Result<Self, SerializationError> {
        match bytes {
            "" | "null" => Ok(ResourceData(None)),
            bytes => match <V>::de(bytes) {
                Ok(value) => Ok(ResourceData(Some(Ok(value)))),
                Err(_) => match <E>::de(bytes) {
                    Ok(error) => Ok(ResourceData(Some(Err(error)))),
                    Err(error) => Err(error),
                },
            },
        }
    }
}

/// A Type that cannot be instantiated. Useful to have a Result that cannot fail.
#[derive(Clone)]
pub enum Never {}

impl Serializable for Never {
    fn ser(&self) -> Result<String, SerializationError> {
        match *self {}
    }

    fn de(_: &str) -> Result<Self, SerializationError> {
        Err(SerializationError::Deserialize(std::rc::Rc::new(
            NeverSerde(),
        )))
    }
}

impl std::fmt::Debug for Never {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

struct NeverSerde();

impl std::error::Error for NeverSerde {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }
}

impl std::fmt::Display for NeverSerde {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NeverSerde")
    }
}
impl std::fmt::Debug for NeverSerde {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NeverSerde").finish()
    }
}
