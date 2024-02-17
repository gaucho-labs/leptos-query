use std::time::Duration;

/**
 * Options for a query [`crate::use_query::use_query`]
 */
#[derive(Debug, Clone)]
pub struct QueryOptions<V> {
    /// Placeholder value to use while the query is loading for the first time.
    pub default_value: Option<V>,
    /// The duration that should pass before a query is considered stale.
    /// If the query is stale, it will be refetched.
    /// If no stale time, the query will never be considered stale.
    /// Stale time is checked when [`QueryState::read`](#impl-<K,V>-for-QueryState<K,V>) is used.
    /// Stale time can never be greater than cache_time.
    /// Default is 0 milliseconds.
    /// NOTE: If different stale times are used for the same key, the minimum time for the currently ACTIVE query will be used.
    pub stale_time: Option<Duration>,
    /// The amount of time a query will be cached, once it's considered stale.
    /// If no cache time, the query will never be revoked from cache.
    /// cache_time can never be less than stale_time.
    /// Default is 5 minutes.
    /// NOTE: If different cache times are used for the same key, the minimum time will be used.
    pub gc_time: Option<Duration>,
    /// If no refetch interval, the query will never refetch.
    pub refetch_interval: Option<Duration>,
    /// Determines which type of resource to use.
    pub resource_option: Option<ResourceOption>,
}

impl<V> QueryOptions<V> {
    /// Only fetches the query once.
    pub fn once() -> Self {
        Self {
            default_value: None,
            stale_time: None,
            gc_time: None,
            refetch_interval: None,
            resource_option: Some(ResourceOption::NonBlocking),
        }
    }

    /// Empty options.
    pub fn empty() -> Self {
        Self {
            default_value: None,
            stale_time: None,
            gc_time: None,
            refetch_interval: None,
            resource_option: None,
        }
    }

    /// Transform the default value.
    pub fn map_value<R>(self, func: impl FnOnce(V) -> R) -> QueryOptions<R> {
        QueryOptions {
            default_value: self.default_value.map(func),
            stale_time: self.stale_time,
            gc_time: self.gc_time,
            refetch_interval: self.refetch_interval,
            resource_option: self.resource_option,
        }
    }
}

const DEFAULT_STALE_TIME: Duration = Duration::from_secs(10);
const DEFAULT_GC_TIME: Duration = Duration::from_secs(60 * 5);

impl<V> Default for QueryOptions<V> {
    fn default() -> Self {
        // Use cache wide defaults if they exist.
        if let Some(client) = leptos::use_context::<crate::QueryClient>() {
            let default_options = client.default_options;
            Self {
                default_value: None,
                stale_time: default_options.stale_time,
                gc_time: default_options.gc_time,
                refetch_interval: default_options.refetch_interval,
                resource_option: Some(default_options.resource_option),
            }
        } else {
            Self {
                default_value: None,
                stale_time: Some(DEFAULT_STALE_TIME),
                gc_time: Some(DEFAULT_GC_TIME),
                refetch_interval: None,
                resource_option: Some(ResourceOption::NonBlocking),
            }
        }
    }
}

/// Determines which type of resource to use.
#[derive(Debug, Clone, Copy, Default)]
pub enum ResourceOption {
    /// Query will use [`create_resource()`](leptos::create_resource)
    #[default]
    NonBlocking,
    /// Query will use [`create_blocking_resource()`](leptos::create_blocking_resource)
    Blocking,
}
