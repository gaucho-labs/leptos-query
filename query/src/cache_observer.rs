use std::rc::Rc;

use leptos::*;

use crate::{
    util::{time_until_stale, use_timeout},
    Query, QueryObserverKind, QueryState,
};

/// Subscribing to cache events
pub trait CacheObserver {
    /// receive a cache event.
    fn process_cache_event(&self, event: CacheEvent);
}

/// The events that can be observed from the query cache.
#[derive(Clone)]
pub enum CacheEvent {
    /// A new query that has become active in the cache.
    Created(CreatedQuery),
    /// A query that has been removed from the cache.
    Removed(QueryCacheKey),
}

impl CacheEvent {
    pub(crate) fn created<K, V>(query: Query<K, V>) -> Self
    where
        K: crate::QueryKey + 'static,
        V: crate::QueryValue + 'static,
    {
        let payload = query.into();
        CacheEvent::Created(payload)
    }

    pub(crate) fn removed<K>(key: &K) -> Self
    where
        K: crate::QueryKey + 'static,
    {
        CacheEvent::Removed(key.into())
    }
}

/// A new query that has become active in the cache.
#[derive(Clone)]
pub struct CreatedQuery {
    /// The key of the query.
    pub key: QueryCacheKey,
    /// The serialized state of the query.
    pub state: Signal<QueryState<String>>,
    /// Whether the query is currently considered stale.
    pub is_stale: Signal<bool>,
    /// The number of active observers for this query.
    pub observer_count: Signal<usize>,

    /// Mark invalid
    pub mark_invalid: Rc<dyn Fn() -> bool>,
}

/// A serialized key for a query in the cache.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryCacheKey(pub String);

impl<K, V> From<Query<K, V>> for CreatedQuery
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    fn from(query: Query<K, V>) -> Self {
        let state = {
            let (state, unsub) = query.register_observer(QueryObserverKind::Passive);

            // TODO: is this sufficient?
            on_cleanup(move || unsub());

            Signal::derive(move || {
                state.get().map_data(|data| {
                    leptos::Serializable::ser(data).expect("Serialize Query Value")
                })
            })
        };

        let is_stale = {
            let (stale, set_stale) = create_signal(false);

            let updated_at = Signal::derive(move || state.with(|s| s.updated_at()));
            let stale_time = query.get_stale_time();

            let _ = use_timeout(move || match (updated_at.get(), stale_time.get()) {
                (Some(updated_at), Some(stale_time)) => {
                    let duration = time_until_stale(updated_at, stale_time);
                    if duration.is_zero() {
                        set_stale.set(true);
                        None
                    } else {
                        set_stale.set(false);
                        set_timeout_with_handle(
                            move || {
                                set_stale.set(true);
                            },
                            duration,
                        )
                        .ok()
                    }
                }
                _ => None,
            });

            stale.into()
        };

        let mark_invalid = {
            let query = query.clone();
            Rc::new(move || query.mark_invalid())
        };

        let observer_count = query.get_active_observer_count();

        let key: QueryCacheKey = (&query.key).into();

        CreatedQuery {
            key,
            state,
            is_stale,
            observer_count,
            mark_invalid,
        }
    }
}

impl<K> From<&K> for QueryCacheKey
where
    K: crate::QueryKey + 'static,
{
    fn from(key: &K) -> Self {
        QueryCacheKey(format!("{:?}", key))
    }
}
