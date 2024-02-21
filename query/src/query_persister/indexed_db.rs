use async_trait::async_trait;

use super::{PersistQueryData, QueryPersister};

#[cfg(any(feature = "hydrate", feature = "csr"))]
use async_cell::unsync::AsyncCell;
#[cfg(any(feature = "hydrate", feature = "csr"))]
use std::rc::Rc;

/// A persister that uses indexed db to persist queries.
#[derive(Clone, Debug)]
pub struct IndexedDbPersister {
    database_name: String,
    object_store: String,
    #[cfg(any(feature = "hydrate", feature = "csr"))]
    database: Rc<AsyncCell<Rc<indexed_db_futures::IdbDatabase>>>,
}

impl Default for IndexedDbPersister {
    fn default() -> Self {
        IndexedDbPersister::new("leptos_query".to_string(), "query_cache".to_string())
    }
}

impl IndexedDbPersister {
    /// Create a new indexed db persister
    pub fn new(database_name: String, object_store: String) -> Self {
        let persister = Self {
            database_name,
            object_store,
            #[cfg(any(feature = "hydrate", feature = "csr"))]
            database: Rc::new(AsyncCell::new()),
        };

        #[cfg(any(feature = "hydrate", feature = "csr"))]
        persister.setup();

        persister
    }

    /// Initialize the persister eagerly, so that it is ready to use when needed.
    #[cfg(any(feature = "hydrate", feature = "csr"))]
    fn setup(&self) {
        let db = {
            let persister = self.clone();
            async move {
                persister.set_up_db().await;
            }
        };
        leptos::spawn_local(async move {
            let _ = db.await;
        })
    }
}

#[cfg(any(feature = "hydrate", feature = "csr"))]
#[async_trait(?Send)]
impl QueryPersister for IndexedDbPersister {
    async fn persist(&self, key: &str, query: PersistQueryData) {
        use js_sys::wasm_bindgen::JsValue;

        let object_store = self.object_store.as_str();
        let db = self.get_database().await;

        let transaction = db
            .transaction_on_one_with_mode(object_store, web_sys::IdbTransactionMode::Readwrite)
            .expect("Failed to create transaction");
        let store = transaction
            .object_store(object_store)
            .expect("Failed to get object store");

        let key = JsValue::from_str(key);
        let value = IndexedDbPersister::to_json_string(&query);

        let _ = store
            .put_key_val(&key, &value)
            .expect("Failed to execute put operation");

        transaction.await;
    }

    async fn remove(&self, key: &str) {
        use js_sys::wasm_bindgen::JsValue;

        let object_store = self.object_store.as_str();
        let db = self.get_database().await;

        let transaction = db
            .transaction_on_one_with_mode(object_store, web_sys::IdbTransactionMode::Readwrite)
            .expect("Failed to create transaction");
        let store = transaction
            .object_store(object_store)
            .expect("Failed to get object store");

        let key = JsValue::from_str(key);

        let _ = store
            .delete(&key)
            .expect("Failed to execute delete operation");

        transaction.await;
    }

    async fn retrieve(&self, key: &str) -> Option<PersistQueryData> {
        use indexed_db_futures::IdbQuerySource;

        let object_store = self.object_store.as_str();
        let db = self.get_database().await;

        let transaction = db
            .transaction_on_one(object_store)
            .expect("Failed to create transaction");
        let store = transaction
            .object_store(object_store)
            .expect("Failed to get object store");

        let key = js_sys::wasm_bindgen::JsValue::from_str(key);
        let request = store
            .get(&key)
            .expect("Failed to execute get operation")
            .await;

        match request {
            Ok(Some(result)) => IndexedDbPersister::from_json_string(&result),
            Ok(None) => None,
            Err(_) => None,
        }
    }

    async fn clear(&self) {
        let object_store = self.object_store.as_str();

        let db = self.get_database().await;

        let transaction = db
            .transaction_on_one_with_mode(object_store, web_sys::IdbTransactionMode::Readwrite)
            .expect("Failed to create transaction");
        let store = transaction
            .object_store(object_store)
            .expect("Failed to get object store");

        let _ = store.clear().expect("Failed to execute clear operation");

        transaction.await;
    }
}

#[cfg(not(any(feature = "hydrate", feature = "csr")))]
#[async_trait(?Send)]
impl QueryPersister for IndexedDbPersister {
    async fn persist(&self, key: &str, query: PersistQueryData) {
        let _ = self.database_name;
        let _ = self.object_store;
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

#[cfg(any(feature = "hydrate", feature = "csr"))]
impl IndexedDbPersister {
    async fn get_database(&self) -> Rc<indexed_db_futures::IdbDatabase> {
        let db = self.database.clone();
        let result = db.get().await;
        result
    }

    async fn set_up_db(&self) {
        let db = self.create_database().await;
        let db = Rc::new(db);

        self.database.set(db);
    }

    async fn create_database(&self) -> indexed_db_futures::IdbDatabase {
        let db_name = self.database_name.as_str();
        let object_store = self.object_store.as_str();

        use indexed_db_futures::{
            request::{IdbOpenDbRequestLike, OpenDbRequest},
            IdbDatabase, IdbVersionChangeEvent,
        };
        use js_sys::wasm_bindgen::JsValue;

        let mut db_req: OpenDbRequest =
            IdbDatabase::open_u32(db_name, 1).expect("Database open request");

        let object_store = object_store.to_string();
        db_req.set_on_upgrade_needed(Some(
            move |evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
                // Check if the object store exists; create it if it doesn't
                if evt
                    .db()
                    .object_store_names()
                    .find(|n| n == object_store.as_str())
                    .is_none()
                {
                    evt.db().create_object_store(object_store.as_str())?;
                }
                Ok(())
            },
        ));

        db_req.await.expect("Database open request")
    }

    fn to_json_string<T: miniserde::Serialize>(value: &T) -> js_sys::wasm_bindgen::JsValue {
        let string = miniserde::json::to_string(value);
        js_sys::wasm_bindgen::JsValue::from_str(&string)
    }

    fn from_json_string<T: miniserde::Deserialize>(
        value: &js_sys::wasm_bindgen::JsValue,
    ) -> Option<T> {
        let value = value.as_string()?;
        miniserde::json::from_str(value.as_str()).ok()
    }
}
