use async_trait::async_trait;

use super::{PersistQueryData, QueryPersister};

/// A persister that uses indexed db to persist queries.
#[derive(Clone, Debug)]
pub struct IndexedDbPersister {
    database_name: String,
    object_store: String,
}

impl Default for IndexedDbPersister {
    fn default() -> Self {
        Self {
            database_name: "leptos_query".to_string(),
            object_store: "query_cache".to_string(),
        }
    }
}

#[cfg(any(feature = "hydrate", feature = "csr"))]
#[async_trait(?Send)]
impl QueryPersister for IndexedDbPersister {
    /// Persist a query to the persister
    async fn persist(&self, key: &str, query: PersistQueryData) {
        use helpers::*;
        use js_sys::wasm_bindgen::JsValue;

        let database_name = self.database_name.as_str();
        let object_store = self.object_store.as_str();
        let db = get_database(database_name, object_store).await;

        let transaction = db
            .transaction_on_one_with_mode(object_store, web_sys::IdbTransactionMode::Readwrite)
            .expect("Failed to create transaction");
        let store = transaction
            .object_store(object_store)
            .expect("Failed to get object store");

        let key = JsValue::from_str(key);
        let value = to_json_string(&query);

        let _ = store
            .put_key_val(&key, &value)
            .expect("Failed to execute put operation");

        transaction.await;
    }
    /// Remove a query from the persister
    async fn remove(&self, key: &str) {
        use helpers::*;
        use js_sys::wasm_bindgen::JsValue;

        let database_name = self.database_name.as_str();
        let object_store = self.object_store.as_str();
        let db = get_database(database_name, object_store).await;

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
    /// Retrieve a query from the persister
    async fn retrieve(&self, key: &str) -> Option<PersistQueryData> {
        use helpers::*;
        use indexed_db_futures::prelude::*;
        use js_sys::wasm_bindgen::JsValue;

        let database_name = self.database_name.as_str();
        let object_store = self.object_store.as_str();
        let db = get_database(database_name, object_store).await;

        let transaction = db
            .transaction_on_one(object_store)
            .expect("Failed to create transaction");
        let store = transaction
            .object_store(object_store)
            .expect("Failed to get object store");

        let key = JsValue::from_str(key);
        let request = store
            .get(&key)
            .expect("Failed to execute get operation")
            .await;

        match request {
            Ok(Some(result)) => from_json_string(&result),
            Ok(None) => None,
            Err(_) => None,
        }
    }

    /// Clear the persister
    async fn clear(&self) {
        use helpers::*;

        let database_name = self.database_name.as_str();
        let object_store = self.object_store.as_str();
        let db = get_database(database_name, object_store).await;

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
mod helpers {

    use std::{cell::OnceCell, rc::Rc};

    use indexed_db_futures::{
        request::{IdbOpenDbRequestLike, OpenDbRequest},
        IdbDatabase, IdbVersionChangeEvent,
    };
    use js_sys::wasm_bindgen::JsValue;

    thread_local! {
       static DATABASE: OnceCell<Rc<IdbDatabase>> = OnceCell::new();
    }

    pub async fn get_database(db_name: &str, object_store: &str) -> Rc<IdbDatabase> {
        if let Some(db) = DATABASE.with(|db| db.get().cloned()) {
            db
        } else {
            let db = create_database(db_name, object_store).await;
            let db = Rc::new(db);

            DATABASE.with(move |db_cell| {
                db_cell.get_or_init(move || db);
            });

            DATABASE.with(|db| db.get().unwrap().clone())
        }
    }

    pub async fn create_database(db_name: &str, object_store: &str) -> IdbDatabase {
        let mut db_req: OpenDbRequest =
            IdbDatabase::open_u32(db_name, 1).expect("Database open request");

        let object_store = object_store.to_string();
        db_req.set_on_upgrade_needed(Some(
            move |evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
                // Check if the object store exists; create it if it doesn't
                if let None = evt
                    .db()
                    .object_store_names()
                    .find(|n| n == object_store.as_str())
                {
                    evt.db().create_object_store(object_store.as_str())?;
                }
                Ok(())
            },
        ));

        db_req.await.expect("Database open request")
    }
    pub fn to_json_string<T: miniserde::Serialize>(value: &T) -> JsValue {
        let string = miniserde::json::to_string(value);
        JsValue::from_str(&string)
    }

    pub fn from_json_string<T: miniserde::Deserialize>(value: &JsValue) -> Option<T> {
        let value = value.as_string()?;
        miniserde::json::from_str(value.as_str()).ok()
    }
}