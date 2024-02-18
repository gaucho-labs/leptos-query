use crate::{query_observer::ListenerKey, *};
use leptos::*;
use std::{
    borrow::Borrow, cell::Cell, collections::HashMap, future::Future, rc::Rc, time::Duration,
};

use self::{
    cache_observer::CacheObserver, query::Query, query_cache::QueryCache,
    query_observer::QueryObserver,
};

/// Provides a Query Client to the current scope.
pub fn provide_query_client() {
    provide_query_client_with_options(DefaultQueryOptions::default());
}

/// Provides a Query Client to the current scope with custom options.
pub fn provide_query_client_with_options(options: DefaultQueryOptions) {
    let owner = Owner::current().expect("Owner to be present");

    provide_context(QueryClient::new(owner, options));
}

/// Retrieves a Query Client from the current scope.
pub fn use_query_client() -> QueryClient {
    use_context::<QueryClient>().expect("Query Client Missing.")
}

/// The Cache Client to store query data.
/// Exposes utility functions to manage queries.
///
/// Queries can be:
/// - [Prefetched](Self::prefetch_query)
///     - Query will start loading before you invoke [use_query](use_query::use_query).
/// - [Invalidated](Self::invalidate_query)
///     - Query will refetch on next usage. Active queries are immediately refetched in the background.
/// - [Introspected](Self::get_query_state)
///     - Let's you see what the current value is of a query is.
/// - [Manually updated](Self::set_query_data)
///     - Useful when you have updated a value and you want to manually set it in cache instead of waiting for query to refetch.
#[derive(Clone)]
pub struct QueryClient {
    pub(crate) cache: QueryCache,
    pub(crate) default_options: DefaultQueryOptions,
}

/// Default options for all queries under this client.
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
            stale_time: Some(Duration::from_secs(10)),
            gc_time: Some(Duration::from_secs(60 * 5)),
            refetch_interval: None,
            resource_option: ResourceOption::NonBlocking,
        }
    }
}

impl QueryClient {
    /// Creates a new Query Client.
    pub fn new(owner: Owner, default_options: DefaultQueryOptions) -> Self {
        Self {
            cache: QueryCache::new(owner),
            default_options,
        }
    }

    /// Fetch a query and store it in cache. Returns QueryResult.
    /// Result can be read outside of Transition.
    ///
    /// If you don't need the result opt for [`prefetch_query()`](Self::prefetch_query)
    pub async fn fetch_query<K, V, Fu>(
        &self,
        key: K,
        fetcher: impl Fn(K) -> Fu + 'static,
    ) -> QueryState<V>
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
        Fu: Future<Output = V> + 'static,
    {
        #[cfg(any(feature = "hydrate", feature = "csr"))]
        {
            let query = self.cache.get_or_create_query::<K, V>(key);

            query::execute_query(query.clone(), fetcher).await;

            query.get_state()
        }
        #[cfg(not(any(feature = "hydrate", feature = "csr")))]
        {
            let _ = key;
            let _ = fetcher;
            QueryState::Created
        }
    }

    /// Prefetch a query and store it in cache.
    /// If the entry already exists it will still be refetched.
    ///
    /// If you need the result opt for [`fetch_query()`](Self::fetch_query)
    pub async fn prefetch_query<K, V, Fu>(&self, key: K, fetcher: impl Fn(K) -> Fu + 'static)
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
        Fu: Future<Output = V> + 'static,
    {
        #[cfg(any(feature = "hydrate", feature = "csr"))]
        {
            let query = self.cache.get_or_create_query::<K, V>(key);

            query::execute_query(query.clone(), fetcher).await;
        }
        #[cfg(not(any(feature = "hydrate", feature = "csr")))]
        {
            let _ = key;
            let _ = fetcher;
        }
    }

    /// Retrieve the current state for an existing query.
    /// If the query does not exist, [`None`](Option::None) will be returned.
    pub fn get_query_state<K, V>(
        &self,
        key: impl Fn() -> K + 'static,
    ) -> Signal<Option<QueryState<V>>>
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        let cache = self.cache.clone();
        let size = self.size();

        // // Memoize state to avoid unnecessary hashmap lookups.
        let maybe_query = create_memo(move |_| {
            let key = key();
            // Subscribe to inserts/deletions.
            size.track();
            cache.get_query::<K, V>(&key)
        });

        let observer = Rc::new(QueryObserver::no_fetcher(
            QueryOptions::empty(),
            maybe_query.get_untracked(),
        ));

        let state_signal = RwSignal::new(maybe_query.get_untracked().map(|q| q.get_state()));

        let listener = Rc::new(Cell::new(None::<ListenerKey>));

        create_isomorphic_effect({
            let observer = observer.clone();
            let listener = listener.clone();
            move |_| {
                // Ensure listener is set.
                if let Some(curr_listener) = listener.take() {
                    listener.set(Some(curr_listener));
                } else {
                    let listener_id = observer.add_listener(move |state| {
                        state_signal.set(Some(state.clone()));
                    });
                    listener.set(Some(listener_id));
                }

                // Update
                let query = maybe_query.get();
                let current_state = query.as_ref().map(|q| q.get_state());
                observer.update_query(query);
                state_signal.set(current_state);
            }
        });

        state_signal.into()
    }

