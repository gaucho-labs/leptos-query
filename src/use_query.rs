use crate::instant::Instant;
use crate::query_result::QueryResult;
use crate::{time_until_stale, CacheEntry, QueryClient, QueryOptions, QueryState};
use leptos::leptos_dom::helpers::TimeoutHandle;
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

pub fn use_query<K, V, Fu>(
    cx: Scope,
    key: K,
    query: impl Fn(K) -> Fu + 'static,
    options: QueryOptions<V>,
) -> QueryResult<V>
where
    Fu: Future<Output = V> + 'static,
    K: Hash + Eq + PartialEq + Clone + 'static,
    V: Clone + Serializable + 'static,
{
    let cache_time = options.cache_time.clone();
    let state = use_cache(cx, {
        let key = key.clone();
        move |(root_scope, cache)| {
            let entry = cache.entry(key.clone());

            let state = match entry {
                Entry::Occupied(entry) => {
                    let entry = entry.into_mut();
                    entry.set_options(options);
                    entry
                }
                Entry::Vacant(entry) => {
                    let state = QueryState::new(root_scope, key.clone(), query, options);
                    entry.insert(state.clone())
                }
            };
            state.observers.set(state.observers.get() + 1);
            state.clone()
        }
    });

    // Keep track of the number of observers for this query.
    let observers = state.observers.clone();
    on_cleanup(cx, {
        let observers = observers.clone();
        move || {
            observers.set(observers.get() - 1);
        }
    });

    // Ensure that the Query is removed from cache up after the specified cache_time.
    let root_scope = use_query_client(cx).cx;
    cache_cleanup::<K, V>(
        root_scope,
        key,
        state.updated_at.into(),
        cache_time,
        observers,
    );

    let data = state.read(cx);
    let is_loading = state.is_loading(cx);
    let is_refetching = state.is_refetching(cx);
    let is_stale = state.is_stale(cx);
    let updated_at = state.updated_at.clone().into();
    let refetch = move |_: ()| state.refetch();

    QueryResult {
        data,
        is_loading,
        is_stale,
        is_refetching,
        updated_at,
        refetch: refetch.mapped_signal_setter(cx),
    }
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

    create_effect(cx, {
        let interval = interval.clone();

        move |maybe_handle: Option<Option<TimeoutHandle>>| {
            if let Some(handle) = maybe_handle.flatten() {
                handle.clear();
            };

            let key = key.clone();
            let observers = observers.clone();
            match (last_updated.get(), cache_time) {
                (Some(last_updated), Some(cache_time)) => {
                    let timeout = time_until_stale(last_updated, cache_time);
                    let handle = set_timeout_with_handle(
                        move || {
                            let removed = use_cache::<K, V, Option<QueryState<K, V>>>(
                                cx,
                                move |(_, cache)| cache.remove(&key),
                            );
                            if let Some(query) = removed {
                                if observers.get() == 0 {
                                    let QueryState {
                                        resource,
                                        stale_time,
                                        refetch_interval,
                                        updated_at: last_updated,
                                        invalidated,
                                        ..
                                    } = query;
                                    // TODO: Dispose resource.
                                    resource.dispose();
                                    stale_time.dispose();
                                    refetch_interval.dispose();
                                    last_updated.dispose();
                                    invalidated.dispose();

                                    drop(query)
                                }
                            };
                        },
                        timeout,
                    )
                    .ok();
                    interval.set(handle);
                    handle
                }
                _ => None,
            }
        }
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
