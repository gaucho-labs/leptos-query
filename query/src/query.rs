use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Duration,
};

use futures_channel::oneshot;
use leptos::*;

use crate::{
    garbage_collector::GarbageCollector,
    query_cache::CacheNotification,
    query_executor::with_query_supressed,
    query_observer::{ObserverKey, QueryObserver},
    use_query_client,
    util::time_until_stale,
    QueryData, QueryState,
};

#[derive(Clone)]
pub(crate) struct Query<K, V> {
    pub(crate) key: K,

    // Cancellation
    current_request: Rc<Cell<Option<oneshot::Sender<()>>>>,

    // State
    state: Rc<Cell<QueryState<V>>>,

    // Synchronization
    observers: Rc<RefCell<HashMap<ObserverKey, QueryObserver<K, V>>>>,
    garbage_collector: Rc<Cell<Option<GarbageCollector<K, V>>>>,
}

impl<K: PartialEq, V> PartialEq for Query<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<K: PartialEq, V> Eq for Query<K, V> {}

impl<K, V> Query<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub(crate) fn new(key: K) -> Self {
        Query {
            key: key.clone(),
            current_request: Rc::new(Cell::new(None)),
            observers: Rc::new(RefCell::new(HashMap::new())),
            state: Rc::new(Cell::new(QueryState::Created)),
            garbage_collector: Rc::new(Cell::new(None)),
        }
    }
}

impl<K, V> Query<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub(crate) fn set_state(&self, state: QueryState<V>) {
        // Notify observers.
        let observers = self.observers.try_borrow().expect("set state borrow");
        for observer in observers.values() {
            observer.notify(state.clone())
        }

        let invalid = matches!(state, QueryState::Invalid(_));

        self.state.set(state);

        // Notify cache. This has to be at the end due to sending the entire query in the notif.
        use_query_client()
            .cache
            .notify(CacheNotification::UpdatedState(self.clone()));

