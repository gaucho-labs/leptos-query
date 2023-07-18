use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use std::{cell::Cell, cell::RefCell, collections::HashMap, hash::Hash, rc::Rc};

use leptos::*;

use crate::instant::{get_instant, Instant};
use crate::{QueryCacheOptions, QueryState};

/// A cache that stores the results of a <K,V> async function.
#[derive(Clone)]
pub(crate) struct QueryCache<K, V>
where
    K: 'static,
    V: 'static,
{
    cx: Scope,

    // Configs.
    // This has to be RefCell because I don't want to enforce Copy.
    default_value: Rc<RefCell<Option<V>>>,
    stale_time: Rc<Cell<Option<Duration>>>,
    refetch_interval: Rc<Cell<Option<Duration>>>,
    // resource_option: Rc<Cell<ResourceOption>>,
    fetcher: Rc<dyn Fn(K) -> Pin<Box<dyn Future<Output = V>>>>,
    cache: Rc<RefCell<HashMap<K, QueryState<K, V>>>>,
}

impl<K, V> QueryCache<K, V>
where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + Serializable + 'static,
{
    // Creates a new query cache.
    pub(crate) fn new<Fu>(
        cx: Scope,
        fetcher: impl Fn(K) -> Fu + 'static,
        options: QueryCacheOptions<V>,
    ) -> Self
    where
        Fu: Future<Output = V> + 'static,
    {
        let fetcher = Rc::new(move |s| Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = V>>>);
        let QueryCacheOptions {
            default_value,
            stale_time,
            refetch_interval,
            ..
        } = options;

        Self {
            cx,
            fetcher,
            cache: Rc::new(RefCell::new(HashMap::new())),
            default_value: Rc::new(RefCell::new(default_value)),
            refetch_interval: Rc::new(Cell::new(refetch_interval)),
            stale_time: Rc::new(Cell::new(stale_time)),
        }
    }

    pub(crate) fn get(&self, key: K) -> QueryState<K, V> {
        let mut map = self.cache.borrow_mut();
        let entry = map.entry(key.clone());

        let result = entry.or_insert_with(|| {
            // This is so unwieldy.
            let fetch = self.fetcher.clone();

            // Save last update time on completion.
            let last_update: RwSignal<Option<Instant>> = create_rw_signal(self.cx, None);

            let fetcher = {
                move |key: K| {
                    let fetch = fetch.clone();

                    async move {
                        let result = fetch(key).await;
                        let instant = get_instant();
                        last_update.set(Some(instant));
                        result
                    }
                }
            };
            let key = key.clone();
            let get_key = {
                let key = key.clone();
                move || key.clone()
            };
            let default_value: Option<V> = self.default_value.borrow().clone();
            let resource =
                create_resource_with_initial_value(self.cx, get_key, fetcher, default_value);
            QueryState::new(
                self.cx,
                key,
                self.stale_time.clone(),
                self.refetch_interval.clone(),
                resource,
                last_update,
            )
        });

        result.clone()
    }

    pub(crate) fn invalidate(&self, key: &K) -> bool {
        let map = self.cache.borrow();
        if let Some(query) = map.get(key) {
            query.invalidate();
            true
        } else {
            false
        }
    }
}
