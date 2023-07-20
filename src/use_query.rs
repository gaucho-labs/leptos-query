use crate::instant::{get_instant, Instant};
use crate::query_result::QueryResult;
use crate::util::{time_until_stale, use_timeout};
use crate::{CacheEntry, QueryClient, QueryOptions, QueryState, ResourceOption};
use leptos::*;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

/// Provides a Query Client to the current scope.
pub fn provide_query_client(cx: Scope) {
    provide_context(cx, QueryClient::new(cx));
}

/// Retrieves a Query Client from the current scope.
pub fn use_query_client(cx: Scope) -> QueryClient {
    use_context::<QueryClient>(cx).expect("Query Client Missing.")
}

/// Creates a query. Useful for data fetching, caching, and synchronization with server state.
///
/// A Query provides:
/// - caching
/// - de-duplication
/// - invalidation
/// - background refetching
/// - refetch intervals
/// - memory management with cache lifetimes
///
///
/// Details:
/// - A query is unique per Key `K`.
/// - A query Key type `K` must only correspond to ONE UNIQUE Value `V` Type.
/// - Meaning a query Key type `K` cannot correspond to multiple Value `V` Types.
///
/// Example
/// ```
///
/// // Create a Newtype for MonkeyId.
/// #[derive(Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
/// struct MonkeyId(String);
///
///
/// // Monkey fetcher.
/// async fn get_monkey(id: MonkeyId) -> Monkey {
/// ...
/// }
///
/// // Query for a Monkey.
/// fn use_monkey_query(cx: Scope, id: MonkeyId) -> QueryResult<Monkey> {
///     leptos_query::use_query(
///         cx,
///         id,
///         get_monkey,
///         QueryOptions {
///             default_value: None,
///             refetch_interval: None,
///             resource_option: ResourceOption::NonBlocking,
///             stale_time: Some(Duration::from_secs(5)),
///             cache_time: Some(Duration::from_secs(30)),
///         },
///     )
/// }
///
/// ```
///
pub fn use_query<K, V, Fu>(
    cx: Scope,
    key: impl Fn() -> K + 'static,
    query: impl Fn(K) -> Fu + 'static,
    options: QueryOptions<V>,
) -> QueryResult<V>
where
    Fu: Future<Output = V> + 'static,
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone
        + Serializable
        + 'static
        + server_fn::serde::de::DeserializeOwned
        + server_fn::serde::Serialize,
{
    let key = create_memo(cx, move |_| key());

    // Find relevant state.
    let state = create_memo(cx, {
        let options = options.clone();
        move |_| {
            use_cache(cx, {
                let options = options.clone();
                let key = key.get();
                move |(root_scope, cache)| {
                    let entry = cache.entry(key.clone());

                    let state = match entry {
                        Entry::Occupied(entry) => {
                            let entry = entry.into_mut();
                            // Enable nested options.
                            entry.set_options(cx, options);
                            entry
                        }
                        Entry::Vacant(entry) => {
                            let state = QueryState::new(root_scope, key, options);
                            entry.insert(state.clone())
                        }
                    };
                    state.clone()
                }
            })
        }
    });

    let query = Rc::new(query);
    let fetcher = move |key: K| {
        let query = query.clone();
        async move {
            let state = state.get_untracked();
            if state.is_loading_untracked() {
                // Suspend indefinitely and wait for interruption.
                gloo_timers::future::sleep(LONG_TIME).await;
                state.value.get_untracked()
            // Ensure no request in flight.
            } else if !(state.fetching.get_untracked()) {
                state.fetching.set(true);
                let result = query(key).await;
                state.updated_at.set(Some(get_instant()));
                state.fetching.set(false);
                state.value.set(Some(result.clone()));
                if state.invalidated.get_untracked() {
                    state.invalidated.set(false);
                }
                return Some(result);
            } else {
                state.value.get_untracked()
            }
        }
    };

    let resource: Resource<K, Option<V>> = {
        match options.resource_option {
            ResourceOption::NonBlocking => create_resource(cx, move || key.get(), fetcher),
            ResourceOption::Blocking => create_blocking_resource(cx, move || key.get(), fetcher),
        }
    };

    // Listen for changes to the same key.
    create_isomorphic_effect(cx, move |prev_key: Option<K>| {
        let state = state.get();
        let data = state.value.get();

        if let Some(prev_key) = prev_key {
            if state.key == prev_key && data.is_some() {
                resource.set(data);
            }
        } else if data.is_some() {
            resource.set(data);
        }
        state.key.clone()
    });

    // TODO: If key changes, and new data isn't loaded, then loading should appear again?
    // When key changes.
    create_isomorphic_effect(cx, move |prev_key: Option<K>| {
        let state = state.get();
        let data = state.value.get();

        if let Some(prev_key) = prev_key {
            if state.key != prev_key && data.is_some() {
                resource.set(data);
            }
        }
        state.key.clone()
    });

    ensure_not_stale(cx, state.clone(), resource.clone());
    sync_refetch(cx, state.clone(), resource.clone());
    sync_observers(cx, state.clone());

    // Ensure that the Query is removed from cache up after the specified cache_time.
    let root_scope = use_query_client(cx).cx;
    let cache_time = options.cache_time;
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();
        let observers = state.observers.clone();
        let key = key.get();
        cache_cleanup::<K, V>(
            root_scope,
            key,
            state.updated_at.into(),
            cache_time,
            observers,
        );
    });

    QueryResult::new(cx, state, resource)
}

