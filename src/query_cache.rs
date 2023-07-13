use std::future::Future;
use std::pin::Pin;
use std::{cell::Cell, cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

use chrono::{Duration, Local};
use leptos::*;

#[derive(Clone)]
pub struct QueryCache<K, V>
where
    K: 'static,
    V: 'static,
{
    cx: Scope,
    default_value: Option<V>,
    stale_time: Rc<Cell<Duration>>,
    fetcher: Rc<dyn Fn(K) -> Pin<Box<dyn Future<Output = V>>>>,
    cache: Rc<RefCell<HashMap<K, QueryState<K, V>>>>,
}

pub struct QueryOptions<V> {
    pub default_value: Option<V>,
    pub stale_time: Option<Duration>,
}

impl<V> Default for QueryOptions<V> {
    fn default() -> Self {
        Self {
            default_value: None,
            stale_time: None,
        }
    }
}

impl<K, V> QueryCache<K, V>
where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + Serializable + 'static,
{
    pub fn provide_resource_cache<Fu>(cx: Scope, fetcher: impl Fn(K) -> Fu + 'static)
    where
        Fu: Future<Output = V> + 'static,
    {
        Self::provide_resource_cache_with_options(cx, fetcher, QueryOptions::<V>::default());
    }

    pub fn provide_resource_cache_with_options<Fu>(
        cx: Scope,
        fetcher: impl Fn(K) -> Fu + 'static,
        options: QueryOptions<V>,
    ) where
        Fu: Future<Output = V> + 'static,
    {
        provide_context(cx, Self::new(cx, fetcher, options));
    }

    pub fn new<Fu>(cx: Scope, fetcher: impl Fn(K) -> Fu + 'static, options: QueryOptions<V>) -> Self
    where
        Fu: Future<Output = V> + 'static,
    {
        let fetcher = Rc::new(move |s| Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = V>>>);
        let QueryOptions {
            default_value,
            stale_time,
        } = options;

        Self {
            cx,
            fetcher,
            cache: Rc::new(RefCell::new(HashMap::new())),
            default_value,
            stale_time: Rc::new(Cell::new(stale_time.unwrap_or(Duration::milliseconds(0)))),
        }
    }

    pub fn get(&self, key: K) -> QueryState<K, V> {
        let mut map = self.cache.borrow_mut();
        log!("cache size: {:?}", map.len());
        let entry = map.entry(key.clone());
        // This is so unwieldy.
        let fetch = self.fetcher.clone();
        let fetcher = move |key: K| {
            let fetch = fetch.clone();
            async move { fetch(key).await }
        };
        let result = entry.or_insert_with(|| {
            let cx = self.cx;
            // TODO: Can I remove key func?
            let get_key = move || key.clone();
            let resource = create_resource_with_initial_value(
                cx,
                get_key,
                fetcher,
                self.default_value.clone(),
            );
            QueryState::new(cx, self.stale_time.clone(), resource)
        });

        result.clone()
    }

    pub fn get_many<'a>(&self, keys: impl Iterator<Item = &'a K>) -> Vec<QueryState<K, V>> {
        keys.into_iter().map(|k| self.get(k.clone())).collect()
    }

    // Do I have to un-register the resource?
    pub fn evict(&self, key: &K) -> Option<QueryState<K, V>> {
        let mut map = self.cache.borrow_mut();
        map.remove(key)
    }

    pub fn invalidate(&self, key: &K) -> bool {
        let map = self.cache.borrow();
        if let Some(query) = map.get(key) {
            query.invalidate();
            true
        } else {
            false
        }
    }

    pub fn set_stale_time(&self, stale_time: Duration) {
        self.stale_time.set(stale_time);
    }
}

#[derive(Clone, Debug)]
pub struct QueryState<K: 'static, V: 'static> {
    stale_time: Rc<Cell<Duration>>,
    // Epoch Millis timestamp of last update.
    last_updated: Rc<Cell<i64>>,
    // whether the resource must be refetched on next read.
    invalidated: RwSignal<bool>,
    resource: Rc<Resource<K, V>>,
}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    fn new(cx: Scope, stale_time: Rc<Cell<Duration>>, resource: Resource<K, V>) -> Self {
        log!("Creating query state");
        Self {
            stale_time,
            last_updated: Rc::new(Cell::new(get_instant())),
            invalidated: create_rw_signal(cx, false),
            resource: Rc::new(resource),
        }
    }

    pub fn refetch(&self) {
        self.resource.refetch()
    }

    pub fn invalidate(&self) {
        self.invalidated.set(true);
    }

    pub fn read(&self, cx: Scope) -> Signal<Option<V>> {
        let resource = self.resource.clone();
        let invalidated = self.invalidated.clone();
        let last_updated = self.last_updated.clone();
        let stale_time = self.stale_time.clone();
        Signal::derive(cx, move || {
            let resource = *resource;
            log!(
                "Retrieving resource, last_updated: {}, stale_time: {}",
                get_instant() - last_updated.get(),
                stale_time.get().num_milliseconds()
            );
            if invalidated() || {
                get_instant() - last_updated.get() > stale_time.get().num_milliseconds()
            } {
                log!("STALE!!!!");
                invalidated.set(false);
                resource.refetch();
                last_updated.set(get_instant());
                log!("refetched.");
            }
            resource.read(cx)
        })
    }
}

// TODO: fix this so it's not seconds.
// Can't use Instant because of wasm.
fn get_instant() -> i64 {
    Local::now().timestamp() * 1000
}
