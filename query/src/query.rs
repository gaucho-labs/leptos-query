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
    garbage_collector: Rc<GarbageCollector<K, V>>,
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
        let gc_time = RwSignal::new(None);
        Query {
            key: key.clone(),
            current_request: Rc::new(Cell::new(None)),
            observers: Rc::new(RefCell::new(HashMap::new())),
            state: Rc::new(Cell::new(QueryState::Created)),
            garbage_collector: Rc::new(GarbageCollector::new(key, gc_time.clone().into())),
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

        if let Some(updated_at) = state.updated_at() {
            self.garbage_collector.new_update(updated_at);
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
        self.garbage_collector.disable_gc();
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
            self.garbage_collector.enable_gc();
        }
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
            .next()
            .map(|o| o.get_fetcher());

        if let Some(fetcher) = fetcher {
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

    pub(crate) fn is_stale(&self, stale_time: Option<Duration>) -> bool {
        let last_update = self.with_state(|state| state.updated_at());

        match (last_update, stale_time) {
            (Some(updated_at), Some(stale_time)) => {
                time_until_stale(updated_at, stale_time).is_zero()
            }

            _ => false,
        }
    }

    // Enables having different stale times & refetch intervals for the same query.
    // The lowest stale time & refetch interval will be used.
    // When the scope is dropped, the stale time & refetch interval will be reset to the previous value (if they existed).
    // Cache time behaves differently. It will only use the minimum cache time found.
    // pub(crate) fn update_options(&self, options: crate::QueryOptions<V>) {
    //     // Use the minimum cache time.
    //     match (self.gc_time.get_untracked(), options.gc_time) {
    //         (Some(current), Some(new)) if new < current => self.gc_time.set(Some(new)),
    //         (None, Some(new)) => self.gc_time.set(Some(new)),
    //         _ => (),
    //     }

    //     // Handle refetch interval.
    //     {
    //         let curr_refetch_interval = self.refetch_interval.get_untracked();
    //         let (prev_refetch, new_refetch) =
    //             match (curr_refetch_interval, options.refetch_interval) {
    //                 (Some(current), Some(new)) if new < current => (Some(current), Some(new)),
    //                 (None, Some(new)) => (None, Some(new)),
    //                 _ => (None, None),
    //             };

    //         if let Some(new_refetch) = new_refetch {
    //             self.refetch_interval.set(Some(new_refetch));
    //         }

    //         let refetch_interval = self.refetch_interval;
    //         on_cleanup(move || {
    //             if let Some(prev_refetch) = prev_refetch {
    //                 refetch_interval.set(Some(prev_refetch));
    //             }
    //         });
    //     }
    //     // Handle stale time.
    //     {
    //         let stale_time = ensure_valid_stale_time(&options.stale_time, &options.gc_time);
    //         let curr_stale_time = self.stale_time.get_untracked();
    //         let (prev_stale, new_stale) = match (curr_stale_time, stale_time) {
    //             (Some(current), Some(new)) if new < current => (Some(current), Some(new)),
    //             (None, Some(new)) => (None, Some(new)),
    //             _ => (None, None),
    //         };

    //         if let Some(stale_time) = new_stale {
    //             self.stale_time.set(Some(stale_time))
    //         }

    //         let stale_time = self.stale_time;
    //         on_cleanup(move || {
    //             if let Some(prev_stale) = prev_stale {
    //                 stale_time.set(Some(prev_stale));
    //             }
    //         })
    //     }
    // }
}

impl<K, V> Query<K, V>
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    pub(crate) fn dispose(&self) {
        // debug_assert!(
        //     {
        //         self.observers
        //             .borrow()
        //             .values()
        //             .map(|o| o.kind)
        //             .all(|k| matches!(k, QueryObserverKind::Passive))
        //     },
        //     "Query has active observers"
        // );
        // self.gc_time.dispose();
        // self.refetch_interval.dispose();
    }
}

async fn execute_with_cancellation<V, Fu>(
    fut: Fu,
    cancellation: oneshot::Receiver<()>,
) -> Result<V, ()>
where
    Fu: std::future::Future<Output = V> + Unpin,
{
    cfg_if::cfg_if! {
        if #[cfg(any(feature = "hydrate", feature = "csr"))] {
            use futures::future::Either;

            let result = futures::future::select(fut, cancellation).await;

            match result {
                Either::Left((result, _)) => Ok(result),
                Either::Right((cancelled ,_)) => {
                    if let Err(_) = cancelled {
                        logging::debug_warn!("Query cancellation was incorrectly dropped.");
                    }

                    Err(())
                },
            }
        // No cancellation on server side.
        } else {
            let _ = cancellation;
            let result = fut.await;
            Ok(result)
        }
    }
}

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
