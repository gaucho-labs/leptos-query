use std::{fmt::Debug, rc::Rc};

use crate::{query::Query, QueryState};

/// Subscribing to cache events
pub trait CacheObserver {
    /// receive a cache event.
    fn process_cache_event(&self, event: CacheEvent);
}

/// The events that can be observed from the query cache.
#[derive(Clone, Debug)]
pub enum CacheEvent {
    /// A new query that has become active in the cache.
    Created(CreatedQuery),
    /// A query that has been updated in the cache.
    Updated(SerializedQuery),
    /// A query that has been removed from the cache.
    Removed(QueryCacheKey),
    /// A new observer has been added to the query.
    ObserverAdded(ObserverAdded),
    /// A observer has been removed from the query.
    ObserverRemoved(QueryCacheKey),
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

    pub(crate) fn updated<K, V>(query: Query<K, V>) -> Self
    where
        K: crate::QueryKey + 'static,
        V: crate::QueryValue + 'static,
    {
        let payload = query.into();
        CacheEvent::Updated(payload)
    }

    pub(crate) fn removed<K>(key: &K) -> Self
    where
        K: crate::QueryKey + 'static,
    {
        CacheEvent::Removed(key.into())
    }

    pub(crate) fn observer_added<K, V>(key: &K, options: crate::QueryOptions<V>) -> Self
    where
        K: crate::QueryKey + 'static,
        V: crate::QueryValue + 'static,
    {
        let options =
            options.map_value(|v| leptos::Serializable::ser(&v).expect("Serialize Query Options"));
        CacheEvent::ObserverAdded(ObserverAdded {
            key: key.into(),
            options,
        })
    }

    pub(crate) fn observer_removed<K>(key: &K) -> Self
    where
        K: crate::QueryKey + 'static,
    {
        CacheEvent::ObserverRemoved(key.into())
    }
}

/// A new query that has become active in the cache.
#[derive(Clone)]
pub struct CreatedQuery {
    /// Serialized query key.
    pub key: QueryCacheKey,
    /// Serialized query state.
    pub state: QueryState<String>,
    /// Mark invalid
    pub mark_invalid: Rc<dyn Fn() -> bool>,
}

impl Debug for CreatedQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreatedQuery")
            .field("key", &self.key)
            .field("state", &self.state)
            .finish()
    }
}

/// A query that has been updated in the cache.
#[derive(Clone, Debug)]
pub struct SerializedQuery {
    /// The key of the query.
    pub key: QueryCacheKey,
    /// The serialized state of the query.
    pub state: QueryState<String>,
}

/// A serialized key for a query in the cache.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryCacheKey(pub String);

/// A new observer has been added to the query.
#[derive(Clone, Debug)]
pub struct ObserverAdded {
    /// The key of the query.
    pub key: QueryCacheKey,
    /// The observers options.
    pub options: crate::QueryOptions<String>,
}

impl<K, V> From<Query<K, V>> for CreatedQuery
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    fn from(query: Query<K, V>) -> Self {
        let key: QueryCacheKey = query.get_key().into();
        let state = query.with_state(|state| {
            state.map_data(|data| leptos::Serializable::ser(data).expect("Serialize Query State"))
        });

        let mark_invalid = Rc::new(move || query.mark_invalid());

        CreatedQuery {
            key,
            state,
            mark_invalid,
        }
    }
}

impl<K, V> From<Query<K, V>> for SerializedQuery
where
    K: crate::QueryKey + 'static,
    V: crate::QueryValue + 'static,
{
    fn from(query: Query<K, V>) -> Self {
        let key: QueryCacheKey = query.get_key().into();
        let state = query.with_state(|state| {
            state.map_data(|data| leptos::Serializable::ser(data).expect("Serialize Query State"))
        });

        SerializedQuery { key, state }
    }
}

impl<K> From<&K> for QueryCacheKey
where
    K: crate::QueryKey + 'static,
{
    fn from(key: &K) -> Self {
        QueryCacheKey(make_cache_key(key))
    }
}

pub(crate) fn make_cache_key<K>(key: &K) -> String
where
    K: crate::QueryKey + 'static,
{
    format!("{key:?}")
}
