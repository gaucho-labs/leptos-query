use leptos::*;
use std::{cell::Cell, future::Future, hash::Hash, rc::Rc, time::Duration};

use crate::{
    instant::{get_instant, Instant},
    query_state::QueryState,
    use_cache, use_query_client,
    util::{time_until_stale, use_timeout},
};

type Refetch = Rc<dyn Fn() -> ()>;

pub(crate) fn execute_query<K, V, Fu>(
    cx: Scope,
    state: Memo<QueryState<K, V>>,
    query: impl Fn(K) -> Fu + 'static,
    callback: impl Fn() -> () + 'static,
) -> Refetch
where
    K: Clone + Hash + Eq + PartialEq + 'static,
    V: Clone + 'static,
    Fu: Future<Output = V> + 'static,
{
    let query = Rc::new(query);
    let callback = Rc::new(callback);
    let refetch = move || {
        let query = query.clone();
        let callback = callback.clone();
        spawn_local(async move {
            let state = state.get_untracked();
            if !state.fetching.get_untracked() {
                state.fetching.set(true);

                let result = query(state.key.clone()).await;

                state.updated_at.set(Some(get_instant()));
                state.fetching.set(false);
                state.value.set(Some(result.clone()));
                if state.invalidated.get_untracked() {
                    state.invalidated.set(false);
                }
                callback()
            }
        })
    };

    let refetch: Refetch = Rc::new(refetch);
    ensure_not_stale(cx, state, refetch.clone());
    sync_refetch(cx, state, refetch.clone());
    sync_observers(cx, state);
    // TODO FIX
    ensure_cache_cleanup(cx, state, None);

    refetch
}

fn ensure_not_stale<K: Clone, V: Clone>(
    cx: Scope,
    state: Memo<QueryState<K, V>>,
    refetch: Refetch,
) {
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();
        let updated_at = state.updated_at;
        let stale_time = state.stale_time;

        // On mount, ensure that the resource is not stale
        match (updated_at.get_untracked(), stale_time.get_untracked()) {
            (Some(updated_at), Some(stale_time)) => {
                if time_until_stale(updated_at, stale_time).is_zero() {
                    refetch();
                }
            }
            _ => (),
        }
    })
}

fn sync_refetch<K, V>(cx: Scope, state: Memo<QueryState<K, V>>, refetch: Refetch)
where
    K: Clone + 'static,
    V: Clone + 'static,
{
    create_isomorphic_effect(cx, {
        let refetch = refetch.clone();
        move |_| {
            let refetch = refetch.clone();

            let state = state.get();
            let invalidated = state.invalidated;
            let refetch_interval = state.refetch_interval;
            let updated_at = state.updated_at;

            // Effect for refetching query on interval.
            use_timeout(cx, {
                let refetch = refetch.clone();
                move || match (updated_at.get(), refetch_interval.get()) {
                    (Some(updated_at), Some(refetch_interval)) => {
                        let refetch = refetch.clone();
                        let timeout = time_until_stale(updated_at, refetch_interval);
                        set_timeout_with_handle(
                            move || {
                                refetch();
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
                        refetch();
                    }
                }
            });
        }
    })
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

fn ensure_cache_cleanup<K, V>(
    cx: Scope,
    state: Memo<QueryState<K, V>>,
    cache_time: Option<Duration>,
) where
    K: Clone + Hash + Eq + PartialEq + 'static,
    V: Clone + 'static,
{
    let root_scope = use_query_client(cx).cx;
    create_isomorphic_effect(cx, move |_| {
        let state = state.get();
        let key = state.key.clone();
        let observers = state.observers.clone();
        cache_cleanup::<K, V>(
            root_scope,
            key,
            state.updated_at.into(),
            cache_time,
            observers,
        );
    });
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