    /// Attempts to invalidate an entry in the Query Cache.
    /// Matching query is marked as invalid, and will be refetched in background once it's active.
    ///
    /// Returns true if the entry was successfully invalidated.
    ///
    /// Example:
    /// ```
    /// use leptos_query::*;
    ///
    /// use leptos_query::*;
    /// fn invalidate() {
    ///     let client = use_query_client();
    ///     let invalidated = client.invalidate_query::<u32, u32>(0);
    /// }
    /// ```
    pub fn invalidate_query<K, V>(&self, key: impl Borrow<K>) -> bool
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        self.cache
            .use_cache_option(|cache: &HashMap<K, Query<K, V>>| {
                cache
                    .get(Borrow::borrow(&key))
                    .map(|state| state.mark_invalid())
            })
            .unwrap_or(false)
    }

    /// Attempts to invalidate multiple entries in the Query Cache with a common <K, V> type.
    /// All matching queries are immediately marked as invalid and active queries are refetched in the background.
    ///
    /// Returns the keys that were successfully invalidated.
    ///
    /// Example:
    /// ```
    /// use leptos_query::*;
    /// fn invalidate() {
    ///     let client = use_query_client();
    ///     let keys: Vec<u32> = vec![0, 1];
    ///     let invalidated = client.invalidate_queries::<u32, u32, _>(keys);
    /// }
    ///
    /// ```
    pub fn invalidate_queries<K, V, Q>(&self, keys: impl IntoIterator<Item = Q>) -> Option<Vec<Q>>
    where
        K: crate::QueryKey + 'static,

        V: crate::QueryValue + 'static,
        Q: Borrow<K> + 'static,
    {
        self.cache
            .use_cache_option(|cache: &HashMap<K, Query<K, V>>| {
                let result = keys
                    .into_iter()
                    .filter(|key| {
                        cache
                            .get(Borrow::borrow(key))
                            .map(|query| query.mark_invalid())
                            .unwrap_or(false)
                    })
                    .collect::<Vec<_>>();
                Some(result)
            })
    }

    /// Invalidate all queries with a common <K, V> type.
    ///
    /// Example:
    /// ```
    /// use leptos_query::*;
    ///
    /// #[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
    /// struct MonkeyId(u32);
    ///
    /// #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
    /// struct Monkey {
    ///     name: String
    /// }
    ///
    /// fn invalidate() {
    ///     let client = use_query_client();
    ///     let keys: Vec<u32> = vec![0, 1];
    ///     let invalidated = client.invalidate_query_type::<String, Monkey>();
    /// }
    ///
    /// ```
    pub fn invalidate_query_type<K, V>(&self)
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        self.cache
            .use_cache_option(|cache: &HashMap<K, Query<K, V>>| {
                for q in cache.values() {
                    q.mark_invalid();
                }
                Some(())
            });
    }

    /// Invalidates all queries in the cache.
    ///
    /// Example:
    ///
    /// ```
    /// use leptos::*;
    /// use leptos_query::*;
    ///
    /// fn invalidate() {
    ///     let client = use_query_client();
    ///     let keys: Vec<u32> = vec![0, 1];
    ///     let invalidated = client.invalidate_all_queries();
    /// }
    ///
    /// ```
    ///
    pub fn invalidate_all_queries(&self) {
        self.cache.invalidate_all_queries()
    }

    /// Returns the current size of the cache.
    ///
    /// Example:
    /// ```
    /// use leptos::*;
    /// use leptos_query::*;
    ///
    /// fn invalidate() {
    ///    let client = use_query_client();
    ///    let cache_size = client.size();
    /// }
    ///
    /// ```
    pub fn size(&self) -> Signal<usize> {
        self.cache.size()
    }

    /// A synchronous function that can be used to immediately set a query's data.
    ///
    /// If the query does not exist, it will be created.
    ///
    /// If you need to fetch the data asynchronously, use [`fetch_query`](Self::fetch_query) or [`prefetch_query`](Self::prefetch_query).
    ///
    /// If the updater function returns [`None`](Option::None), the query data will not be updated.
    ///
    /// If the updater function receives [`None`](Option::None) as input, you can return [`None`](Option::None) to bail out of the update and thus not create a new cache entry.
    ///
    /// Example:
    /// ```
    /// use leptos_query::*;
    ///
    /// #[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
    /// struct MonkeyId(u32);
    ///
    /// #[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
    /// struct Monkey {
    ///     name: String
    /// }
    ///
    /// fn invalidate() {
    ///     let client = use_query_client();
    ///     // Overwrite existing data.
    ///     client.update_query_data::<MonkeyId, Monkey>(MonkeyId(0), |_| Some(Monkey { name: "George".to_string() }));
    ///
    ///     // Don't overwrite George.
    ///     client.update_query_data::<MonkeyId, Monkey>(MonkeyId(0), |probably_george| {
    ///        if let Some(Monkey { name }) = probably_george {
    ///            if name == "George" {
    ///               return None;
    ///            }
    ///        }
    ///        Some(Monkey { name: "Luffy".to_string() })
    ///     });
    ///
    /// }
    /// ```
    pub fn update_query_data<K, V>(
        &self,
        key: K,
        updater: impl FnOnce(Option<&V>) -> Option<V> + 'static,
    ) where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        self.cache
            .use_cache_entry(key.clone(), move |(owner, entry)| match entry {
                Some(query) => {
                    query.maybe_map_state(|state| match state {
                        QueryState::Created | QueryState::Loading => {
                            if let Some(result) = updater(None) {
                                Ok(QueryState::Loaded(QueryData::now(result)))
                            } else {
                                Err(state)
                            }
                        }
                        QueryState::Fetching(ref data) => {
                            if let Some(result) = updater(Some(&data.data)) {
                                Ok(QueryState::Fetching(QueryData::now(result)))
                            } else {
                                Err(state)
                            }
                        }
                        QueryState::Loaded(ref data) => {
                            if let Some(result) = updater(Some(&data.data)) {
                                Ok(QueryState::Loaded(QueryData::now(result)))
                            } else {
                                Err(state)
                            }
                        }
                        QueryState::Invalid(ref data) => {
                            if let Some(result) = updater(Some(&data.data)) {
                                Ok(QueryState::Loaded(QueryData::now(result)))
                            } else {
                                Err(state)
                            }
                        }
                    });
                    None
                }
                None => {
                    if let Some(result) = updater(None) {
                        let query = with_owner(owner, || Query::new(key));
                        query.set_state(QueryState::Loaded(QueryData::now(result)));
                        Some(query)
                    } else {
                        None
                    }
                }
            });
    }

    /// Update the query's data.
    pub fn set_query_data<K, V>(&self, key: K, data: V)
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        self.update_query_data(key, |_| Some(data));
    }

    /// Mutate the existing data if it exists.
    pub fn update_query_data_mut<K, V>(
        &self,
        key: impl Borrow<K>,
        updater: impl FnOnce(&mut V),
    ) -> bool
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        self.cache.use_cache::<K, V, bool>(move |cache| {
            let mut updated = false;
            if let Some(query) = cache.get(key.borrow()) {
                query.update_state(|state| {
                    if let Some(data) = state.data_mut() {
                        updater(data);
                        updated = true;
                    }
                });
            }
            updated
        })
    }

    /// Cancel any currently executing query.
    /// Returns whether the query was cancelled or not.
    pub fn cancel_query<K, V>(&self, key: K) -> bool
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        self.cache.use_cache::<K, V, bool>(move |cache| {
            if let Some(query) = cache.get(&key) {
                query.cancel()
            } else {
                false
            }
        })
    }

    /// Registers the cache observer.
    pub fn register_cache_observer(&self, observer: impl CacheObserver + 'static) {
        self.cache.register_query_observer(observer);
    }
}

