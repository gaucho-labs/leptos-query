use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;
use std::{borrow::Borrow, future::Future};

use leptos::Signal;

use crate::{
    use_query, use_query_client, QueryKey, QueryOptions, QueryResult, QueryState, QueryValue,
    RefetchFn, ResourceOption,
};
/// Creates a new `QueryScope` for managing queries with specific key and value types.
///
/// # Type Parameters
///
/// * `K`: The type of the query key.
/// * `V`: The type of the query value.
/// * `Fu`: The future type returned by the fetcher function.
///
/// # Parameters
///
/// * `fetcher`: A function that, given a key of type `K`, returns a `Future` that resolves to a value of type `V`.
/// * `options`: Query options used to configure all queries within this scope.
///
/// # Returns
///
/// Returns a new instance of `QueryScope<K, V>`.
///
/// # Example
///
/// ```
/// use leptos_query::*;
///  
/// fn main() {
///     let query_scope = create_query::<UserId, UserData, _>(fetch_user_data, QueryOptions::default());
/// }
///
/// async fn fetch_user_data(id: UserId) -> UserData {
///    todo!()
/// }
///
/// #[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
/// struct UserId(i32);
///
/// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// struct UserData {
///    name: String,
/// }
///
/// ```
pub fn create_query<K, V, Fu>(
    fetcher: impl Fn(K) -> Fu + 'static,
    options: QueryOptions<V>,
) -> QueryScope<K, V>
where
    K: QueryKey + 'static,
    V: QueryValue + 'static,
    Fu: Future<Output = V> + 'static,
{
    let fetcher = Rc::new(move |s| Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = V>>>);
    QueryScope { fetcher, options }
}

/// A scope for managing queries with specific key and value types within a type-safe environment.
///
/// This struct allows operations such as fetching, prefetching, and invalidating queries to be performed.
///
/// # Type Parameters
///
/// * `K`: The type of the query key.
/// * `V`: The type of the query value.
pub struct QueryScope<K, V> {
    fetcher: Rc<dyn Fn(K) -> Pin<Box<dyn Future<Output = V>>>>,
    options: QueryOptions<V>,
}

