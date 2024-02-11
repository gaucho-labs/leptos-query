use leptos::*;

use crate::{Query, QueryObserverKind, QueryState};

/// Subscribing to cache events
pub trait CacheObserver {
    /// receive a cache event.
    fn process_cache_event(&self, event: CacheEvent);
}

/// The events that can be observed from the query cache.
#[derive(Debug, Clone)]
pub enum CacheEvent {
    /// A new query that has become active in the cache.
    Created(QueryCachePayload),
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
#[derive(Debug, Clone)]
pub struct QueryCachePayload {
    /// The key of the query.
    pub key: QueryCacheKey,
    /// The serialized state of the query.
    pub state: Signal<QueryState<String>>,
    /// The number of active observers for this query.
    pub observer_count: Signal<usize>,
}

/// A key for a query in the cache.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryCacheKey(pub String);

impl<K, V> From<Query<K, V>> for QueryCachePayload
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    fn from(query: Query<K, V>) -> Self {
        let state = {
            // TODO: How to unsub?
            let (state, _) = query.register_observer(QueryObserverKind::Passive);

            Signal::derive(move || {
                state.get().map_data(|data| {
                    leptos::Serializable::ser(data).expect("Serialize Query Value")
                })
            })
        };

        let observer_count = query.get_active_observer_count();

        let key: QueryCacheKey = (&query.key).into();

        QueryCachePayload {
            key,
            state,
            observer_count,
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