const LONG_TIME: Duration = Duration::from_secs(60 * 60 * 24);

fn ensure_not_stale<K: Clone, V: Clone>(
    cx: Scope,
    state: Memo<QueryState<K, V>>,
    resource: Resource<K, Option<V>>,
) {
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();
        let updated_at = state.updated_at;
        let stale_time = state.stale_time;

        // On mount, ensure that the resource is not stale
        match (updated_at.get_untracked(), stale_time.get_untracked()) {
            (Some(updated_at), Some(stale_time)) => {
                if time_until_stale(updated_at, stale_time).is_zero() {
                    resource.refetch();
                }
            }
            _ => (),
        }
    })
}

// Effects for syncing on interval and invalidation.
fn sync_refetch<K, V>(cx: Scope, state: Memo<QueryState<K, V>>, resource: Resource<K, Option<V>>)
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();
        let invalidated = state.invalidated;
        let refetch_interval = state.refetch_interval;
        let updated_at = state.updated_at;

        // Effect for refetching query on interval.
        use_timeout(cx, move || {
            match (updated_at.get(), refetch_interval.get()) {
                (Some(updated_at), Some(refetch_interval)) => {
                    let timeout = time_until_stale(updated_at, refetch_interval);
                    set_timeout_with_handle(
                        move || {
                            resource.refetch();
                        },
                        timeout,
                    )
                    .ok()
                }
                _ => None,
            }
        });

        // Refetch query if invalidated.
        create_isomorphic_effect(cx, {
            move |_| {
                if invalidated.get() {
                    resource.refetch();
                }
            }
        });
    })
}

// Will cleanup the cache corresponding to the key when the cache_time has elapsed, and the query has not been updated.
fn cache_cleanup<K, V>(
    cx: Scope,
    key: K,
    last_updated: Signal<Option<Instant>>,
    cache_time: Option<Duration>,
    observers: Rc<Cell<usize>>,
) where
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: 'static,
{
    use_timeout(cx, move || match (last_updated.get(), cache_time) {
        (Some(last_updated), Some(cache_time)) => {
            let timeout = time_until_stale(last_updated, cache_time);
            let key = key.clone();
            let observers = observers.clone();
            set_timeout_with_handle(
                move || {
                    let removed =
                        use_cache::<K, V, Option<QueryState<K, V>>>(cx, move |(_, cache)| {
                            cache.remove(&key)
                        });
                    if let Some(query) = removed {
                        if observers.get() == 0 {
                            query.dispose();
                            drop(query)
                        }
                    };
                },
                timeout,
            )
            .ok()
        }
        _ => None,
    });
}

// Ensure that observers are kept track of.
fn sync_observers<K: Clone, V: Clone>(cx: Scope, state: Memo<QueryState<K, V>>) {
    type Observer = Rc<Cell<usize>>;
    let last_observer: Rc<Cell<Option<Observer>>> = Rc::new(Cell::new(None));

    on_cleanup(cx, {
        let last_observer = last_observer.clone();
        move || {
            if let Some(observer) = last_observer.take() {
                observer.set(observer.get() - 1);
            }
        }
    });

    // Ensure that observers are kept track of.
    create_isomorphic_effect(cx, move |observers: Option<Rc<Cell<usize>>>| {
        if let Some(observers) = observers {
            last_observer.set(None);
            observers.set(observers.get() - 1);
        }
        let observers = state.get().observers;
        last_observer.set(Some(observers.clone()));
        observers.set(observers.get() + 1);
        observers
    });
}

fn use_cache<K, V, R>(
    cx: Scope,
    func: impl FnOnce((Scope, &mut HashMap<K, QueryState<K, V>>)) -> R + 'static,
) -> R
where
    K: 'static,
    V: 'static,
{
    let client = use_query_client(cx);
    let mut cache = client.cache.borrow_mut();
    let entry = cache.entry(TypeId::of::<K>());

    let cache = entry.or_insert_with(|| {
        let wrapped: CacheEntry<K, V> = Rc::new(RefCell::new(HashMap::new()));
        let boxed = Box::new(wrapped) as Box<dyn Any>;
        boxed
    });

    let mut cache = cache
        .downcast_ref::<CacheEntry<K, V>>()
        .expect("Query Cache Type Mismatch.")
        .borrow_mut();

    func((client.cx, &mut cache))
}
