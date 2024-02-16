use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    rc::Rc,
};

use leptos::*;
use slotmap::SlotMap;

use crate::{
    cache_observer::{CacheEvent, CacheObserver},
    query::Query,
    QueryKey, QueryValue,
};

#[derive(Clone)]
pub(crate) struct QueryCache {
    owner: Owner,
    cache: Rc<RefCell<HashMap<(TypeId, TypeId), Box<dyn CacheEntryTrait>>>>,
    observers: Rc<RefCell<SlotMap<CacheObserverKey, Box<dyn CacheObserver>>>>,
    size: RwSignal<usize>,
}

slotmap::new_key_type! {
    struct CacheObserverKey;
}

pub(crate) struct CacheEntry<K, V>(HashMap<K, Query<K, V>>);

// Trait to enable cache introspection among distinct cache entry maps.
pub(crate) trait CacheEntryTrait: CacheSize + CacheInvalidate {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<K, V> CacheEntryTrait for CacheEntry<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
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
    K: QueryKey + 'static,
    V: QueryValue + 'static,
{
    fn invalidate(&self) {
        for (_, query) in self.0.iter() {
            query.mark_invalid();
        }
    }
}

impl QueryCache {
    pub(crate) fn new(owner: Owner) -> Self {
        Self {
            owner,
            cache: Rc::new(RefCell::new(HashMap::new())),
            observers: Rc::new(RefCell::new(SlotMap::with_key())),
            size: RwSignal::new(0),
        }
    }

    pub(crate) fn get_or_create_query<K, V>(&self, key: K) -> Query<K, V>
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        let query_cache = self.clone();
        let (query, created) = self.use_cache(move |cache| {
            let entry = cache.entry(key.clone());

            let (query, new) = match entry {
                Entry::Occupied(entry) => {
                    let entry = entry.into_mut();
                    (entry, false)
                }
                Entry::Vacant(entry) => {
                    let query = with_owner(query_cache.owner, || Query::new(key));
                    query_cache.notify_new_query(query.clone());
                    (entry.insert(query), true)
                }
            };
            (query.clone(), new)
        });

        // Notify on insert.
        if created {
            self.size.set(self.size.get_untracked() + 1);
        }

        query
    }

    pub(crate) fn get_query<K, V>(&self, key: K) -> Option<Query<K, V>>
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        self.use_cache_option_mut(move |cache| cache.get(&key).cloned())
    }

    pub(crate) fn get_query_signal<K, V>(&self, key: impl Fn() -> K + 'static) -> Memo<Query<K, V>>
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        let client = self.clone();

        // This memo is crucial to avoid crazy amounts of lookups.
        create_memo(move |_| {
            let key = key();
            client.get_or_create_query(key)
        })
    }

    pub(crate) fn size(&self) -> Signal<usize> {
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                let size_signal = self.size.clone();
                let cache = self.cache.clone();
                create_memo(move |_| {
                    let size = size_signal.get();
                    let cache = RefCell::try_borrow(&cache).expect("size borrow");
                    let real_size: usize = cache.values().map(|b| b.size()).sum();
                    assert!(size == real_size, "Cache size mismatch");
                    size
                }).into()
            } else {
                self.size.into()
            }
        }
    }

    pub(crate) fn evict_query<K, V>(&self, key: &K) -> bool
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        let result = self.use_cache_option_mut::<K, V, _, _>(move |cache| cache.remove(key));

        if let Some(query) = result {
            self.notify_query_eviction(&query.key);
            self.size.set(self.size.get_untracked() - 1);
            query.dispose();
            true
        } else {
            false
        }
    }

    pub fn invalidate_all_queries(&self) {
        for cache in RefCell::try_borrow(&self.cache)
            .expect("invalidate_all_queries borrow")
            .values()
        {
            cache.invalidate();
        }
    }

    pub(crate) fn use_cache_option<K, V, F, R>(&self, func: F) -> Option<R>
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
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

    pub(crate) fn use_cache_option_mut<K, V, F, R>(&self, func: F) -> Option<R>
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
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

    pub(crate) fn use_cache<K, V, R>(
        &self,
        func: impl FnOnce(&mut HashMap<K, Query<K, V>>) -> R,
    ) -> R
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
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
            .expect(EXPECT_CACHE_ERROR);

        func(&mut cache.0)
    }

    pub(crate) fn use_cache_entry<K, V>(
        &self,
        key: K,
        // Return true if the entry was added.
        func: impl FnOnce((Owner, Entry<'_, K, Query<K, V>>)) -> bool,
    ) where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        let query_cache = self;

        let inserted = {
            let key = key.clone();
            self.use_cache(|cache| {
                let entry = cache.entry(key);
                let inserted = func((query_cache.owner, entry));
                inserted
            })
        };

        if inserted {
            query_cache.size.set(self.size.get_untracked() + 1);
            if let Some(query) = self.get_query::<K, V>(key) {
                self.notify_new_query(query)
            }
        }
    }

    pub(crate) fn register_query_observer(&self, observer: impl CacheObserver + 'static) {
        self.observers
            .try_borrow_mut()
            .expect("register_query_observer borrow mut")
            .insert(Box::new(observer));
    }

    pub(crate) fn notify<K, V>(&self, notification: CacheNotification<K, V>)
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        let event = match notification {
            CacheNotification::UpdatedState(query) => CacheEvent::updated(query.into()),
            CacheNotification::NewObserver(key) => CacheEvent::observer_added(&key),
            CacheNotification::ObserverRemoved(key) => CacheEvent::observer_removed(&key),
        };
        self.notify_observers(event);
    }

    pub(crate) fn notify_new_query<K, V>(&self, query: Query<K, V>)
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        // TODO: CHECK THIS.
        // this is crucial to avoid the signal going out of scope once the use_query instance unmounts.
        let event = CacheEvent::created(query);
        self.notify_observers(event);
    }

    pub(crate) fn notify_query_eviction<K>(&self, key: &K)
    where
        K: QueryKey + 'static,
    {
        let event = CacheEvent::removed(key);
        self.notify_observers(event);
    }

    pub(crate) fn notify_observers(&self, notification: CacheEvent) {
        let observers = self
            .observers
            .try_borrow()
            .expect("notify_observers borrow");
        for observer in observers.values() {
            observer.process_cache_event(notification.clone())
        }
    }
}

pub(crate) enum CacheNotification<K, V> {
    UpdatedState(Query<K, V>),
    NewObserver(K),
    ObserverRemoved(K),
}

const EXPECT_CACHE_ERROR: &str =
    "Error: Query Cache Type Mismatch. This should not happen. Please file a bug report.";
