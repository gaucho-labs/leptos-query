use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    future::Future,
    hash::Hash,
    rc::Rc,
};

use leptos::*;

use crate::{query_cache::QueryCache, QueryCacheOptions, QueryOptions, QueryState};

#[derive(Clone)]
pub struct QueryClient {
    cx: Scope,
    cache: Rc<RefCell<HashMap<TypeId, Box<dyn Any>>>>,
}

impl QueryClient {
    pub fn new(cx: Scope) -> Self {
        Self {
            cx,
            cache: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    // Attempts to invalidate an entry in the Query Cache.
    // Returns true if the entry was successfully invalidated.
    pub fn invalidate<K, V>(&self, key: K) -> bool
    where
        K: Hash + Eq + PartialEq + Clone + 'static,
        V: Clone + Serializable + 'static,
    {
        let cache = self.cache.borrow();

        if let Some(cache) = cache.get(&TypeId::of::<K>()) {
            if let Some(cache) = cache.downcast_ref::<Rc<RefCell<QueryCache<K, V>>>>() {
                return cache.borrow_mut().invalidate(&key);
            }
        }
        false
    }

    // Attempts to invalidate many entries in the Query Cache.
    // Returns the keys that were successfully invalidated.
    pub fn invalidate_many<K, V, Keys>(&self, keys: Keys) -> Option<Vec<K>>
    where
        K: Hash + Eq + PartialEq + Clone + 'static,
        V: Clone + Serializable + 'static,
        Keys: Iterator<Item = K>,
    {
        let cache = self.cache.borrow();

        if let Some(cache) = cache.get(&TypeId::of::<K>()) {
            if let Some(cache) = cache.downcast_ref::<Rc<RefCell<QueryCache<K, V>>>>() {
                let cache = cache.borrow_mut();
                let invalidated = keys
                    .into_iter()
                    .filter_map(|key| {
                        if cache.invalidate(&key) {
                            Some(key)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                return Some(invalidated);
            }
        }
        None
    }
}

// Provides a Query Client to the current scope.
pub fn provide_query_client(cx: Scope) {
    provide_context(cx, QueryClient::new(cx));
}

// Retrieves a Query Client from the current scope.
pub fn use_query_client(cx: Scope) -> QueryClient {
    use_context::<QueryClient>(cx).expect("Query Client Missing.")
}

/// Creates a query for a pair of associated types <K, V>
///
///
pub fn use_query<K, V, Fu>(
    cx: Scope,
    key: K,
    query: impl Fn(K) -> Fu + 'static,
    // These options are shared between all queries of the same type.
    common_options: QueryCacheOptions<V>,
) -> QueryState<K, V>
where
    Fu: Future<Output = V> + 'static,
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + Serializable + 'static,
{
    use_query_options(cx, key, query, common_options, None)
}

pub fn use_query_options<K, V, Fu>(
    cx: Scope,
    key: K,
    query: impl Fn(K) -> Fu + 'static,
    // These options are shared between all queries of the same type.
    common_options: QueryCacheOptions<V>,
    // These options are specific to this query instance.
    options: Option<QueryOptions>,
) -> QueryState<K, V>
where
    Fu: Future<Output = V> + 'static,
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + Serializable + 'static,
{
    let cache = use_query_client(cx);
    let root_scope = cache.cx;
    let mut cache = cache.cache.borrow_mut();
    let entry = cache.entry(TypeId::of::<K>());

    let cache = entry.or_insert_with(|| {
        let cache = QueryCache::new(root_scope, query, common_options.clone());
        let wrapped = Rc::new(RefCell::new(cache));
        let boxed = Box::new(wrapped) as Box<dyn Any>;
        boxed
    });

    let cache = cache
        .downcast_ref::<Rc<RefCell<QueryCache<K, V>>>>()
        .expect(
            "Query Cache Type Mismatch. Ensure that every use_query uses the same (K, V) types.",
        );

    let cache = cache.borrow();

    let state = cache.get(key);

    if let Some(options) = options {
        state.set_options(options);
    }
    state
}
