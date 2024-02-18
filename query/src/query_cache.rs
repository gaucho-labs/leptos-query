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
    QueryKey, QueryOptions, QueryValue,
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
        let query_cache = self;

        let mut created = false;

        let query = self.use_cache(|cache| {
            let entry = cache.entry(key.clone());

            let query = match entry {
                Entry::Occupied(entry) => {
                    let entry = entry.into_mut();
                    entry
                }
                Entry::Vacant(entry) => {
                    let query = with_owner(query_cache.owner, || Query::new(key));
                    query_cache.notify_new_query(query.clone());
                    created = true;
                    entry.insert(query)
                }
            };
            query.clone()
        });

        // It's necessary to delay the size update until we are out of the borrow, to avoid borrow errors.
        if created {
            self.size.update(|size| *size = *size + 1);
        }

        query
    }

    pub(crate) fn get_query<K, V>(&self, key: &K) -> Option<Query<K, V>>
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        self.use_cache_option(move |cache| cache.get(key).cloned())
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
            self.notify_query_eviction(query.get_key());
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
        func: impl FnOnce((Owner, Option<&Query<K, V>>)) -> Option<Query<K, V>>,
    ) where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
        let query_cache = self;

        let mut created = false;

        self.use_cache(|cache| match cache.entry(key) {
            Entry::Vacant(entry) => {
                if let Some(query) = func((query_cache.owner, None)) {
                    entry.insert(query.clone());
                    // Report insert.
                    created = true;
                    self.notify_new_query(query)
                }
            }
            Entry::Occupied(mut entry) => {
                let query = entry.get();
                if let Some(query) = func((query_cache.owner, Some(query))) {
                    entry.insert(query);
                }
            }
        });

        // It's necessary to delay the size update until we are out of the borrow, to avoid borrow errors.
        if created {
            self.size.update(|size| *size = *size + 1);
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
            CacheNotification::NewObserver(observer) => {
                CacheEvent::observer_added(&observer.key, observer.options)
            }
            CacheNotification::ObserverRemoved(key) => CacheEvent::observer_removed(&key),
        };
        self.notify_observers(event);
    }

    pub(crate) fn notify_new_query<K, V>(&self, query: Query<K, V>)
    where
        K: QueryKey + 'static,
        V: QueryValue + 'static,
    {
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
    NewObserver(NewObserver<K, V>),
    ObserverRemoved(K),
}

pub(crate) struct NewObserver<K, V> {
    pub(crate) key: K,
    pub(crate) options: QueryOptions<V>,
}

const EXPECT_CACHE_ERROR: &str =
    "Error: Query Cache Type Mismatch. This should not happen. Please file a bug report.";
