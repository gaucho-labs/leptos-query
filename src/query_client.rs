use crate::*;
use leptos::*;
use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    cell::RefCell,
    collections::hash_map::Entry,
    collections::HashMap,
    future::Future,
    hash::Hash,
    rc::Rc,
};

/// Provides a Query Client to the current scope.
pub fn provide_query_client() {
    provide_context(QueryClient::new(
        Owner::current().expect("Owner to be present"),
    ));
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
#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct QueryClient {
    pub(crate) owner: Owner,
    // Signal to indicate a cache entry has been added or removed.
    pub(crate) notify: RwSignal<()>,
    pub(crate) cache: Rc<RefCell<HashMap<(TypeId, TypeId), Box<dyn CacheEntryTrait>>>>,
}

pub(crate) struct CacheEntry<K: 'static, V: 'static>(HashMap<K, Query<K, V>>);

// Trait to enable cache introspection among distinct cache entry maps.
pub(crate) trait CacheEntryTrait: CacheSize + CacheInvalidate {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<K, V> CacheEntryTrait for CacheEntry<K, V>
where
    K: Clone,
    V: Clone,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(crate) trait CacheSize {
    fn size(&self) -> usize;
}

impl<K, V> CacheSize for CacheEntry<K, V> {
    fn size(&self) -> usize {
        self.0.len()
    }
}

pub(crate) trait CacheInvalidate {
    fn invalidate(&self);
}

impl<K, V> CacheInvalidate for CacheEntry<K, V>
where
    K: Clone,
    V: Clone,
{
    fn invalidate(&self) {
        for (_, query) in self.0.iter() {
            query.mark_invalid();
        }
    }
}

impl QueryClient {
    /// Creates a new Query Client.
    pub fn new(owner: Owner) -> Self {
        Self {
            notify: create_rw_signal(()),
            owner,
            cache: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Prefetch a query and store it in cache. Returns QueryResult.
    /// If the entry already exists it will still be refetched.
    ///
    /// If you don't need the result opt for [`prefetch_query()`](Self::prefetch_query)
    // pub fn fetch_query<K, V, Fu>(
    //     &self,
    //     key: impl Fn() -> K + 'static,
    //     fetcher: impl Fn(K) -> Fu + 'static,
    //     isomorphic: bool,
    // ) -> QueryResult<V, impl RefetchFn>
    // where
    //     K: Hash + Eq + Clone + 'static,
    //     V: Clone + 'static,
    //     Fu: Future<Output = V> + 'static,
    // {
    //     todo!()
    //     let state = self.get_query_signal(key);

    //     let state = Signal::derive(move || state.get().0);

    //     let executor = create_executor(state, fetcher);

    //     let sync = {
    //         let executor = executor.clone();
    //         move |_| {
    //             let _ = state.get();
    //             executor()
    //         }
    //     };
    //     if isomorphic {
    //         create_isomorphic_effect(sync);
    //     } else {
    //         create_effect(sync);
    //     }

    //     synchronize_state(state, executor.clone());

    //     create_query_result(
    //         state,
    //         Signal::derive(move || state.get().state.get().data().cloned()),
    //         executor,
    //     )
    // }

    /// Prefetch a query and store it in cache.
    /// If the entry already exists it will still be refetched.
    ///
    /// If you need the result opt for [`fetch_query()`](Self::fetch_query)
    pub fn prefetch_query<K, V, Fu>(
        &self,
        key: impl Fn() -> K + 'static,
        query: impl Fn(K) -> Fu + 'static,
        isomorphic: bool,
    ) where
        K: Hash + Eq + Clone + 'static,
        V: Clone + 'static,
        Fu: Future<Output = V> + 'static,
    {
        // let state = self.get_query_signal(key);

        // let state = Signal::derive(move || state.get().0);

        // let executor = create_executor(state, query);

        // let sync = {
        //     move |_| {
        //         let _ = state.get();
        //         executor()
        //     }
        // };
        // if isomorphic {
        //     create_isomorphic_effect(sync);
        // } else {
        //     create_effect(sync);
        // }
        ()
    }

    /// Retrieve the current state for an existing query.
    /// If the query does not exist, [`None`](Option::None) will be returned.
    pub fn get_query_state<K, V>(
        &self,
        key: impl Fn() -> K + 'static,
    ) -> Signal<Option<QueryState<V>>>
    where
        K: Hash + Eq + Clone + 'static,
        V: Clone,
    {
        // let client = self.clone();

        // // Memoize state to avoid unnecessary hashmap lookups.
        // let maybe_query = create_memo(move |_| {
        //     let key = key();
        //     client.notify.get();
        //     client.use_cache_option(|cache: &HashMap<K, Query<K, V>>| cache.get(&key).cloned())
        // });

        // synchronize_observer(maybe_query.into());

        // Signal::derive(move || maybe_query.get().map(|s| s.state.get()))
        todo!();
    }

    /// Attempts to invalidate an entry in the Query Cache.
    /// Matching query is marked as invalid, and will be refetched in background once it's active.
    ///
    /// Returns true if the entry was successfully invalidated.
    ///
    /// Example:
    /// ```
    /// let client = use_query_client();
    /// let invalidated = client.invalidate_query::<u32, u32>(0);
    /// ```
    pub fn invalidate_query<K, V>(&self, key: impl Borrow<K>) -> bool
    where
        K: Hash + Eq + Clone + 'static,
        V: Clone + 'static,
    {
        self.use_cache_option(|cache: &HashMap<K, Query<K, V>>| {
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
    /// let client = use_query_client();
    /// let keys: Vec<u32> = vec![0, 1];
    /// let invalidated = client.invalidate_queries::<u32, u32, _>(keys)
    ///
    /// ```
    pub fn invalidate_queries<K, V, Q>(&self, keys: impl IntoIterator<Item = Q>) -> Option<Vec<Q>>
    where
        K: Hash + Eq + Clone + 'static,
        V: Clone + 'static,
        Q: Borrow<K>,
    {
        // Find all states, drop borrow, then mark invalid.
        let cache_borrowed = RefCell::try_borrow(&self.cache).expect("invalidate_queries borrow");
        let type_key = (TypeId::of::<K>(), TypeId::of::<V>());
        let cache = cache_borrowed.get(&type_key)?;
        let cache = cache
            .as_any()
            .downcast_ref::<CacheEntry<K, V>>()
            .expect(EXPECT_CACHE_ERROR);
        let result = keys
            .into_iter()
            .filter(|key| {
                cache
                    .0
                    .get(Borrow::borrow(key))
                    .map(|query| query.mark_invalid())
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        Some(result)
    }

    /// Invalidate all queries with a common <K, V> type.
    ///
    /// Example:
    /// ```
    /// use leptos::*;
    /// use leptos_query::*;
    ///
    /// let client = use_query_client();
    /// client.invalidate_query_type::<String, Monkey>();
    ///
    /// ```
    pub fn invalidate_query_type<K, V>(&self) -> &Self
    where
        K: Clone + 'static,
        V: Clone + 'static,
    {
        self.use_cache_option(|cache: &HashMap<K, Query<K, V>>| {
            for q in cache.values() {
                q.mark_invalid();
            }
            Some(())
        });

        self
    }

    /// Invalidates all queries in the cache.
    ///
    /// Example:
    ///
    /// ```
    /// use leptos::*;
    /// use leptos_query::*;
    ///
    /// let client = use_query_client();
    /// client.invalidate_all_queries();
    ///
    /// ```
    ///
    pub fn invalidate_all_queries(&self) -> &Self {
        for cache in RefCell::try_borrow(&self.cache)
            .expect("invalidate_all_queries borrow")
            .values()
        {
            cache.invalidate();
        }
        self
    }

    /// Returns the current size of the cache.
    ///
    /// Example:
    /// ```
    /// use leptos::*;
    /// use leptos_query::*;
    ///
    /// let client = use_query_client();
    /// let cache_size = client.size();
    ///
    /// ```
    pub fn size(&self) -> Signal<usize> {
        let notify = self.notify;
        let cache = self.cache.clone();
        create_memo(move |_| {
            notify.get();
            let cache = RefCell::try_borrow(&cache).expect("size borrow");
            cache.values().map(|b| b.size()).sum()
        })
        .into()
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
    /// use leptos::*;
    /// use leptos_query::*;
    ///
    /// let client = use_query_client();
    /// let new_monkey: Monkey = todo!();
    ///
    /// // Overwrites existing cache data.
    /// client.set_query_data::<u32, Monkey>(1, |_| Some(new_monkey));
    ///
    /// // Only updates if query data exists.
    /// client.set_query_data::<u32, Monkey>(1, |maybe_monkey| {
    ///     let prev_monkey = maybe_monkey?;
    ///     new_monkey
    /// });
    /// ```
    pub fn update_query_data<K, V>(
        &self,
        key: K,
        updater: impl FnOnce(Option<&V>) -> Option<V> + 'static,
    ) where
        K: Clone + Eq + Hash + 'static,
        V: Clone + 'static,
    {
        enum SetResult {
            Inserted,
            Updated,
            Nothing,
        }
        let result = self.use_cache(move |(owner, cache)| {
            match cache.entry(key.clone()) {
                Entry::Occupied(entry) => {
                    let query = entry.get();

                    let updated = query.maybe_map_state(|state| match state {
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

                    if updated {
                        SetResult::Updated
                    } else {
                        SetResult::Nothing
                    }
                }
                Entry::Vacant(entry) => {
                    // Only insert query if updater returns Some.
                    if let Some(result) = updater(None) {
                        let query = with_owner(owner, || Query::new(key));
                        query.set_state(QueryState::Loaded(QueryData::now(result)));
                        entry.insert(query);
                        SetResult::Inserted
                    } else {
                        SetResult::Nothing
                    }
                }
            }
        });

        if let SetResult::Inserted = result {
            self.notify.set(());
        }
    }

    /// Mutate the existing data if it exists.
    pub fn update_query_data_mut<K, V>(
        &self,
        key: impl Borrow<K>,
        updater: impl FnOnce(&mut V),
    ) -> bool
    where
        K: Clone + Eq + Hash + 'static,
        V: Clone + 'static,
    {
        self.use_cache::<K, V, bool>(move |(_, cache)| {
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
        K: Clone + Eq + Hash + 'static,
        V: Clone + 'static,
    {
        self.use_cache::<K, V, bool>(move |(_, cache)| {
            if let Some(query) = cache.get(&key) {
                query.cancel()
            } else {
                false
            }
        })
    }

    fn use_cache_option<K, V, F, R>(&self, func: F) -> Option<R>
    where
        K: 'static,
        V: 'static,
        F: FnOnce(&HashMap<K, Query<K, V>>) -> Option<R>,
        R: 'static,
    {
        let cache = RefCell::try_borrow(&self.cache).expect("use_cache_option borrow");
        let type_key = (TypeId::of::<K>(), TypeId::of::<V>());
        let cache = cache.get(&type_key)?;
        let cache = cache
            .as_any()
            .downcast_ref::<CacheEntry<K, V>>()
            .expect(EXPECT_CACHE_ERROR);
        func(&cache.0)
    }

    fn use_cache_option_mut<K, V, F, R>(&self, func: F) -> Option<R>
    where
        K: 'static,
        V: 'static,
        F: FnOnce(&mut HashMap<K, Query<K, V>>) -> Option<R>,
        R: 'static,
    {
        let mut cache = RefCell::try_borrow_mut(&self.cache).expect("use_cache_option_mut borrow");
        let type_key = (TypeId::of::<K>(), TypeId::of::<V>());
        let cache = cache.get_mut(&type_key)?;
        let cache = cache
            .as_any_mut()
            .downcast_mut::<CacheEntry<K, V>>()
            .expect(EXPECT_CACHE_ERROR);
        func(&mut cache.0)
    }

    fn use_cache<K, V, R>(&self, func: impl FnOnce((Owner, &mut HashMap<K, Query<K, V>>)) -> R) -> R
    where
        K: Clone + 'static,
        V: Clone + 'static,
    {
        let mut cache = RefCell::try_borrow_mut(&self.cache).expect("use_cache borrow");

        let type_key = (TypeId::of::<K>(), TypeId::of::<V>());

        let cache: &mut Box<dyn CacheEntryTrait> = match cache.entry(type_key) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let wrapped: CacheEntry<K, V> = CacheEntry(HashMap::new());
                v.insert(Box::new(wrapped))
            }
        };

        let cache: &mut CacheEntry<K, V> = cache
            .as_any_mut()
            .downcast_mut::<CacheEntry<K, V>>()
            .expect(
            "Error: Query Cache Type Mismatch. This should not happen. Please file a bug report.",
        );

        func((self.owner, &mut cache.0))
    }

    fn get_or_create_query<K, V>(&self, key: K) -> Query<K, V>
    where
        K: Clone + Eq + Hash + 'static,
        V: Clone + 'static,
    {
        let result = self.use_cache(move |(owner, cache)| {
            let entry = cache.entry(key.clone());

            let (query, new) = match entry {
                Entry::Occupied(entry) => {
                    let entry = entry.into_mut();
                    (entry, false)
                }
                Entry::Vacant(entry) => {
                    let query = with_owner(owner, || Query::new(key));
                    (entry.insert(query.clone()), true)
                }
            };
            (query.clone(), new)
        });

        // Notify on insert.
        if result.1 {
            self.notify.set(());
        }

        result.0
    }

    pub(crate) fn get_query_signal<K, V>(&self, key: impl Fn() -> K + 'static) -> Memo<Query<K, V>>
    where
        K: Hash + Eq + Clone + 'static,
        V: Clone + 'static,
    {
        let client = self.clone();

        // This memo is crucial to avoid crazy amounts of lookups.
        create_memo(move |_| {
            let key = key();
            client.get_or_create_query(key)
        })
    }

    pub(crate) fn evict_and_notify<K, V: 'static>(&self, key: &K) -> Option<Query<K, V>>
    where
        K: Hash + Eq + 'static,
        V: 'static,
    {
        let result = self.use_cache_option_mut(move |cache| cache.remove(key));

        if result.is_some() {
            self.notify.set(());
        }
        result
    }
}

const EXPECT_CACHE_ERROR: &str =
    "Error: Query Cache Type Mismatch. This should not happen. Please file a bug report.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefetch_loads_data() {
        let _ = create_runtime();

        provide_query_client();
        let client = use_query_client();

        assert_eq!(0, client.size().get_untracked());

        let state = client.get_query_state::<u32, String>(|| 0);

        assert_eq!(None, state.get_untracked());

        client.prefetch_query(|| 0, |num: u32| async move { num.to_string() }, true);

        assert_eq!(
            Some("0".to_string()),
            state.get_untracked().and_then(|q| q.data().cloned())
        );

        assert!(matches!(
            state.get_untracked(),
            Some(QueryState::Loaded { .. })
        ));

        assert_eq!(1, client.size().get_untracked());

        client.invalidate_query::<u32, String>(0);

        assert!(matches!(
            state.get_untracked(),
            Some(QueryState::Invalid { .. })
        ));
    }

    #[test]
    fn set_query_data() {
        let _ = create_runtime();

        provide_query_client();
        let client = use_query_client();

        let state = client.get_query_state::<u32, String>(|| 0);
        assert_eq!(None, state.get_untracked());
        assert_eq!(0, client.size().get_untracked());

        client.update_query_data::<u32, String>(0, |_| None);

        assert_eq!(None, state.get_untracked());
        assert_eq!(0, client.size().get_untracked());

        client.update_query_data::<u32, String>(0, |_| Some("0".to_string()));

        assert_eq!(1, client.size().get_untracked());

        assert_eq!(
            Some("0".to_string()),
            state.get_untracked().and_then(|q| q.data().cloned())
        );

        assert!(matches!(
            state.get_untracked(),
            Some(QueryState::Loaded { .. })
        ));

        client.update_query_data::<u32, String>(0, |_| Some("1".to_string()));

        assert_eq!(
            Some("1".to_string()),
            state.get_untracked().and_then(|q| q.data().cloned())
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
            subscription.get();
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
