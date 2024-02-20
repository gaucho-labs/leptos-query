use std::time::Duration;

/// Default options for all queries under this client.
/// Only differs from [`QueryOptions`] in that it doesn't have a default value.
#[derive(Debug, Clone, Copy)]
pub struct DefaultQueryOptions {
    /// Time before a query is considered stale.
    pub stale_time: Option<Duration>,
    /// Time before an inactive query is removed from cache.
    pub gc_time: Option<Duration>,
    /// Time before a query is refetched.
    pub refetch_interval: Option<Duration>,
    /// Determines which type of resource to use.
    pub resource_option: ResourceOption,
}

impl Default for DefaultQueryOptions {
    fn default() -> Self {
        Self {
            stale_time: Some(DEFAULT_STALE_TIME),
            gc_time: Some(DEFAULT_GC_TIME),
            refetch_interval: None,
            resource_option: ResourceOption::default(),
        }
    }
}

const DEFAULT_STALE_TIME: Duration = Duration::from_secs(10);
const DEFAULT_GC_TIME: Duration = Duration::from_secs(60 * 5);

/**
 * Options for a query [`use_query()`](crate::use_query())
 */
#[derive(Debug, Clone)]
pub struct QueryOptions<V> {
    /// Placeholder value to use while the query is loading for the first time.
    pub default_value: Option<V>,
    /// The duration that should pass before a query is considered stale.
    /// If the query is stale, it will be refetched.
    /// If no stale_time, the query will never be considered stale.
    /// Stale time is checked when [`use_query()`](crate::use_query()) instance is mounted.
    /// Stale_time can never be greater than cache_time.
    /// Default is 10 seconds.
    /// NOTE: If different stale_time are used for the same key, the MINIMUM time will be used.
    pub stale_time: Option<Duration>,
    /// The amount of time a query will be cached, once it's considered stale.
    /// If no cache time, the query will never be revoked from cache.
    /// cache_time can never be less than stale_time.
    /// Default is 5 minutes.
    /// NOTE: If different cache times are used for the same key, the MAXIMUM time will be used.
    pub gc_time: Option<Duration>,
    /// If no refetch interval, the query will never refetch.
    pub refetch_interval: Option<Duration>,
    /// Determines which type of resource to use.
    pub resource_option: Option<ResourceOption>,
}

impl<V> QueryOptions<V> {
    /// Set the default value.
    pub fn set_default_value(self, default_value: Option<V>) -> Self {
        QueryOptions {
            default_value,
            ..self
        }
    }

    /// Set the stale_time.
    pub fn set_stale_time(self, stale_time: Option<Duration>) -> Self {
        QueryOptions { stale_time, ..self }
    }

    /// Set the gc time.
    pub fn set_gc_time(self, gc_time: Option<Duration>) -> Self {
        QueryOptions { gc_time, ..self }
    }

    /// Set the refetch interval.
    pub fn set_refetch_interval(self, refetch_interval: Option<Duration>) -> Self {
        QueryOptions {
            refetch_interval,
            ..self
        }
    }

    /// Set the resource option.
    pub fn set_resource_option(self, resource_option: Option<ResourceOption>) -> Self {
        QueryOptions {
            resource_option,
            ..self
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

    /// Ensures that gc_time is >= than stale_time.
    pub fn validate(self) -> Self {
        let stale_time = self.stale_time;
        let gc_time = self.gc_time;

        let stale_time = ensure_valid_stale_time(&stale_time, &gc_time);

        QueryOptions {
            default_value: self.default_value,
            stale_time,
            gc_time: self.gc_time,
            refetch_interval: self.refetch_interval,
            resource_option: self.resource_option,
        }
    }
}

impl<V> Default for QueryOptions<V> {
    fn default() -> Self {
        // Use cache wide defaults if they exist.
        let default_options = leptos::use_context::<crate::QueryClient>()
            .map(|c| c.default_options)
            .unwrap_or_default();
        Self {
            default_value: None,
            stale_time: default_options.stale_time,
            gc_time: default_options.gc_time,
            refetch_interval: default_options.refetch_interval,
            resource_option: Some(default_options.resource_option),
        }
        .validate()
    }
}

/// Determines which type of resource to use.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ResourceOption {
    /// Query will use [`create_resource()`](leptos::create_resource)
    #[default]
    NonBlocking,
    /// Query will use [`create_blocking_resource()`](leptos::create_blocking_resource)
    Blocking,
    /// Query will use [`create_local_resource()`](leptos::create_local_resource)
    Local,
}

fn ensure_valid_stale_time(
    stale_time: &Option<Duration>,
    gc_time: &Option<Duration>,
) -> Option<Duration> {
    match (stale_time, gc_time) {
        (Some(ref stale_time), Some(ref gc_time)) => {
            if stale_time > gc_time {
                leptos::logging::debug_warn!(
                    "stale_time is greater than gc_time. Using gc time instead. stale_time: {}, gc_time: {}",
                    stale_time.as_millis(),
                    gc_time.as_millis()
                );
                Some(*gc_time)
            } else {
                Some(*stale_time)
            }
        }
        (None, Some(ref gc_duration)) => {
            leptos::logging::debug_warn!(
                "stale_time (infinity) is greater than gc_time. Using gc_time instead. gc_time: {}",
                gc_duration.as_millis()
            );
            let _ = gc_duration;
            *gc_time
        }
        (stale_time, _) => *stale_time,
    }
}

#[cfg(test)]
mod tests {
    use crate::provide_query_client_with_options;

