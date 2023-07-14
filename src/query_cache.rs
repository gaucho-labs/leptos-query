use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use std::{cell::Cell, cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

use leptos::leptos_dom::helpers::TimeoutHandle;
use leptos::*;

use crate::instant::{get_instant, Instant};

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
    // Creates a query cache and provides it in a context object.
    pub fn provide_resource_cache<Fu>(cx: Scope, fetcher: impl Fn(K) -> Fu + 'static)
    where
        Fu: Future<Output = V> + 'static,
    {
        Self::provide_resource_cache_with_options(cx, fetcher, QueryOptions::<V>::default());
    }

    // Creates a query cache from the given options, and provides it in a context object.
    pub fn provide_resource_cache_with_options<Fu>(
        cx: Scope,
        fetcher: impl Fn(K) -> Fu + 'static,
        options: QueryOptions<V>,
    ) where
        Fu: Future<Output = V> + 'static,
    {
        provide_context(cx, Self::new(cx, fetcher, options));
    }

    // Creates a new query cache.
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
            let last_update: RwSignal<Option<Instant>> = create_rw_signal(self.cx, None);

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
            let get_key = {
                let key = key.clone();
                move || key.clone()
            };
            let resource = create_resource_with_initial_value(
                cx,
                get_key,
                fetcher,
                self.default_value.clone(),
            );
            QueryState::new(
                cx,
                key.clone(),
                self.stale_time.clone(),
                resource,
                last_update,
            )
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

    pub fn invalidate_all(&self) {
        self.cache
            .borrow()
            .values()
            .for_each(|query| query.invalidate());
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
    key: K,
    stale_time: Rc<Cell<Duration>>,
    // Epoch Millis timestamp of last update.
    last_updated: RwSignal<Option<Instant>>,
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
        // TODO: Should this be an Rc?
        key: K,
        stale_time: Rc<Cell<Duration>>,
        resource: Resource<K, V>,
        last_updated: RwSignal<Option<Instant>>,
    ) -> Self {
        log!("Creating query state");
        Self {
            key,
            stale_time,
            resource: Rc::new(resource),
            last_updated,
            invalidated: create_rw_signal(cx, false),
        }
    }

    pub fn key(&self) -> K {
        self.key.clone()
    }

    pub fn refetch(&self) {
        self.resource.refetch()
    }

    pub fn invalidate(&self) {
        self.invalidated.set(true);
    }

    pub fn loading(&self) -> Signal<bool> {
        self.resource.loading().into()
    }

    pub fn read(&self, cx: Scope) -> Signal<Option<V>> {
        let resource = self.resource.clone();
        let invalidated = self.invalidated.clone();
        let stale_time = self.stale_time.clone();
        let last_updated = self.last_updated.clone();

        // This is a kind of a hack. Happens when the resource is SSR'd.
        if resource.read(cx).is_some() && last_updated.get().is_none() {
            last_updated.set(Some(get_instant()));
        }

        // Saves last interval to be cleared on cleanup.
        // TODO: Ensure this is necessary.
        let interval: Rc<Cell<Option<TimeoutHandle>>> = Rc::new(Cell::new(None));
        let clean_up = {
            let interval = interval.clone();
            move || {
                if let Some(handle) = interval.take() {
                    handle.clear();
                }
            }
        };
        on_cleanup(cx, clean_up);

        // Sets timeout to refetch resource once it becomes stale.
        create_effect(cx, {
            let stale_time = stale_time.clone();
            let resource = resource.clone();
            let interval = interval.clone();

            move |maybe_handle: Option<Option<TimeoutHandle>>| {
                let maybe_handle = maybe_handle.flatten();
                if let Some(handle) = maybe_handle {
                    handle.clear();
                };
                if let Some(last_updated) = last_updated.get() {
                    let timeout = time_until_stale(last_updated, stale_time.get());

                    log!("Setting refetch timeout for: {:?}", timeout);
                    let resource = resource.clone();
                    let handle = set_timeout_with_handle(
                        move || {
                            resource.refetch();
                        },
                        timeout,
                    )
                    .ok();
                    interval.set(handle);
                    handle
                } else {
                    None
                }
            }
        });

        // Refetch query if invalidated.
        create_effect(cx, {
            let resource = resource.clone();
            move |_| {
                if let Some(_) = last_updated.get() {
                    if invalidated.get() {
                        log!("Refetching invalidated query");
                        invalidated.set_untracked(false);
                        resource.refetch();
                    }
                }
            }
        });

        Signal::derive(cx, move || resource.read(cx))
    }
}

impl<K, V> QueryState<K, V>
where
    K: Clone + 'static,
    V: Clone + PartialEq + 'static,
{
    // Render optimized version of read?
    // Not sure if this is needed.
    pub fn read_memo(&self, cx: Scope) -> Memo<Option<V>> {
        let signal = self.read(cx);
        create_memo(cx, move |_| signal.get())
    }
}

fn time_until_stale(last_updated: Instant, stale_time: Duration) -> Duration {
    let last_updated = last_updated.0.as_millis() as i64;
    let now = get_instant().0.as_millis() as i64;
    let stale_time = stale_time.as_millis() as i64;
    let result = (last_updated + stale_time) - now;
    let ensure_non_negative = result.max(0);
    Duration::from_millis(ensure_non_negative as u64)
}
