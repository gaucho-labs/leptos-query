use std::future::Future;
use std::pin::Pin;
use std::{cell::Cell, cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

use chrono::{Duration, Local};
use leptos::*;

#[derive(Clone)]
pub struct ResourceCache<K, V>
where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + Serializable + 'static,
{
    cx: Scope,
    fetcher: Rc<dyn Fn(K) -> Pin<Box<dyn Future<Output = V>>>>,
    cache: Rc<RefCell<HashMap<K, QueryState<K, V>>>>,
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

impl<K, V> ResourceCache<K, V>
where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + Serializable + 'static,
{
    pub fn provide_resource_cache<Fu>(cx: Scope, fetcher: impl Fn(K) -> Fu + 'static)
    where
        Fu: Future<Output = V> + 'static,
    {
        provide_context(cx, Self::new(cx, fetcher));
    }

    pub fn new<Fu>(cx: Scope, fetcher: impl Fn(K) -> Fu + 'static) -> Self
    where
        Fu: Future<Output = V> + 'static,
    {
        Self {
            cx,
            fetcher: Rc::new(move |s| Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = V>>>),
            cache: Rc::new(RefCell::new(HashMap::new())),
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
            let resource = create_resource(cx, get_key, fetcher);
            QueryState::new(cx, resource)
        });

        result.clone()
    }
}

impl<K: Clone + 'static, V: Clone + 'static> QueryState<K, V> {
    pub fn new(cx: Scope, resource: Resource<K, V>) -> Self {
        log!("Creating query state");
        Self {
            stale_time: Rc::new(Cell::new(Duration::seconds(10))),
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
