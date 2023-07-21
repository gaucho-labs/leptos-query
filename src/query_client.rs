use crate::QueryState;
use leptos::*;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    hash::Hash,
    rc::Rc,
};

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

pub(crate) type CacheEntry<K, V> = Rc<RefCell<HashMap<K, QueryState<K, V>>>>;

impl QueryClient {
    /// Creates a new Query Client.
    pub fn new(cx: Scope) -> Self {
        Self {
            cx,
            cache: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Attempts to retrieve data for a query from the Query Cache.
    // pub fn get_query_data<K, V>(&self, cx: Scope, key: &K) -> Option<QueryResult<V>>
    // where
    //     K: Hash + Eq + PartialEq + Clone + 'static,
    //     V: Clone + 'static,
    // {
    //     self.use_cache(|cache: &HashMap<K, QueryState<K, V>>| {
    //         cache
    //             .get(key)
    //             .map(|state| QueryResult::from_state(cx, state.clone()))
    //     })
    // }

    /// Attempts to invalidate an entry in the Query Cache.
    /// Returns true if the entry was successfully invalidated.
    pub fn invalidate_query<K, V>(&self, key: &K) -> bool
    where
        K: Hash + Eq + PartialEq + Clone + 'static,
        V: Clone + 'static,
    {
        self.use_cache_option(|cache: &HashMap<K, QueryState<K, V>>| {
            cache.get(key).map(|state| state.invalidate())
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
                            state.invalidate();
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
        F: FnOnce(&HashMap<K, QueryState<K, V>>) -> Option<R>,
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
    func: impl FnOnce((Scope, &mut HashMap<K, QueryState<K, V>>)) -> R + 'static,
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
