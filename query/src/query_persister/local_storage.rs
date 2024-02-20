use crate::query_persister::*;

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
#[cfg(any(feature = "hydrate", feature = "csr"))]
#[async_trait(?Send)]
impl QueryPersister for LocalStoragePersister {
    async fn persist(&self, key: &str, query: PersistQueryData) {
        if let Some(storage) = local_storage() {
            let value = miniserde::json::to_string(&query);
            let _ = storage.set(&key, &value);
        }
    }

    async fn remove(&self, key: &str) {
        if let Some(storage) = local_storage() {
            let _ = storage.remove_item(key);
        }
    }

    async fn retrieve(&self, key: &str) -> Option<PersistQueryData> {
        if let Some(storage) = local_storage() {
            if let Some(value) = storage.get_item(key).ok().flatten() {
                return miniserde::json::from_str(&value).ok();
            }
        }
        None
    }

    async fn clear(&self) {
        if let Some(storage) = local_storage() {
            let _ = storage.clear();
        }
    }
}

#[cfg(not(any(feature = "hydrate", feature = "csr")))]
#[async_trait(?Send)]
impl QueryPersister for LocalStoragePersister {
    async fn persist(&self, key: &str, query: PersistQueryData) {
        let _ = key;
        let _ = query;
    }

    async fn remove(&self, key: &str) {
        let _ = key;
    }

    async fn retrieve(&self, key: &str) -> Option<PersistQueryData> {
        let _ = key;
        None
    }

    async fn clear(&self) {}
}
