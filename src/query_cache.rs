use std::future::Future;
use std::ops::{Add, Sub};
use std::pin::Pin;
use std::time::Duration;
use std::{cell::Cell, cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

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

/**
 * Options for a Query Cache.
 */
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
            stale_time: Rc::new(Cell::new(stale_time.unwrap_or(Duration::from_millis(0)))),
        }
    }

    pub fn get(&self, key: K) -> QueryState<K, V> {
        let mut map = self.cache.borrow_mut();
        log!("cache size: {:?}", map.len());
        let entry = map.entry(key.clone());

        let result = entry.or_insert_with(|| {
            // This is so unwieldy.
            let fetch = self.fetcher.clone();

            // Save last update time on completion.
            let last_update = Rc::new(Cell::new(None as Option<Instant>));

            let fetcher = {
                let last_update = last_update.clone();
                move |key: K| {
                    let fetch = fetch.clone();
                    let last_update = last_update.clone();

                    async move {
                        let result = fetch(key).await;
                        let instant = get_instant();
                        last_update.set(Some(instant));
                        result
                    }
                }
            };
            let cx = self.cx;
            // TODO: Can I remove key func?
            let get_key = move || key.clone();
            let resource = create_resource_with_initial_value(
                cx,
                get_key,
                fetcher,
                self.default_value.clone(),
            );
            QueryState::new(cx, self.stale_time.clone(), resource, last_update)
        });

        result.clone()
    }

    pub fn get_many<'a>(&self, keys: impl Iterator<Item = &'a K>) -> Vec<QueryState<K, V>> {
        keys.into_iter().map(|k| self.get(k.clone())).collect()
    }

    // TODO: Unregister resource with leptos runtime.
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
pub struct QueryState<K, V>
where
    K: 'static,
    V: 'static,
{
    stale_time: Rc<Cell<Duration>>,
    // Epoch Millis timestamp of last update.
    last_updated: Rc<Cell<Option<Instant>>>,
    // Whether the resource must be refetched on next read.
    invalidated: RwSignal<bool>,
    resource: Rc<Resource<K, V>>,
}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    fn new(
        cx: Scope,
        stale_time: Rc<Cell<Duration>>,
        resource: Resource<K, V>,
        last_updated: Rc<Cell<Option<Instant>>>,
    ) -> Self {
        log!("Creating query state");
        Self {
            stale_time,
            resource: Rc::new(resource),
            last_updated,
            invalidated: create_rw_signal(cx, false),
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
        let stale_time = self.stale_time.clone();
        let last_updated = self.last_updated.clone();

        if resource.read(cx).is_some() && last_updated.get().is_none() {
            log!("Resource was Server Side Rendered!");
            last_updated.set(Some(get_instant()));
        }

        Signal::derive(cx, move || {
            let resource = *resource;
            let now = get_instant();
            if let Some(last_updated) = last_updated.get() {
                log!(
                    "Reading resource, now: {:?}, last_updated: {:?}, diff: {:?}, stale_time: {}",
                    now,
                    last_updated,
                    get_instant() - last_updated,
                    stale_time.get().as_millis()
                );
                if invalidated() || (now - last_updated) > stale_time.get() {
                    log!("Refetching!");
                    invalidated.set_untracked(false);
                    resource.refetch();
                }
            }
            resource.read(cx)
        })
    }
}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + PartialEq + 'static,
{
    pub fn read_memo(&self, cx: Scope) -> Memo<Option<V>> {
        let signal = self.read(cx);
        create_memo(cx, move |_| signal())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Hash)]
struct Instant(std::time::Duration);

impl Sub<Instant> for Instant {
    type Output = Duration;

    #[inline]
    fn sub(self, rhs: Instant) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add<Instant> for Instant {
    type Output = Duration;
    #[inline]
    fn add(self, rhs: Instant) -> Self::Output {
        self.0 + rhs.0
    }
}

fn get_instant() -> Instant {
    use cfg_if::cfg_if;
    cfg_if! { if #[cfg(not(feature = "ssr"))] {
        let millis = js_sys::Date::now();
        let duration = std::time::Duration::from_millis(millis as u64);
        Instant(duration)
    }}
    cfg_if! { if #[cfg(feature = "ssr")] {
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("System clock was before 1970.");
        Instant(duration)
    }}
}