#[cfg(all(test, not(any(feature = "csr", feature = "hydrate"))))]
mod tests {
    use super::*;

    #[test]
    fn set_query_data() {
        let _ = create_runtime();

        provide_query_client();
        let client = use_query_client();

        let state = || {
            use_query_client()
                .cache
                .get_query::<u32, String>(&0)
                .map(|q| q.get_state())
        };

        assert_eq!(None, state());
        assert_eq!(0, client.size().get_untracked());

        client.update_query_data::<u32, String>(0, |_| None);

        assert_eq!(None, state());
        assert_eq!(0, client.size().get_untracked());

        client.update_query_data::<u32, String>(0, |_| Some("0".to_string()));

        assert_eq!(1, client.size().get_untracked());

        assert_eq!(
            Some("0".to_string()),
            state().and_then(|q| q.data().cloned())
        );

        assert!(matches!(state(), Some(QueryState::Loaded { .. })));

        client.update_query_data::<u32, String>(0, |_| Some("1".to_string()));

        assert_eq!(
            Some("1".to_string()),
            state().and_then(|q| q.data().cloned())
        );
    }

    #[test]
    fn can_use_same_key_with_different_value_types() {
        let _ = create_runtime();

        provide_query_client();
        let client = use_query_client();

        client.update_query_data::<u32, String>(0, |_| Some("0".to_string()));

        client.update_query_data::<u32, u32>(0, |_| Some(1234));

        assert_eq!(2, client.size().get_untracked());
    }

