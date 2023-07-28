use crate::{
    query_executor::{create_executor, synchronize_state},
    *,
};
use leptos::*;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
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
#[derive(Clone)]
pub struct QueryClient {
    pub(crate) cx: Scope,
    pub(crate) cache: Rc<RefCell<HashMap<TypeId, Box<dyn Any>>>>,
}

pub(crate) type CacheEntry<K, V> = Rc<RefCell<HashMap<K, Query<K, V>>>>;

impl QueryClient {
    /// Creates a new Query Client.
    pub fn new(cx: Scope) -> Self {
        Self {
            cx,
            cache: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Prefetch a query and store it in cache. Returns QueryResult.
    /// If you don't need the result opt for [`QueryClient::prefetch_query()`](::prefetch_query)
    pub fn fetch_query<K, V, Fu>(
        &self,
        cx: Scope,
        key: impl Fn() -> K + 'static,
        fetcher: impl Fn(K) -> Fu + 'static,
        isomorphic: bool,
    ) -> QueryResult<V>
    where
        K: Hash + Eq + PartialEq + Clone + 'static,
        V: Clone + 'static,
        Fu: Future<Output = V> + 'static,
    {
        let state = get_state(cx, key);

        let state = Signal::derive(cx, move || state.get().0);

        let executor = Rc::new(create_executor(state, fetcher));

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

        QueryResult::new(
            cx,
            state,
            Signal::derive(self.cx, move || state.get().state.get().data().cloned()),
            executor,
        )
    }

    /// Prefetch a query and store it in cache.
    /// If you need the result opt for [`QueryClient::fetch_query()`](Self::fetch_query)
    pub fn prefetch_query<K, V, Fu>(
        &self,
        cx: Scope,
        key: impl Fn() -> K + 'static,
        query: impl Fn(K) -> Fu + 'static,
        isomorphic: bool,
    ) where
        K: Hash + Eq + PartialEq + Clone + 'static,
        V: Clone + 'static,
        Fu: Future<Output = V> + 'static,
    {
        let state = get_state(cx, key);

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

    /// Attempts to invalidate an entry in the Query Cache.
    /// Returns true if the entry was successfully invalidated.
    pub fn invalidate_query<K, V>(&self, key: &K) -> bool
    where
        K: Hash + Eq + PartialEq + Clone + 'static,
        V: Clone + 'static,
    {
        self.use_cache_option(|cache: &HashMap<K, Query<K, V>>| {
            cache.get(key).map(|state| state.mark_invalid())
        })
        .is_some()
    }

    /// Attempts to invalidate multiple entries in the Query Cache.
    /// Returns the keys that were successfully invalidated.
    pub fn invalidate_queries<'s, 'k, K, V, Keys>(&'s self, keys: Keys) -> Option<Vec<&'k K>>
    where
        K: Hash + Eq + PartialEq + Clone + 'static,
        V: Clone + 'static,
        Keys: Iterator<Item = &'k K>,
    {
        let cache = self.cache.borrow();

        if let Some(cache) = cache.get(&TypeId::of::<K>()) {
            if let Some(cache) = cache.downcast_ref::<CacheEntry<K, V>>() {
                let cache = cache.borrow();
                let invalidated = keys
                    .into_iter()
                    .filter(|key| {
                        if let Some(state) = cache.get(key) {
                            state.mark_invalid();
                            true
                        } else {
                            false
                        }
                    })
                    .collect::<Vec<_>>();
                return Some(invalidated);
            }
        }
        None
    }

    fn use_cache_option<K, V, R, F>(&self, func: F) -> Option<R>
    where
        K: Clone + 'static,
        V: Clone + 'static,
        R: 'static,
        F: FnOnce(&HashMap<K, Query<K, V>>) -> Option<R>,
    {
        let cache = self.cache.borrow();
        if let Some(cache) = cache.get(&TypeId::of::<K>()) {
            if let Some(cache) = cache.downcast_ref::<CacheEntry<K, V>>() {
                return func(&cache.borrow());
            }
        }
        None
    }
}

pub(crate) fn use_cache<K, V, R>(
    cx: Scope,
    func: impl FnOnce((Scope, &mut HashMap<K, Query<K, V>>)) -> R + 'static,
) -> R
where
    K: 'static,
    V: 'static,
{
    let client = use_query_client(cx);
    let mut cache = client.cache.borrow_mut();
    let entry = cache.entry(TypeId::of::<K>());

    let cache = entry.or_insert_with(|| {
        let wrapped: CacheEntry<K, V> = Rc::new(RefCell::new(HashMap::new()));
        Box::new(wrapped) as Box<dyn Any>
    });

    let mut cache = cache
        .downcast_ref::<CacheEntry<K, V>>()
        .expect("Query Cache Type Mismatch.")
        .borrow_mut();

    func((client.cx, &mut cache))
}

// bool is if the state was created!
pub(crate) fn get_state<K, V>(
    cx: Scope,
    key: impl Fn() -> K + 'static,
) -> Signal<(Query<K, V>, bool)>
where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + 'static,
{
    use std::collections::hash_map::Entry;
    let key = create_memo(cx, move |_| key());

    // Find relevant state.
    Signal::derive(cx, {
        move || {
            let key = key.get();
            use_cache(cx, {
                move |(root_scope, cache)| {
                    let entry = cache.entry(key.clone());

                    let (state, new) = match entry {
                        Entry::Occupied(entry) => {
                            let entry = entry.into_mut();
                            (entry, false)
                        }
                        Entry::Vacant(entry) => {
                            let state = Query::new(root_scope, key);
                            (entry.insert(state.clone()), true)
                        }
                    };
                    (state.clone(), new)
                }
            })
        }
    })
}
