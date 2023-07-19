use crate::QueryState;
use leptos::*;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    hash::Hash,
    rc::Rc,
};

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

    /// Attempts to invalidate an entry in the Query Cache.
    /// Returns true if the entry was successfully invalidated.
    pub fn invalidate<K, V>(&self, key: &K) -> bool
    where
        K: Hash + Eq + PartialEq + Clone + 'static,
        V: Clone + Serializable + 'static,
    {
        let cache = self.cache.borrow();

        if let Some(cache) = cache.get(&TypeId::of::<K>()) {
            if let Some(cache) = cache.downcast_ref::<CacheEntry<K, V>>() {
                return cache
                    .borrow_mut()
                    .get(key)
                    .map(|state| state.invalidate())
                    .is_some();
            }
        }
        false
    }

    /// Attempts to invalidate multiple entries in the Query Cache.
    /// Returns the keys that were successfully invalidated.
    pub fn invalidate_many<'s, 'k, K, V, Keys>(&'s self, keys: Keys) -> Option<Vec<&'k K>>
    where
        K: Hash + Eq + PartialEq + Clone + 'static,
        V: Clone + Serializable + 'static,
        Keys: Iterator<Item = &'k K>,
    {
        let cache = self.cache.borrow();

        if let Some(cache) = cache.get(&TypeId::of::<K>()) {
            if let Some(cache) = cache.downcast_ref::<CacheEntry<K, V>>() {
                let cache = cache.borrow_mut();
                let invalidated = keys
                    .into_iter()
                    .filter_map(|key| {
                        if let Some(state) = cache.get(&key) {
                            state.invalidate();
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