impl<K, V> QueryScope<K, V>
where
    K: QueryKey + 'static,
    V: QueryValue + 'static,
{
    /// Executes a query using the provided key function and the fetcher function specified at creation.
    /// Data must be read inside of a Suspense/Transition component
    ///
    /// # Parameters
    ///
    /// * `key`: A function that returns the query key of type `K`.
    ///
    /// # Returns
    ///
    /// Returns a `QueryResult<V, impl RefetchFn>`, which includes the query data and a function to refetch the query.
    ///
    /// # Example
    ///
    /// ```
    /// let user_data = query_scope.use_query(|| UserId(1));
    /// ```
    pub fn use_query(&self, key: impl Fn() -> K + 'static) -> QueryResult<V, impl RefetchFn> {
        self.use_query_with_options(key, OverrideOptions::default())
    }

    /// Executes a query with additional options that override the default options provided at the scope's creation.
    ///
    /// # Parameters
    ///
    /// * `key`: A function that returns the query key of type `K`.
    /// * `options`: Additional options to override the default query options.
    ///
    /// # Returns
    ///
    /// Returns a `QueryResult<V, impl RefetchFn>` similar to `use_query`, but with the provided override options applied.
    pub fn use_query_with_options(
        &self,
        key: impl Fn() -> K + 'static,
        options: OverrideOptions<V>,
    ) -> QueryResult<V, impl RefetchFn> {
        use_query(key, self.make_fetcher(), self.merge_options(options))
    }

    /// Prefetches a query and stores it in the cache. Useful for preloading data before it is needed.
    ///
    /// # Parameters
    ///
    /// * `key`: A function that returns the query key of type `K`.
    /// * `isomorphic`: A boolean indicating whether the prefetch should be performed isomorphically (both on server and client in SSR environments).
    pub fn prefetch_query(&self, key: impl Fn() -> K + 'static, isomorphic: bool) {
        use_query_client().prefetch_query(key, self.make_fetcher(), isomorphic)
    }

    /// Retrieves the current state of a query identified by the given key function.
    ///
    /// # Parameters
    ///
    /// * `key`: A function that returns the query key of type `K`.
    ///
    /// # Returns
    ///
    /// A `Signal` containing an `Option` with the current `QueryState` of the query. If the query does not exist, the `Signal` will contain `None`.
    pub fn get_query_state(&self, key: impl Fn() -> K + 'static) -> Signal<Option<QueryState<V>>> {
        use_query_client().get_query_state(key)
    }

    /// Invalidates a query in the cache, identified by a specific key, marking it as needing a refetch.
    ///
    /// # Parameters
    ///
    /// * `key`: A key that identifies the query to be invalidated.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the query was successfully invalidated.
    pub fn invalidate_query(&self, key: impl Borrow<K>) -> bool {
        use_query_client().invalidate_query::<K, V>(key)
    }

    /// Invalidates multiple queries in the cache, identified by a collection of keys.
    ///
    /// # Parameters
    ///
    /// * `keys`: An iterator over keys that identify the queries to be invalidated.
    ///
    /// # Returns
    ///
    /// An `Option` containing a `Vec` of keys that were successfully invalidated. If no queries were invalidated, `None` is returned.
    pub fn invalidate_queries<Q>(&self, keys: impl IntoIterator<Item = Q>) -> Option<Vec<Q>>
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
        Q: Borrow<K> + 'static,
    {
        use_query_client().invalidate_queries::<K, V, Q>(keys)
    }

    /// Invalidates all queries in the cache of a specific type, triggering a refetch for active queries.
    pub fn invalidate_all_queries(&self) {
        use_query_client().invalidate_query_type::<K, V>();
    }

    /// Updates the data of an existing query in the cache, identified by a specific key.
    ///
    /// # Parameters
    ///
    /// * `key`: The key that identifies the query to update.
    /// * `updater`: A closure that takes the current query data as an argument and returns updated data.
    ///
    /// If the updater returns `None`, the query data is not updated.
    pub fn update_query_data(
        &self,
        key: K,
        updater: impl FnOnce(Option<&V>) -> Option<V> + 'static,
    ) {
        use_query_client().update_query_data(key, updater);
    }

    /// Sets the data of an existing query in the cache, identified by a specific key.
    ///
    /// # Parameters
    ///
    /// * `key`: The key that identifies the query to update.
    /// * `data`: The new value.
    ///
    pub fn set_query_data(&self, key: K, data: V) {
        use_query_client().set_query_data(key, data);
    }

    /// Mutates the data of an existing query in the cache, identified by a specific key.
    ///
    /// # Parameters
    ///
    /// * `key`: A key that identifies the query to mutate.
    /// * `updater`: A closure that takes a mutable reference to the current query data.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the query data was successfully mutated.
    pub fn update_query_data_mut(&self, key: impl Borrow<K>, updater: impl FnOnce(&mut V)) -> bool {
        use_query_client().update_query_data_mut(key, updater)
    }

    /// Cancels an ongoing fetch operation for a query, identified by a specific key.
    ///
    /// # Parameters
    ///
    /// * `key`: The key that identifies the query to cancel.
    ///
    /// # Returns
    ///
    /// A boolean indicating whether the fetch operation was successfully cancelled.
    pub fn cancel_query(&self, key: K) -> bool {
        use_query_client().cancel_query::<K, V>(key)
    }

    fn make_fetcher(&self) -> impl Fn(K) -> Pin<Box<dyn Future<Output = V>>> {
        let fetcher = self.fetcher.clone();
        move |key| fetcher(key)
    }

    fn merge_options(&self, options: OverrideOptions<V>) -> QueryOptions<V> {
        QueryOptions {
            stale_time: options.stale_time.or(self.options.stale_time),
            gc_time: options.gc_time.or(self.options.gc_time),
            refetch_interval: options.refetch_interval.or(self.options.refetch_interval),
            resource_option: options
                .resource_option
                .unwrap_or(self.options.resource_option),
            default_value: options
                .default_value
                .or_else(|| self.options.default_value.clone()),
        }
    }
}

/// Override options for a query. These options can be used to customize the behavior of individual queries within a `QueryScope`.
///
/// # Type Parameters
///
/// * `V`: The type of the query value.
///
/// # Example
///
/// ```
/// use std::time::Duration;    
/// use leptos_query::*;
///
/// let override_options: OverrideOptions<u32> = OverrideOptions {
///     stale_time: Some(Duration::from_secs(30)),
///     ..OverrideOptions::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct OverrideOptions<V> {
    /// Optional duration before a query is considered stale and needs refetching.
    pub stale_time: Option<Duration>,
    /// Optional duration before an inactive query is removed from the cache.
    pub gc_time: Option<Duration>,
    ///  Optional interval for periodically refetching the query.
    pub refetch_interval: Option<Duration>,
    /// Optional resource option to specify how the query should be executed.
    pub resource_option: Option<ResourceOption>,
    /// Optional default value for the query.
    pub default_value: Option<V>,
}

impl<V> Default for OverrideOptions<V> {
    fn default() -> Self {
        Self {
            stale_time: None,
            gc_time: None,
            refetch_interval: None,
            resource_option: None,
            default_value: None,
        }
    }
}
