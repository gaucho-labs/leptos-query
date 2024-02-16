use std::cell::{Cell, RefCell};
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
    query: Rc<Cell<Option<Query<K, V>>>>,
    fetcher: Rc<dyn Fn(K) -> Pin<Box<dyn Future<Output = V>>>>,
    options: QueryOptions<V>,
    listeners: Rc<RefCell<SlotMap<ListenerKey, Box<dyn Fn(&QueryState<V>)>>>>,
}

impl<K, V> std::fmt::Debug for QueryObserver<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryObserver")
            .field("id", &self.id)
            // .field("query", &self.query)
            .field("fetcher", &"...")
            // .field("options", &self.options)
            .field("listeners", &"...")
            .finish()
    }
}

new_key_type! {
    pub (crate) struct ListenerKey;
}

impl<K, V> QueryObserver<K, V>
where
    K: QueryKey + 'static,
    V: QueryValue + 'static,
{
    pub fn new<F, Fu>(fetcher: F, options: QueryOptions<V>, query: Query<K, V>) -> Self
    where
        F: Fn(K) -> Fu + 'static,
        Fu: Future<Output = V> + 'static,
    {
        let fetcher = Rc::new(move |s| Box::pin(fetcher(s)) as Pin<Box<dyn Future<Output = V>>>);
        let query = Rc::new(Cell::new(Some(query)));
        let id = next_id();

        Self {
            id,
            query,
            fetcher,
            options,
            listeners: Rc::new(RefCell::new(SlotMap::with_key())),
        }
    }

    pub fn get_fetcher(&self) -> Rc<dyn Fn(K) -> Pin<Box<dyn Future<Output = V>>>> {
        self.fetcher.clone()
    }

    pub fn get_id(&self) -> ObserverKey {
        self.id
    }

    pub fn notify(&self, state: QueryState<V>) {
        let listeners = self.listeners.try_borrow().expect("notify borrow");
        for listener in listeners.values() {
            listener(&state);
        }

        // If the query is invalid, execute it again.
        // self.with_query(|query| {
        //     if query.with_state(|state| matches!(state, QueryState::Invalid(_))) {
        //         query.execute();
        //     }
        // });
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

        self.query.set(Some(query));

        self.with_query(|q| {
            if q.is_stale(self.options.stale_time) {
                q.execute()
            }
        });

        self.with_query(|q| {
            if q.with_state(|state| matches!(state, QueryState::Created)) {
                q.execute()
            }
        });
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

    pub fn with_query<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Query<K, V>) -> R,
    {
        let query = self.query.take().expect("Query To Exist");
        let result = f(&query);
        self.query.set(Some(query));
        result
    }

    fn get_query(&self) -> Query<K, V> {
        let query = self.query.take().expect("Query To Exist");
        let cloned = query.clone();
        self.query.set(Some(query));
        cloned
    }
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ObserverKey(u32);

fn next_id() -> ObserverKey {
    ObserverKey(NEXT_ID.fetch_add(1, Ordering::Relaxed))
}
