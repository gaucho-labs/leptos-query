use crate::query_executor::create_executor;
use crate::query_result::QueryResult;
use crate::{
    use_query_client, Query, QueryData, QueryOptions, QueryState, RefetchFn, ResourceOption,
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
/// fn use_monkey_query(id: impl Fn() -> String + 'static) -> QueryResult<Monkey, impl RefetchFn> {
///     leptos_query::use_query(
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
    key: impl Fn() -> K + 'static,
    fetcher: impl Fn(K) -> Fu + 'static,
    options: QueryOptions<V>,
) -> QueryResult<V, impl RefetchFn>
where
    K: std::fmt::Debug + Hash + Eq + Clone + 'static,
    V: std::fmt::Debug + Clone + Serializable + 'static,
    Fu: Future<Output = V> + 'static,
{
    // Find relevant state.
    let query = use_query_client().get_query_signal(key);

    // Update options.
    // create_isomorphic_effect({
    //     let options = options.clone();
    //     move |_| {
    //         let (query, new) = query.get();
    //         if new {
    //             query.overwrite_options(options.clone())
    //         } else {
    //             query.update_options(options.clone())
    //         }
    //     }
    // });

    let query = Signal::derive(move || query.get().0);

    let query_observer = create_memo(move |_| {
        logging::log!("Registering observer {:?}", query.get_untracked().key);
        query.get().register_observer()
    });
    let query_state = Signal::derive(move || query_observer.get().state_signal().get());

    let resource_fetcher = move |query: Query<K, V>| {
        async move {
            match query.get_state() {
                // Immediately provide cached value.
                QueryState::Loaded(data)
                | QueryState::Invalid(data)
                | QueryState::Fetching(data) => ResourceData(Some(data.data)),

                // Suspend indefinitely and wait for interruption.
                QueryState::Created | QueryState::Loading => {
                    sleep(LONG_TIME).await;
                    ResourceData(None)
                }
            }
        }
    };

    let resource: Resource<Query<K, V>, ResourceData<V>> = {
        let default = options.default_value;
        match options.resource_option {
            ResourceOption::NonBlocking => create_resource_with_initial_value(
                move || query.get(),
                resource_fetcher,
                default.map(|default| ResourceData(Some(default))),
            ),
            ResourceOption::Blocking => {
                create_blocking_resource(move || query.get(), resource_fetcher)
            }
        }
    };

    create_isomorphic_effect(move |_| {
        if query_state.with(|s| matches!(s, QueryState::Loaded(_))) {
            resource.refetch();
        }
    });

    let executor = create_executor(query.into(), fetcher);

    // Ensure key changes are considered.
    create_isomorphic_effect({
        let executor = executor.clone();
        move |prev_query: Option<Query<K, V>>| {
            let query = query.get();
            if let Some(prev_query) = prev_query {
                if prev_query != query {
                    if query.with_state(|state| matches!(state, QueryState::Created)) {
                        executor()
                    }
                }
            }
            query
        }
    });

    let data = Signal::derive({
        let executor = executor.clone();
        move || {
            let read = resource.get().and_then(|r| r.0);
            let query = query.get_untracked();

            // First Read.
            // Putting this in an effect will cause it to always refetch needlessly on the client after SSR.
            if read.is_none() && query.with_state(|state| matches!(state, QueryState::Created)) {
                executor()
            }

            // SSR edge case.
            // Given hydrate can happen before resource resolves, signals on the client can be out of sync with resource.
            // Need to force insert the resource data into the query state.
            #[cfg(feature = "hydrate")]
            if let Some(ref data) = read {
                if query.with_state(|state| matches!(state, QueryState::Created)) {
                    let updated_at = crate::Instant::now();
                    let data = QueryData {
                        data: data.clone(),
                        updated_at,
                    };
                    query.set_state(QueryState::Loaded(data));
                }
            }
            read
        }
    });

    QueryResult {
        data,
        state: query_state,
        refetch: executor,
    }
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
            logging::debug_warn!("You are missing a Cargo feature for leptos_query. Please use one of 'ssr' or 'hydrate'")
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
