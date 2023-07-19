use std::time::Duration;

/**
 * Options for a [`crate::query_client::QueryClient`]
 */
#[derive(Clone)]
pub struct QueryOptions<V> {
    /// Placeholder value to use while the query is loading for the first time.
    pub default_value: Option<V>,
    /// The duration that should pass before a query is considered stale.
    /// If the query is stale, it will be refetched.
    /// If no stale time, the query will never be considered stale.
    /// Stale time is checked when [`QueryState::read`](#impl-<K,V>-for-QueryState<K,V>) is used.
    pub stale_time: Option<Duration>,
    /// The amount of time a query will be cached, once it's considered stale.
    /// If no cache time, the query will never be revoked from cache.
    /// cache_time can never be less than stale_time.
    /// Default is 5 minutes.
    pub cache_time: Option<Duration>,
    /// If no refetch interval, the query will never refetch.
    pub refetch_interval: Option<Duration>,
    /// Determines which type of resource to use.
    pub resource_option: ResourceOption,
}

/// Determines which type of resource to use.
#[derive(Clone, Copy)]
pub enum ResourceOption {
    /// Query will use [`create_resource()`](leptos::create_resource)
    NonBlocking,
    /// Query will use [`create_blocking_resource()`](leptos::create_blocking_resource)
    Blocking,
}

impl<V> QueryOptions<V> {
    /// QueryOption with custom stale_time.
    pub fn stale_time(stale_time: Duration) -> Self {
        Self {
            default_value: None,
            stale_time: Some(stale_time),
            cache_time: Some(DEFAULT_STALE_TIME),
            refetch_interval: None,
            resource_option: ResourceOption::NonBlocking,
        }
    }

    /// QueryOption with custom refetch_interval.
    pub fn refetch_interval(refetch_interval: Duration) -> Self {
        Self {
            default_value: None,
            stale_time: None,
            cache_time: Some(DEFAULT_STALE_TIME),
            refetch_interval: Some(refetch_interval),
            resource_option: ResourceOption::NonBlocking,
        }
    }
}

const DEFAULT_STALE_TIME: Duration = Duration::from_secs(60 * 5);

impl<V> Default for QueryOptions<V> {
    fn default() -> Self {
        Self {
            default_value: None,
            stale_time: None,
            cache_time: Some(DEFAULT_STALE_TIME),
            refetch_interval: None,
            resource_option: ResourceOption::NonBlocking,
        }
    }
}
