use std::cell::RefCell;
use std::future::Future;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{pin::Pin, rc::Rc};

use leptos::logging;
use slotmap::{new_key_type, SlotMap};

use crate::query::Query;
use crate::{QueryKey, QueryOptions, QueryState, QueryValue};

#[derive(Clone)]
pub struct QueryObserver<K, V> {
    id: ObserverKey,
    query: Rc<RefCell<Option<Query<K, V>>>>,
    fetcher: Option<Fetcher<K, V>>,
    options: QueryOptions<V>,
    listeners: Rc<RefCell<SlotMap<ListenerKey, Box<dyn Fn(&QueryState<V>)>>>>,
}

type Fetcher<K, V> = Rc<dyn Fn(K) -> Pin<Box<dyn Future<Output = V>>>>;

new_key_type! {
    pub (crate) struct ListenerKey;
}

impl<K, V> std::fmt::Debug for QueryObserver<K, V>
where
    K: QueryKey + 'static,
    V: QueryValue + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryObserver")
            .field("id", &self.id)
            // .field("query", &self.query)
            .field("fetcher", &"...")
            .field("options", &self.options)
            .field("listeners", &self.listeners.borrow().len())
            .finish()
    }
}

impl<K, V> QueryObserver<K, V>
where
    K: QueryKey + 'static,
    V: QueryValue + 'static,
{
    pub fn with_fetcher<F, Fu>(fetcher: F, options: QueryOptions<V>, query: Query<K, V>) -> Self
    where
        F: Fn(K) -> Fu + 'static,
        Fu: Future<Output = V> + 'static,
    {
        let fetcher =
            Some(
                Rc::new(move |s| Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = V>>>)
                    as Fetcher<K, V>,
            );
        let query = Rc::new(RefCell::new(Some(query)));
        let id = next_id();

        Self {
            id,
            query,
            fetcher,
            options,
            listeners: Rc::new(RefCell::new(SlotMap::with_key())),
        }
    }

    pub fn get_fetcher(&self) -> Option<Fetcher<K, V>> {
        self.fetcher.clone()
    }

    pub fn get_id(&self) -> ObserverKey {
        self.id
    }

    pub fn get_options(&self) -> &QueryOptions<V> {
        &self.options
    }

    pub fn notify(&self, state: QueryState<V>) {
        let listeners = self.listeners.try_borrow().expect("notify borrow");
        for listener in listeners.values() {
            listener(&state);
        }
    }

    pub fn add_listener(&self, listener: impl Fn(&QueryState<V>) + 'static) -> ListenerKey {
        let listener = Box::new(listener);
        let key = self
            .listeners
            .try_borrow_mut()
            .expect("add_listener borrow_mut")
            .insert(listener);
        key
    }

    pub fn remove_listener(&self, key: ListenerKey) -> bool {
        self.listeners
            .try_borrow_mut()
            .expect("remove_listener borrow_mut")
            .remove(key)
            .is_some()
    }

    pub fn update_query(&self, query: Query<K, V>) {
        if let Some(current_query) = self.query.take() {
            current_query.unsubscribe(self);
        } else {
            leptos::logging::debug_warn!(
                "QueryObserver::update_query: QueryObserver::query is None"
            );
        }

        query.subscribe(self);

        *self.query.borrow_mut() = Some(query);

        self.query
            .borrow()
            .as_ref()
            .expect("update_query borrow")
            .ensure_execute();
    }

    pub fn cleanup(&self) {
        if let Some(query) = self.query.take() {
            query.unsubscribe(self);
        } else {
            logging::debug_warn!("QueryObserver::cleanup: QueryObserver::query is None")
        }

        if !self
            .listeners
            .try_borrow()
            .expect("cleanup borrow")
            .is_empty()
        {
            leptos::logging::debug_warn!(
                "QueryObserver::cleanup: QueryObserver::listeners is not empty"
            );
        }
    }
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ObserverKey(u32);

fn next_id() -> ObserverKey {
    ObserverKey(NEXT_ID.fetch_add(1, Ordering::Relaxed))
}
