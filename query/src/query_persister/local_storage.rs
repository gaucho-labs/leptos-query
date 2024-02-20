use crate::query_persister::*;
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

#[async_trait(?Send)]
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