        if invalid {
            self.execute();
        }
    }

    pub(crate) fn update_state(&self, update_fn: impl FnOnce(&mut QueryState<V>)) {
        let mut state = self.state.take();
        update_fn(&mut state);
        self.set_state(state);
    }

    /// Be careful with this function. Used to avoid cloning.
    /// If update returns Ok(_) the state will be updated and subscribers will be notified.
    /// If update returns Err(_) the state will not be updated and subscribers will not be notified.
    /// Err(_) should always contain the previous state.
    pub(crate) fn maybe_map_state(
        &self,
        update_fn: impl FnOnce(QueryState<V>) -> Result<QueryState<V>, QueryState<V>>,
    ) -> bool {
        let current_state = self.state.take();

        match update_fn(current_state) {
            Ok(new_state) => {
                self.set_state(new_state);
                true
            }
            Err(old_state) => {
                self.state.set(old_state);
                false
            }
        }
    }

    /// Marks the resource as invalid, which will cause it to be refetched on next read.
    pub(crate) fn mark_invalid(&self) -> bool {
        let mut updated = false;
        self.maybe_map_state(|state| {
            if let QueryState::Loaded(data) = state {
                updated = true;
                Ok(QueryState::Invalid(data))
            } else {
                Err(state)
            }
        });
        updated
    }

    pub fn subscribe(&self, observer: &QueryObserver<K, V>) {
        let observer_id = observer.get_id();
        let mut observers = self
            .observers
            .try_borrow_mut()
            .expect("subscribe borrow_mut");
        observers.insert(observer_id, observer.clone());

        // Disable GC.
        self.disable_gc();
        self.update_gc_time(observer.get_options().gc_time);

        // Notify cache.
        use_query_client()
            .cache
            .notify::<K, V>(CacheNotification::NewObserver(self.key.clone()))
    }

    pub fn unsubscribe(&self, observer: &QueryObserver<K, V>) {
        let mut observers = self
            .observers
            .try_borrow_mut()
            .expect("unsubscribe borrow_mut");
        if let Some(_) = observers.remove(&observer.get_id()) {
            use_query_client()
                .cache
                .notify::<K, V>(CacheNotification::ObserverRemoved(self.key.clone()))
        }

        if observers.is_empty() {
            drop(observers);
            self.enable_gc();
        }
    }

    pub fn update_gc_time(&self, gc_time: Option<Duration>) {
        self.with_gc(|gc| gc.update_gc_time(gc_time));
    }

    pub fn enable_gc(&self) {
        self.with_gc(|gc| gc.enable_gc());
    }

    pub fn disable_gc(&self) {
        self.with_gc(|gc| gc.disable_gc());
    }

    pub fn with_gc(&self, func: impl FnOnce(&GarbageCollector<K, V>)) {
        let garbage_collector = self
            .garbage_collector
            .take()
            .unwrap_or_else(|| GarbageCollector::new(self.clone()));
        func(&garbage_collector);

        self.garbage_collector.set(Some(garbage_collector));
    }

    pub(crate) fn get_state(&self) -> QueryState<V> {
        let state = self.state.take();
        let state_clone = state.clone();
        self.state.set(state);
        state_clone
    }

    // Useful to avoid clones.
    pub(crate) fn with_state<T>(&self, func: impl FnOnce(&QueryState<V>) -> T) -> T {
        let state = self.state.take();
        let result = func(&state);
        self.state.set(state);
        result
    }

    /**
     * Execution and Cancellation.
     */

    pub(crate) fn execute(&self) {
        let query = self.clone();
        let fetcher = self
            .observers
            .try_borrow()
            .expect("execute borrow")
            .values()
            .find_map(|f| f.get_fetcher());

        if let Some(fetcher) = fetcher {
            with_query_supressed(move |supressed| {
                if !supressed {
                    leptos::spawn_local(async move {
                        match query.new_execution() {
                            None => {}
                            Some(cancellation) => {
                                match query.get_state() {
                                    // Already loading.
                                    QueryState::Loading | QueryState::Created => {
                                        query.set_state(QueryState::Loading);
                                        let fetch = std::pin::pin!(fetcher(query.key.clone()));
                                        match execute_with_cancellation(fetch, cancellation).await {
                                            Ok(data) => {
                                                let data = QueryData::now(data);
                                                query.set_state(QueryState::Loaded(data));
                                            }
                                            Err(_) => {
                                                logging::error!("Initial fetch was cancelled!");
                                                query.set_state(QueryState::Created);
                                            }
                                        }
                                    }
                                    // Subsequent loads.
                                    QueryState::Fetching(data)
                                    | QueryState::Loaded(data)
                                    | QueryState::Invalid(data) => {
                                        query.set_state(QueryState::Fetching(data));
                                        let fetch = std::pin::pin!(fetcher(query.key.clone()));
                                        match execute_with_cancellation(fetch, cancellation).await {
                                            Ok(data) => {
                                                let data = QueryData::now(data);
                                                query.set_state(QueryState::Loaded(data));
                                            }
                                            Err(_) => {
                                                query.maybe_map_state(|state| {
                                                    if let QueryState::Fetching(data) = state {
                                                        Ok(QueryState::Loaded(data))
                                                    } else {
                                                        Err(state)
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                                query.finalize_execution()
                            }
                        }
                    });
                }
            });
        }
    }

    // Only scenario where two requests can exist at the same time is the first is cancelled.
    pub(crate) fn new_execution(&self) -> Option<oneshot::Receiver<()>> {
        let current_request = self.current_request.take();
        if current_request.is_none() {
            let (sender, receiver) = oneshot::channel();
            self.current_request.set(Some(sender));
            Some(receiver)
        } else {
            self.current_request.set(current_request);
            None
        }
    }

    pub(crate) fn finalize_execution(&self) {
        self.current_request.set(None);
    }

    pub(crate) fn cancel(&self) -> bool {
        if let Some(current_request) = self.current_request.take() {
            let cancellation = current_request.send(());
            if cancellation.is_err() {
                logging::error!("Failed to cancel request {:?}", self.key);
            }
            cancellation.is_ok()
        } else {
            false
        }
    }

    pub(crate) fn needs_execute(&self) -> bool {
        // TODO: How to handle SSR Hydration case?
        self.with_state(|s| matches!(s, QueryState::Created))
            || self.with_state(|s| matches!(s, QueryState::Invalid(_)))
            || self.is_stale()
    }

    pub(crate) fn ensure_execute(&self) {
        if self.needs_execute() {
            self.execute();
        }
    }

    pub(crate) fn is_stale(&self) -> bool {
        let stale_time = self
            .observers
            .borrow()
            .iter()
            .flat_map(|(_, o)| o.get_options().stale_time)
            .min();
        let updated_at = self.with_state(|s| s.updated_at());

        match (updated_at, stale_time) {
            (Some(updated_at), Some(stale_time)) => {
                time_until_stale(updated_at, stale_time).is_zero()
            }
            _ => false,
        }
    }

    pub(crate) fn get_updated_at(&self) -> Option<crate::Instant> {
        self.with_state(|s| s.updated_at())
    }

    pub(crate) fn get_key(&self) -> &K {
        &self.key
    }
}

impl<K, V> Query<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub(crate) fn dispose(&self) {
        debug_assert!(
            { self.observers.borrow().is_empty() },
            "Query has active observers"
        );
    }
}

#[cfg(any(feature = "hydrate", feature = "csr"))]
async fn execute_with_cancellation<V, Fu>(
    fut: Fu,
    cancellation: oneshot::Receiver<()>,
) -> Result<V, ()>
where
    Fu: std::future::Future<Output = V> + Unpin,
{
    use futures::future::Either;

    let result = futures::future::select(fut, cancellation).await;

    match result {
        Either::Left((result, _)) => Ok(result),
        Either::Right((cancelled, _)) => {
            if let Err(_) = cancelled {
                logging::debug_warn!("Query cancellation was incorrectly dropped.");
            }

            Err(())
        }
    }
}

// No cancellation on server side.
#[cfg(not(any(feature = "hydrate", feature = "csr")))]
async fn execute_with_cancellation<V, Fu>(
    fut: Fu,
    cancellation: oneshot::Receiver<()>,
) -> Result<V, ()>
where
    Fu: std::future::Future<Output = V> + Unpin,
{
    let _ = cancellation;
    let result = fut.await;
    Ok(result)
}

// TODO: USE THIS?
#[allow(unused)]
pub(crate) fn ensure_valid_stale_time(
    stale_time: &Option<Duration>,
    gc_time: &Option<Duration>,
) -> Option<Duration> {
    match (stale_time, gc_time) {
        (Some(ref stale_time), Some(ref gc_time)) => {
            if stale_time > gc_time {
                logging::debug_warn!(
                    "Stale time is greater than gc time. Using gc time instead. Stale: {}, GC: {}",
                    stale_time.as_millis(),
                    gc_time.as_millis()
                );
                Some(*gc_time)
            } else {
                Some(*stale_time)
            }
        }
        (stale_time, _) => *stale_time,
    }
}
