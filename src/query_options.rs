use std::time::Duration;

/**
 * Options for a Query Client.
 */
#[derive(Clone)]
pub struct QueryCacheOptions<V> {
    pub default_value: Option<V>,
    pub stale_time: Option<Duration>,
    pub refetch_interval: Option<Duration>,
    pub resource_option: ResourceOption,
}

#[derive(Clone, Copy)]
pub enum ResourceOption {
    /// Equivalent to [`create_resource()`]
    NonBlocking,
    /// Equivalent to [`create_blocking_resource()`]
    Blocking,
    /// Equivalent to [`create_local_resource()`]
    Local,
}

impl<V> QueryCacheOptions<V> {
    pub fn stale_time(stale_time: Duration) -> Self {
        Self {
            default_value: None,
            stale_time: Some(stale_time),
            refetch_interval: None,
            resource_option: ResourceOption::NonBlocking,
        }
    }

    pub fn refetch_interval(refetch_interval: Duration) -> Self {
        Self {
            default_value: None,
            stale_time: None,
            refetch_interval: Some(refetch_interval),
            resource_option: ResourceOption::NonBlocking,
        }
    }
}

impl<V> Default for QueryCacheOptions<V> {
    fn default() -> Self {
        Self {
            default_value: None,
            stale_time: None,
            refetch_interval: None,
            resource_option: ResourceOption::NonBlocking,
        }
    }
}

#[derive(Clone, Copy)]
pub struct QueryOptions {
    pub stale_time: Option<Duration>,
    pub refetch_interval: Option<Duration>,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            stale_time: None,
            refetch_interval: None,
        }
    }
}

impl QueryOptions {
    pub fn stale_time(stale_time: Duration) -> Self {
        Self {
            stale_time: Some(stale_time),
            refetch_interval: None,
        }
    }

    pub fn refetch_interval(refetch_interval: Duration) -> Self {
        Self {
            stale_time: None,
            refetch_interval: Some(refetch_interval),
        }
    }
}