    #[test]
    fn can_invalidate_while_subscribed() {
        let _ = create_runtime();

        provide_query_client();
        let client = use_query_client();

        let subscription = client.get_query_state::<u32, u32>(|| 0_u32);

        create_isomorphic_effect(move |_| {
            subscription.track();
        });

        client.update_query_data::<u32, u32>(0_u32, |_| Some(1234));

        assert!(client.invalidate_query::<u32, u32>(0));
        let state = subscription.get_untracked();

        assert!(
            matches!(state, Some(QueryState::Invalid { .. })),
            "Query should be invalid"
        );
    }

    #[test]
    fn can_invalidate_multiple() {
        let _ = create_runtime();

        provide_query_client();
        let client = use_query_client();

        client.update_query_data::<u32, u32>(0, |_| Some(1234));
        client.update_query_data::<u32, u32>(1, |_| Some(1234));
        let keys: Vec<u32> = vec![0, 1];
        let invalidated = client
            .invalidate_queries::<u32, u32, _>(keys.clone())
            .unwrap_or_default();

        assert_eq!(keys, invalidated)
    }

    #[test]
    fn can_invalidate_multiple_strings() {
        let _ = create_runtime();

        provide_query_client();
        let client = use_query_client();

        let zero = "0".to_string();
        let one = "1".to_string();

        client.update_query_data::<String, String>(zero.clone(), |_| Some("1234".into()));
        client.update_query_data::<String, String>(one.clone(), |_| Some("5678".into()));

        let keys = vec![zero, one];
        let invalidated = client
            .invalidate_queries::<String, String, _>(keys.clone())
            .unwrap_or_default();

        assert_eq!(keys, invalidated)
    }

    #[test]
    fn invalidate_all() {
        let _ = create_runtime();

        provide_query_client();
        let client = use_query_client();

        let zero = "0".to_string();
        let one = "1".to_string();

        client.update_query_data::<String, String>(zero.clone(), |_| Some("1234".into()));
        client.update_query_data::<String, String>(one.clone(), |_| Some("5678".into()));
        client.update_query_data::<u32, u32>(0, |_| Some(1234));
        client.update_query_data::<u32, u32>(1, |_| Some(5678));

        let state0_string = client.get_query_state::<String, String>(move || zero.clone());

        let state1_string = client.get_query_state::<String, String>(move || one.clone());

        let state0 = client.get_query_state::<u32, u32>(|| 0);
        let state1 = client.get_query_state::<u32, u32>(|| 1);

        client.invalidate_all_queries();

        assert!(matches!(
            state0.get_untracked(),
            Some(QueryState::Invalid { .. })
        ));
        assert!(matches!(
            state1.get_untracked(),
            Some(QueryState::Invalid { .. })
        ));
        assert!(matches!(
            state0_string.get_untracked(),
            Some(QueryState::Invalid { .. })
        ));
        assert!(matches!(
            state1_string.get_untracked(),
            Some(QueryState::Invalid { .. })
        ));
    }

    #[test]
    fn can_invalidate_subset() {
        let _ = create_runtime();

        provide_query_client();
        let client = use_query_client();

        client.update_query_data::<u32, u32>(0, |_| Some(1234));
        client.update_query_data::<u32, u32>(1, |_| Some(1234));

        let state0 = client.get_query_state::<u32, u32>(|| 0);
        let state1 = client.get_query_state::<u32, u32>(|| 1);

        client.invalidate_query_type::<u32, u32>();

        assert!(matches!(
            state0.get_untracked(),
            Some(QueryState::Invalid { .. })
        ));
        assert!(matches!(
            state1.get_untracked(),
            Some(QueryState::Invalid { .. })
        ));
    }
}
