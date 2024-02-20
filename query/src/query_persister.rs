use async_trait::async_trait;

use crate::cache_observer::{CacheEvent, CacheObserver};

/// A utility for client side query persistance
#[async_trait]
pub trait QueryPersister {
    /// Persist a query to the persister
    async fn persist(&self, key: &str, query: PersistQueryData);
    /// Remove a query from the persister
    async fn remove(&self, key: &str);
    /// Retrieve a query from the persister
    async fn retrieve(&self, key: &str) -> Option<PersistQueryData>;
    /// Clear the persister
    async fn clear(&self);
}

impl<Persist> CacheObserver for Persist
where
    Persist: QueryPersister + Clone + 'static,
{
    fn process_cache_event(&self, event: CacheEvent) {
        match event {
            #[cfg(any(feature = "hydrate", feature = "csr"))]
            CacheEvent::Created(query) => {
                if let Ok(value) = TryInto::<PersistQueryData>::try_into(query.state) {
                    let key = query.key.0;
                    let persister = self.clone();
                    leptos::spawn_local(async move {
                        persister.persist(&key, value).await;
                    })
                }
            }
            #[cfg(any(feature = "hydrate", feature = "csr"))]
            CacheEvent::Updated(query) => {
                if let Ok(value) = TryInto::<PersistQueryData>::try_into(query.state) {
                    let key = query.key.0;
                    let persister = self.clone();
                    leptos::spawn_local(async move {
                        persister.persist(&key, value).await;
                    })
                }
            }
            #[cfg(any(feature = "hydrate", feature = "csr"))]
            CacheEvent::Removed(key) => {
                let persister = self.clone();
                leptos::spawn_local(async move {
                    let _ = persister.remove(&key.0).await;
                })
            }
            _ => (),
        }
    }
}

/// Serialized query data.
#[derive(Clone)]
#[cfg_attr(
    feature = "local_storage",
    derive(miniserde::Serialize, miniserde::Deserialize)
)]
pub struct PersistQueryData {
    /// The serialized query data.
    pub value: String,
    /// The time the query was last updated in millis.
    pub updated_at: u64,
}

impl<V> TryFrom<PersistQueryData> for crate::QueryData<V>
where
    V: crate::QueryValue,
{
    type Error = leptos::SerializationError;

    fn try_from(value: PersistQueryData) -> Result<Self, Self::Error> {
        let data = leptos::Serializable::de(value.value.as_str())?;
        let updated_at = crate::Instant(std::time::Duration::from_millis(value.updated_at));
        Ok(crate::QueryData { data, updated_at })
    }
}

impl TryFrom<crate::QueryState<String>> for PersistQueryData {
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

impl From<crate::QueryData<String>> for PersistQueryData {
    fn from(data: crate::QueryData<String>) -> Self {
        let value = data.data;
        let updated_at = data.updated_at.0.as_millis() as u64;
        PersistQueryData { value, updated_at }
    }
}

#[cfg(feature = "local_storage")]
pub use local_storage_persister::LocalStoragePersister;

/// A persister that uses local storage to persist queries.
#[cfg(feature = "local_storage")]
pub mod local_storage_persister {
    use super::*;
    use cfg_if::cfg_if;

    /// A persister that uses local storage to persist queries.
    #[derive(Clone, Copy)]
    pub struct LocalStoragePersister;

    #[cfg(any(feature = "hydrate", feature = "csr"))]
    thread_local! {
        #[cfg(any(feature = "hydrate", feature = "csr"))]
        pub(crate) static LOCAL_STORAGE: Option<web_sys::Storage> = leptos::window().local_storage().ok().flatten()
    }

    #[cfg(any(feature = "hydrate", feature = "csr"))]
    fn local_storage() -> Option<web_sys::Storage> {
        LOCAL_STORAGE.with(Clone::clone)
    }

    #[async_trait]
    impl QueryPersister for LocalStoragePersister {
        async fn persist(&self, key: &str, query: PersistQueryData) {
            cfg_if! {
                if #[cfg(any(feature = "hydrate", feature = "csr"))] {
                    if let Some(storage) = local_storage() {
                        let value = miniserde::json::to_string(&query);
                        let _ = storage.set(&key, &value);
                    }
                } else {
                    let _ = query;
                    let _ = key;
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

        async fn retrieve(&self, key: &str) -> Option<PersistQueryData> {
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