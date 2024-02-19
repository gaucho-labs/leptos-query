use crate::{CacheEvent, CacheObserver};
use async_trait::async_trait;

/// A utility for client side query persistance
#[async_trait]
pub trait QueryPersister {
    /// Persist a query to the persister
    async fn persist(&self, query: PersistedQuery);
    /// Remove a query from the persister
    async fn remove(&self, key: &str);
    /// Retrieve a query from the persister
    async fn retrieve(&self, key: &str) -> Option<PersistedQueryData>;
    /// Clear the persister
    async fn clear(&self);
}

impl<Persist> CacheObserver for Persist
where
    Persist: QueryPersister + Clone + 'static,
{
    fn process_cache_event(&self, event: CacheEvent) {
        match event {
            CacheEvent::Created(query) => {
                if let Ok(value) = query.state.try_into() {
                    let key = query.key.0;
                    let query = PersistedQuery { key, value };
                    let persister = self.clone();
                    leptos::spawn_local(async move {
                        persister.persist(query).await;
                    })
                }
            }
            CacheEvent::Updated(query) => {
                if let Ok(value) = query.state.try_into() {
                    let key = query.key.0;
                    let query = PersistedQuery { key, value };
                    let persister = self.clone();
                    leptos::spawn_local(async move {
                        persister.persist(query).await;
                    })
                }
            }
            CacheEvent::Removed(key) => {
                let _ = self.remove(&key.0);
            }
            _ => (),
        }
    }
}

#[derive(Clone)]

pub struct PersistedQuery {
    pub key: String,
    pub value: PersistedQueryData,
}

#[derive(Clone)]
#[cfg_attr(
    feature = "local_storage",
    derive(miniserde::Serialize, miniserde::Deserialize)
)]
pub struct PersistedQueryData {
    pub value: String,
    pub updated_at: u64,
}

impl<V> TryFrom<PersistedQueryData> for crate::QueryData<V>
where
    V: crate::QueryValue,
{
    type Error = leptos::SerializationError;

    fn try_from(value: PersistedQueryData) -> Result<Self, Self::Error> {
        let data = leptos::Serializable::de(value.value.as_str())?;
        let updated_at = crate::Instant(std::time::Duration::from_millis(value.updated_at));
        Ok(crate::QueryData { data, updated_at })
    }
}

impl From<crate::QueryData<String>> for PersistedQueryData {
    fn from(data: crate::QueryData<String>) -> Self {
        let value = data.data;
        let updated_at = data.updated_at.0.as_millis() as u64;
        PersistedQueryData { value, updated_at }
    }
}

impl TryFrom<crate::QueryState<String>> for PersistedQueryData {
    type Error = ();

    fn try_from(state: crate::QueryState<String>) -> Result<Self, Self::Error> {
        match state {
            // Only convert loaded state.
            crate::QueryState::Loaded(data) => Ok(data.into()),
            // Ignore other states.
            crate::QueryState::Loading
            | crate::QueryState::Created
            | crate::QueryState::Invalid(_)
            | crate::QueryState::Fetching(_) => Err(()),
        }
    }
}

impl<V> TryFrom<PersistedQueryData> for crate::QueryState<V>
where
    V: crate::QueryValue,
{
    type Error = leptos::SerializationError;

    fn try_from(data: PersistedQueryData) -> Result<Self, Self::Error> {
        let data = crate::QueryData::try_from(data)?;
        Ok(crate::QueryState::Loaded(data))
    }
}

#[cfg(feature = "local_storage")]
pub mod local_storage_persister {
    use super::*;
    use cfg_if::cfg_if;

    #[derive(Clone, Copy)]
    pub struct LocalStoragePersister;

    #[cfg(any(feature = "hydrate", feature = "csr"))]
    thread_local! {
        pub(crate) static LOCAL_STORAGE: Option<web_sys::Storage> = leptos::window().local_storage().ok().flatten()
    }

    #[cfg(any(feature = "hydrate", feature = "csr"))]
    fn local_storage() -> Option<web_sys::Storage> {
        LOCAL_STORAGE.with(Clone::clone)
    }

    #[async_trait]
    impl QueryPersister for LocalStoragePersister {
        async fn persist(&self, query: PersistedQuery) {
            cfg_if! {
                if #[cfg(any(feature = "hydrate", feature = "csr"))] {
                    let value = miniserde::json::to_string(&query.value);
                    let key = query.key;
                    if let Some(storage) = local_storage() {
                        let _ = storage.set(&key, &value);
                    }
                } else {
                    let _ = query;
                    ()
                }
            }
        }

        async fn remove(&self, key: &str) {
            cfg_if! {
                if #[cfg(any(feature = "hydrate", feature = "csr"))] {
                    if let Some(storage) = local_storage() {
                        let _ = storage.remove_item(key);
                    }
                } else {
                    let _ = key;
                    ()
                }
            }
        }

        async fn retrieve(&self, key: &str) -> Option<PersistedQueryData> {
            cfg_if! {
                if #[cfg(any(feature = "hydrate", feature = "csr"))] {
                    if let Some(storage) = local_storage() {
                        if let Some(value) = storage.get_item(key).ok().flatten() {
                            return miniserde::json::from_str(&value).ok()
                        }
                    }
                    None
                } else {
                    let _ = key;
                    None
                }
            }
        }

        async fn clear(&self) {
            cfg_if! {
                if #[cfg(any(feature = "hydrate", feature = "csr"))] {
                    if let Some(storage) = local_storage() {
                        let _ = storage.clear();
                    }
                } else {
                    ()
                }
            }
        }
    }
}
