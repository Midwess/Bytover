use core_services::utils::never_send::NeverSend;
use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use idb::{Database, TransactionMode};
use shared::repository::device_alias::DeviceAliasRepository;
use shared::repository::errors::PersistenceError;
use wasm_bindgen::JsValue;

const DEVICE_ALIAS_STORE: &str = "device_alias";
const ALIASES_KEY: &str = "aliases";

pub struct DeviceAliasRepositoryImpl {
    pub db: PoolRequest<NeverSend<Database>>,
}

impl DeviceAliasRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<NeverSend<Database>> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait(?Send)]
impl DeviceAliasRepository for DeviceAliasRepositoryImpl {
    async fn save_aliases(&self, aliases: Vec<String>) -> Result<(), PersistenceError> {
        let db = self.get_db().await;
        let transaction = db
            .transaction(&[DEVICE_ALIAS_STORE], TransactionMode::ReadWrite)
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        let store = transaction
            .object_store(DEVICE_ALIAS_STORE)
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        let value = serde_wasm_bindgen::to_value(&aliases).map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        store
            .put(&value, Some(&JsValue::from_str(ALIASES_KEY)))
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?
            .await
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        transaction.commit().map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        Ok(())
    }

    async fn get_all_aliases(&self) -> Result<Vec<String>, PersistenceError> {
        let db = self.get_db().await;
        let transaction = db
            .transaction(&[DEVICE_ALIAS_STORE], TransactionMode::ReadOnly)
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        let store = transaction
            .object_store(DEVICE_ALIAS_STORE)
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        let result = store
            .get(JsValue::from_str(ALIASES_KEY))
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?
            .await
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        match result {
            Some(value) => {
                let aliases: Vec<String> =
                    serde_wasm_bindgen::from_value(value).map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;
                Ok(aliases)
            }
            None => Ok(vec![]),
        }
    }

    async fn clear_all(&self) -> Result<(), PersistenceError> {
        let db = self.get_db().await;
        let transaction = db
            .transaction(&[DEVICE_ALIAS_STORE], TransactionMode::ReadWrite)
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        let store = transaction
            .object_store(DEVICE_ALIAS_STORE)
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        let _ = store
            .delete(JsValue::from_str(ALIASES_KEY))
            .map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?
            .await;

        transaction.commit().map_err(|e| PersistenceError::IOError(format!("{:?}", e)))?;

        Ok(())
    }
}
