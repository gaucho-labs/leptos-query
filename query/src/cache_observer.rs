use leptos::*;

use crate::{Query, QueryObserverKind, QueryState};

/// Subscribing to cache events
pub trait CacheObserver {
    /// receive a cache event.
    fn process_cache_event(&self, event: CacheEvent);
}

#[derive(Debug, Clone)]
pub enum CacheEvent {
    Created(QueryCachePayload),
    Removed(QueryCacheKey),
}

impl CacheEvent {
    pub(crate) fn updated<K, V>(query: Query<K, V>) -> Self
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

#[derive(Debug, Clone)]
pub struct QueryCachePayload {
    pub key: QueryCacheKey,
    pub state: Signal<QueryState<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryCacheKey(String);

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

        let key: QueryCacheKey = (&query.key).into();

        QueryCachePayload { key, state }
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
