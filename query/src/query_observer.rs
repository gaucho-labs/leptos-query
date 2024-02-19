use std::cell::{Cell, RefCell};
use std::future::Future;
use std::{pin::Pin, rc::Rc};

use leptos::leptos_dom::helpers::IntervalHandle;
use slotmap::{new_key_type, SlotMap};

use crate::query::Query;
use crate::{QueryKey, QueryOptions, QueryState, QueryValue};

#[derive(Clone)]
pub struct QueryObserver<K, V> {
    id: ObserverKey,
    query: Rc<RefCell<Option<Query<K, V>>>>,
    fetcher: Option<Fetcher<K, V>>,
    refetch: Rc<Cell<Option<IntervalHandle>>>,
    options: QueryOptions<V>,
    listeners: Rc<RefCell<SlotMap<ListenerKey, Box<dyn Fn(&QueryState<V>)>>>>,
}

type Fetcher<K, V> = Rc<dyn Fn(K) -> Pin<Box<dyn Future<Output = V>>>>;

new_key_type! {
    pub struct ListenerKey;
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

        #[cfg(any(feature = "csr", feature = "hydrate"))]
        let refetch = {
            use leptos::logging;

            let interval = {
                if let Some(refetch_interval) = options.refetch_interval {
                    let query = query.clone();
                    let timeout = leptos::set_interval_with_handle(
                        move || {
                            if let Ok(query) = query.try_borrow() {
                                if let Some(query) = query.as_ref() {
                                    query.execute()
                                }
                            } else {
                                logging::debug_warn!("QueryObserver: Query is already borrowed")
                            }
                        },
                        refetch_interval,
                    )
                    .ok();
                    if timeout.is_none() {
                        logging::debug_warn!("QueryObserver: Failed to set refetch interval")
                    }
                    timeout
                } else {
                    None
                }
            };
            Rc::new(Cell::new(interval))
        };
        #[cfg(not(any(feature = "csr", feature = "hydrate")))]
        let refetch = Rc::new(Cell::new(None));

        Self {
            id,
            query,
            fetcher,
            refetch,
            options,
            listeners: Rc::new(RefCell::new(SlotMap::with_key())),
        }
    }

    pub fn no_fetcher(options: QueryOptions<V>, query: Option<Query<K, V>>) -> Self {
        let query = Rc::new(RefCell::new(query));
        let id = next_id();

        Self {
            id,
            query,
            fetcher: None,
            refetch: Rc::new(Cell::new(None)),
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

    pub fn update_query(&self, query: Option<Query<K, V>>) {
        if let Some(current_query) = self.query.take() {
            current_query.unsubscribe(self);
        }

        if let Some(query) = query {
            query.subscribe(self);
            *self.query.borrow_mut() = Some(query);
            self.query
                .borrow()
                .as_ref()
                .expect("update_query borrow")
                .ensure_execute();
        } else {
            *self.query.borrow_mut() = None;
        }
    }

    pub fn cleanup(&self) {
        if let Some(query) = self.query.take() {
            query.unsubscribe(self);
        }

        if let Some(interval) = self.refetch.take() {
            interval.clear();
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

thread_local! {
    static NEXT_ID: Cell<u32> = Cell::new(1);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObserverKey(u32);

fn next_id() -> ObserverKey {
    NEXT_ID.with(|id| {
        let current_id = id.get();
        id.set(current_id + 1);
        ObserverKey(current_id)
    })
}
