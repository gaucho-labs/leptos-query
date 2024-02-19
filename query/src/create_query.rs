use std::pin::Pin;
use std::rc::Rc;
use std::{borrow::Borrow, future::Future};

use leptos::Signal;

use crate::{
    use_query, use_query_client, QueryKey, QueryOptions, QueryResult, QueryState, QueryValue,
    RefetchFn,
};

/// Creates a new `QueryScope` for managing queries with specific key and value types.
///
/// Useful for having typed invalidation, setting, and updating of queries.
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
/// fn create_query_test() {
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
#[derive(Clone)]
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
    /// use leptos_query::*;
    ///
    /// fn test() {
    ///     provide_query_client();
    ///     let query_scope = create_query(fetch_user_data, QueryOptions::default());
    ///     let query = query_scope.use_query(|| UserId(1));
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
    /// ```
    pub fn use_query(&self, key: impl Fn() -> K + 'static) -> QueryResult<V, impl RefetchFn> {
        use_query(key, self.make_fetcher(), self.options.clone())
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
        options: QueryOptions<V>,
    ) -> QueryResult<V, impl RefetchFn> {
        use_query(
            key,
            self.make_fetcher(),
            options.merge(self.options.clone()),
        )
    }

    /// Prefetches a query and stores it in the cache. Useful for preloading data before it is needed.
    /// If you don't need the result opt for [`fetch_query()`](Self::fetch_query)
    pub async fn prefetch_query(&self, key: K) {
        use_query_client()
            .prefetch_query(key, self.make_fetcher())
            .await
    }

    /// Fetch a query and store it in cache.
    /// Result can be read outside of Transition.
    ///
    /// If you don't need the result opt for [`prefetch_query()`](Self::prefetch_query)
    pub async fn fetch_query(&self, key: K) -> QueryState<V> {
        use_query_client()
            .fetch_query(key, self.make_fetcher())
            .await
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

    /// Retrieve the current state for an existing query.
    /// If the query does not exist, [`None`](Option::None) will be returned.
    /// Useful for when you want to introspect the state of a query without subscribing to it.
    pub fn peek_query_state(&self, key: &K) -> Option<QueryState<V>> {
        use_query_client().peek_query_state(key)
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
}