    use super::*;

    #[test]
    fn validate_stale_time_less_than_gc_time() {
        let options = QueryOptions::<i32> {
            default_value: None,
            stale_time: Some(Duration::from_secs(5)),
            gc_time: Some(Duration::from_secs(10)),
            refetch_interval: None,
            resource_option: None,
        }
        .validate();

        assert_eq!(
            options.stale_time,
            Some(Duration::from_secs(5)),
            "Stale_time should remain unchanged"
        );
        assert_eq!(
            options.gc_time,
            Some(Duration::from_secs(10)),
            "GC time should remain unchanged"
        );
    }

    #[test]
    fn validate_stale_time_greater_than_gc_time() {
        let options = QueryOptions::<i32> {
            default_value: None,
            stale_time: Some(Duration::from_secs(15)),
            gc_time: Some(Duration::from_secs(10)),
            refetch_interval: None,
            resource_option: None,
        }
        .validate();

        assert_eq!(
            options.stale_time,
            Some(Duration::from_secs(10)),
            "Stale_time should be adjusted to GC time"
        );
        assert_eq!(
            options.gc_time,
            Some(Duration::from_secs(10)),
            "GC time should remain unchanged"
        );
    }

    #[test]
    fn validate_stale_time_without_gc_time() {
        let options = QueryOptions::<i32> {
            default_value: None,
            stale_time: Some(Duration::from_secs(5)),
            gc_time: None,
            refetch_interval: None,
            resource_option: None,
        }
        .validate();

        assert_eq!(
            options.stale_time,
            Some(Duration::from_secs(5)),
            "Stale_time should remain unchanged"
        );
        assert_eq!(options.gc_time, None, "GC time should remain None");
    }

    #[test]
    fn validate_gc_time_without_stale_time() {
        let options = QueryOptions::<i32> {
            default_value: None,
            stale_time: None,
            gc_time: Some(Duration::from_secs(10)),
            refetch_interval: None,
            resource_option: None,
        }
        .validate();
        assert_eq!(
            options.stale_time,
            Some(Duration::from_secs(10)),
            "Stale_time should become gc_time"
        );
        assert_eq!(
            options.gc_time,
            Some(Duration::from_secs(10)),
            "GC time should remain unchanged"
        );
    }

    #[test]
    fn validate_none_stale_and_gc_time() {
        let options = QueryOptions::<i32> {
            default_value: None,
            stale_time: None,
            gc_time: None,
            refetch_interval: None,
            resource_option: None,
        }
        .validate();

        assert_eq!(options.stale_time, None, "Stale_time should remain None");
        assert_eq!(options.gc_time, None, "GC time should remain None");
    }

    #[test]
    fn test_default() {
        let _ = leptos::create_runtime();

        provide_query_client_with_options(DefaultQueryOptions {
            stale_time: Some(Duration::from_secs(1)),
            gc_time: Some(Duration::from_secs(2)),
            refetch_interval: Some(Duration::from_secs(3)),
            resource_option: ResourceOption::NonBlocking,
        });

        // Action: Create a QueryOptions instance using Default::default()
        let default_options: QueryOptions<()> = Default::default();

        // Verification: Assert that QueryOptions has the expected default values
        assert_eq!(
            default_options.stale_time,
            Some(Duration::from_secs(1)),
            "Default stale_time should match the provided QueryClient's default"
        );
        assert_eq!(
            default_options.gc_time,
            Some(Duration::from_secs(2)),
            "Default gc_time should match the provided QueryClient's default"
        );
        assert_eq!(
            default_options.refetch_interval,
            Some(Duration::from_secs(3)),
            "Default refetch_interval should match the provided QueryClient's default"
        );
        assert_eq!(
            default_options.resource_option,
            Some(ResourceOption::NonBlocking),
            "Default resource_option should match the provided QueryClient's default"
        );

        // Additional check: Ensure the default options are validated
        // This ensures gc_time is not less than stale_time after validation
        assert!(
            default_options.gc_time.unwrap() >= default_options.stale_time.unwrap(),
            "After validation, gc_time should not be less than stale_time"
        );
    }
}
