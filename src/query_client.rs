use crate::{
    query_executor::{create_executor, synchronize_state},
    *,
};
use leptos::*;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::hash_map::Entry,
    collections::HashMap,
    future::Future,
    hash::Hash,
    rc::Rc,
};

/// Provides a Query Client to the current scope.
pub fn provide_query_client(cx: Scope) {
    provide_context(cx, QueryClient::new(cx));
}

/// Retrieves a Query Client from the current scope.
pub fn use_query_client(cx: Scope) -> QueryClient {
    use_context::<QueryClient>(cx).expect("Query Client Missing.")
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
    pub(crate) cx: Scope,
    // Signal to indicate a cache entry has been added or removed.
    pub(crate) notify: RwSignal<()>,
    pub(crate) cache: Rc<RefCell<HashMap<(TypeId, TypeId), Box<dyn CacheEntryTrait>>>>,
}

pub(crate) struct CacheEntry<K: 'static, V: 'static>(HashMap<K, Query<K, V>>);

// Trait to enable cache introspection among distinct cache entry maps.
pub(crate) trait CacheEntryTrait: CacheSize {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<K, V> CacheEntryTrait for CacheEntry<K, V> {
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

impl QueryClient {
    /// Creates a new Query Client.
    pub fn new(cx: Scope) -> Self {
        Self {
            cx,
            notify: create_rw_signal(cx, ()),
            cache: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Prefetch a query and store it in cache. Returns QueryResult.
    /// If the entry already exists it will still be refetched.
    ///
    /// If you don't need the result opt for [`prefetch_query()`](Self::prefetch_query)
    pub fn fetch_query<K, V, Fu>(
        &self,
        cx: Scope,
        key: impl Fn() -> K + 'static,
        fetcher: impl Fn(K) -> Fu + 'static,
        isomorphic: bool,
    ) -> QueryResult<V, impl RefetchFn>
    where
        K: Hash + Eq + Clone + 'static,
        V: Clone + 'static,
        Fu: Future<Output = V> + 'static,
    {
        let state = self.get_query_signal(cx, key);

        let state = Signal::derive(cx, move || state.get().0);

        let executor = create_executor(state, fetcher);

        let sync = {
            let executor = executor.clone();
            move |_| {
                let _ = state.get();
                executor()
            }
        };
        if isomorphic {
            create_isomorphic_effect(cx, sync);
        } else {
            create_effect(cx, sync);
        }

        synchronize_state(cx, state, executor.clone());

        create_query_result(
            cx,
            state,
            Signal::derive(self.cx, move || state.get().state.get().data().cloned()),
            executor,
        )
    }

    /// Prefetch a query and store it in cache.
    /// If the entry already exists it will still be refetched.
    ///
    /// If you need the result opt for [`fetch_query()`](Self::fetch_query)
    pub fn prefetch_query<K, V, Fu>(
        &self,
        cx: Scope,
        key: impl Fn() -> K + 'static,
        query: impl Fn(K) -> Fu + 'static,
        isomorphic: bool,
    ) where
        K: Hash + Eq + Clone + 'static,
        V: Clone + 'static,
        Fu: Future<Output = V> + 'static,
    {
        let state = self.get_query_signal(cx, key);

        let state = Signal::derive(cx, move || state.get().0);

        let executor = create_executor(state, query);

        let sync = {
            move |_| {
                let _ = state.get();
                executor()
            }
        };
        if isomorphic {
            create_isomorphic_effect(cx, sync);
        } else {
            create_effect(cx, sync);
        }
    }

    /// Retrieve the current state for an existing query.
    /// If the query does not exist, [`None`](Option::None) will be returned.
    pub fn get_query_state<K, V>(
        self,
        cx: Scope,
        key: impl Fn() -> K + 'static,
    ) -> Signal<Option<QueryState<V>>>
    where
        K: Hash + Eq + Clone + 'static,
        V: Clone,
    {
        let client = self.clone();

        // Memoize state to avoid unnecessary hashmap lookups.
        let maybe_query = create_memo(cx, move |_| {
            let key = key();
            client.notify.get();
            client.use_cache_option(|cache: &HashMap<K, Query<K, V>>| cache.get(&key).cloned())
        });

        synchronize_observer(cx, maybe_query.into());

        Signal::derive(cx, move || maybe_query.get().map(|s| s.state.get()))
    }

    /// Attempts to invalidate an entry in the Query Cache.
    /// Matching query is marked as invalid, and will be refetched in background once it's active.
    ///
    /// Returns true if the entry was successfully invalidated.
    pub fn invalidate_query<K, V>(&self, key: &K) -> bool
    where
        K: Hash + Eq + Clone + 'static,
        V: Clone + 'static,
    {
        self.use_cache_option(|cache: &HashMap<K, Query<K, V>>| {
            cache.get(key).map(|state| state.mark_invalid())
        })
        .unwrap_or(false)
    }

    /// Attempts to invalidate multiple entries in the Query Cache.
    /// All matching queries are immediately marked as invalid and active queries are refetched in the background.
    ///
    /// Returns the keys that were successfully invalidated.
    pub fn invalidate_queries<'s, 'k, K, V, Keys>(&'s self, keys: Keys) -> Option<Vec<&'k K>>
    where
        K: Hash + Eq + Clone + 'static,
        V: Clone + 'static,
        Keys: IntoIterator<Item = &'k K> + 'static,
    {
        // Find all states, drop borrow, then mark invalid.
        let cache_borrowed = self.cache.borrow();
        let type_key = (TypeId::of::<K>(), TypeId::of::<V>());
        let cache = cache_borrowed.get(&type_key)?;
        let cache = cache.as_any().downcast_ref::<CacheEntry<K, V>>()?;
        let result = keys
            .into_iter()
            .filter(|key| {
                cache
                    .0
                    .get(key)
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
    /// #[component]
    /// fn SomeComponent(cx: Scope) -> impl IntoView {
    ///     let client = use_query_client(cx);
    ///     client.invalidate_all_queries::<String, Monkey>()
    ///
    ///     view!{cx,
    ///         <div>
    ///         </div>
    ///     }
    /// }
    ///
    /// ```
    pub fn invalidate_all_queries<K, V>(&self) -> &Self
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

    /// Returns the current size of the cache.
    pub fn size(&self) -> Signal<usize> {
        let notify = self.notify;
        let cache = self.cache.clone();
        Signal::derive(self.cx, move || {
            notify.get();
            let cache = cache.borrow();
            cache.values().map(|b| b.size()).sum()
        })
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
    /// #[component]
    /// fn SomeComponent(cx: Scope) -> impl IntoView {
    ///    let client = use_query_client(cx);
    ///    client.set_query_data::<u32, Monkey>(1, |maybe_data| {
    ///         if let Some(data) = maybe_data {
    ///             None
    ///         } else {
    ///            Some(QueryData::now(Monkey::new()))
    ///         }
    ///    })
    ///     
    ///     view!{cx,
    ///         <div>
    ///         </div>
    ///    }
    /// }
    ///
    /// ```
    pub fn set_query_data<K, V>(
        &self,
        key: K,
        updater: impl FnOnce(Option<&QueryData<V>>) -> Option<QueryData<V>> + 'static,
    ) -> &Self
    where
        K: Clone + Eq + Hash + 'static,
        V: Clone + 'static,
    {
        enum SetResult {
            Inserted,
            Updated,
            Nothing,
        }
        let result = self.use_cache(
            move |(root_scope, cache): (Scope, &mut HashMap<K, Query<K, V>>)| match cache
                .entry(key.clone())
            {
                Entry::Occupied(entry) => {
                    let query = entry.get();
                    // Only update query data if updater returns Some.
                    if let Some(result) = updater(query.state.get_untracked().query_data()) {
                        query.state.set(QueryState::Loaded(result));
                        SetResult::Updated
                    } else {
                        SetResult::Nothing
                    }
                }
                Entry::Vacant(entry) => {
                    // Only insert query if updater returns Some.
                    if let Some(result) = updater(None) {
                        let query = Query::new(root_scope, key);
                        query.state.set(QueryState::Loaded(result));
                        entry.insert(query);
                        SetResult::Inserted
                    } else {
                        SetResult::Nothing
                    }
                }
            },
        );

        if let SetResult::Inserted = result {
            self.notify.set(());
        }

        self
    }

    /// A synchronous function that can be used to immediately set a query's data. Will use [`Instant::now`](Instant::now) for [`updated_at`](QueryData::updated_at) if set is successful.
    ///
    /// If the query does not exist, it will be created.
    ///
    /// If you need to fetch the data asynchronously, use [`fetch_query`](Self::fetch_query) or [`prefetch_query`](Self::prefetch_query).
    ///
    /// If the updater function returns [`None`](Option::None), the query data will not be updated.
    ///
    /// If the updater function receives [`None`](Option::None) as input, you can return [`None`](Option::None) to bail out of the update and thus not create a new cache entry.
    pub fn set_query_data_now<K, V>(
        &self,
        key: K,
        updater: impl FnOnce(Option<&V>) -> Option<V> + 'static,
    ) -> &Self
    where
        K: Clone + Eq + Hash + 'static,
        V: Clone + 'static,
    {
        self.set_query_data(key, move |maybe_data| {
            let input = maybe_data.map(|data| &data.data);
            let result = updater(input);
            result.map(|data| QueryData::now(data))
        });
        self
    }

    fn use_cache_option<K, V, F, R>(&self, func: F) -> Option<R>
    where
        K: 'static,
        V: 'static,
        F: FnOnce(&HashMap<K, Query<K, V>>) -> Option<R>,
        R: 'static,
    {
        let cache = self.cache.borrow();
        let type_key = (TypeId::of::<K>(), TypeId::of::<V>());
        let cache = cache.get(&type_key)?;
        let cache = cache.as_any().downcast_ref::<CacheEntry<K, V>>()?;
        func(&cache.0)
    }

    fn use_cache_option_mut<K, V, F, R>(&self, func: F) -> Option<R>
    where
        K: 'static,
        V: 'static,
        F: FnOnce(&mut HashMap<K, Query<K, V>>) -> Option<R>,
        R: 'static,
    {
        let mut cache = self.cache.borrow_mut();
        let type_key = (TypeId::of::<K>(), TypeId::of::<V>());
        let cache = cache.get_mut(&type_key)?;
        let cache = cache.as_any_mut().downcast_mut::<CacheEntry<K, V>>()?;
        func(&mut cache.0)
    }

    fn use_cache<K, V, R>(
        &self,
        func: impl FnOnce((Scope, &mut HashMap<K, Query<K, V>>)) -> R + 'static,
    ) -> R
    where
        K: 'static,
        V: 'static,
    {
        let mut cache = self.cache.borrow_mut();

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

        func((self.cx, &mut cache.0))
    }

    fn get_or_create_query<K, V>(&self, key: K) -> (Query<K, V>, bool)
    where
        K: Clone + Eq + Hash + 'static,
        V: Clone + 'static,
    {
        let result = self.use_cache(move |(root_scope, cache)| {
            let entry = cache.entry(key.clone());

            let (query, new) = match entry {
                Entry::Occupied(entry) => {
                    let entry = entry.into_mut();
                    (entry, false)
                }
                Entry::Vacant(entry) => {
                    let query = Query::new(root_scope, key);
                    (entry.insert(query.clone()), true)
                }
            };
            (query.clone(), new)
        });

        // Notify on insert.
        if result.1 {
            self.notify.set(());
        }

        result
    }

    pub(crate) fn get_query_signal<K, V>(
        &self,
        cx: Scope,
        key: impl Fn() -> K + 'static,
    ) -> Signal<(Query<K, V>, bool)>
    where
        K: Hash + Eq + Clone + 'static,
        V: Clone + 'static,
    {
        let client = self.clone();

        // This memo is crucial to avoid crazy amounts of lookups.
        create_memo(cx, move |_| {
            let key = key();
            client.get_or_create_query(key)
        })
        .into()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefetch_loads_data() {
        run_scope(create_runtime(), |cx| {
            provide_query_client(cx);
            let client = use_query_client(cx);

            assert_eq!(0, client.clone().size().get_untracked());

            let state = client.clone().get_query_state::<u32, String>(cx, || 0);

            assert_eq!(None, state.get_untracked());

            client.clone().prefetch_query(
                cx,
                || 0,
                |num: u32| async move { num.to_string() },
                true,
            );

            assert_eq!(
                Some("0".to_string()),
                state.get_untracked().and_then(|q| q.data().cloned())
            );

            assert!(matches!(
                state.get_untracked(),
                Some(QueryState::Loaded { .. })
            ));

            assert_eq!(1, client.clone().size().get_untracked());

            client.clone().invalidate_query::<u32, String>(&0);

            assert!(matches!(
                state.get_untracked(),
                Some(QueryState::Invalid { .. })
            ));
        });
    }

    #[test]
    fn set_query_data() {
        run_scope(create_runtime(), |cx| {
            provide_query_client(cx);
            let client = use_query_client(cx);

            let state = client.clone().get_query_state::<u32, String>(cx, || 0);
            assert_eq!(None, state.get_untracked());
            assert_eq!(0, client.clone().size().get_untracked());

            client.clone().set_query_data::<u32, String>(0, |_| None);

            assert_eq!(None, state.get_untracked());
            assert_eq!(0, client.size().get_untracked());

            client.clone().set_query_data::<u32, String>(0, |_| {
                Some(QueryData {
                    data: "0".to_string(),
                    updated_at: Instant::now(),
                })
            });

            assert_eq!(1, client.clone().size().get_untracked());

            assert_eq!(
                Some("0".to_string()),
                state.get_untracked().and_then(|q| q.data().cloned())
            );

            assert!(matches!(
                state.get_untracked(),
                Some(QueryState::Loaded { .. })
            ));

            client.clone().set_query_data::<u32, String>(0, |_| {
                Some(QueryData {
                    data: "1".to_string(),
                    updated_at: Instant::now(),
                })
            });

            assert_eq!(
                Some("1".to_string()),
                state.get_untracked().and_then(|q| q.data().cloned())
            );
        });
    }

    #[test]
    fn can_use_same_key_with_different_value_types() {
        run_scope(create_runtime(), |cx| {
            provide_query_client(cx);
            let client = use_query_client(cx);

            client.set_query_data_now::<u32, String>(0, |_| Some("0".to_string()));

            client.set_query_data_now::<u32, u32>(0, |_| Some(1234));

            assert_eq!(2, client.size().get_untracked());
        });
    }

    #[test]
    fn can_invalidate_while_subscribed() {
        run_scope(create_runtime(), |cx| {
            provide_query_client(cx);
            let client = use_query_client(cx);

            let subscription = client.clone().get_query_state::<u32, u32>(cx, || 0_u32);

            create_isomorphic_effect(cx, move |_| {
                subscription.get();
            });

            client.set_query_data_now::<u32, u32>(0_u32, |_| Some(1234));

            assert!(client.invalidate_query::<u32, u32>(&0));
        });
    }
}
