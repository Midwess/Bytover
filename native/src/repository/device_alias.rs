use core_services::utils::pool::reponse::PoolResponse;
use core_services::utils::pool::request::PoolRequest;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use shared::repository::device_alias::DeviceAliasRepository;
use shared::repository::errors::PersistenceError;

const DEVICE_ALIAS_TABLE: TableDefinition<&str, Vec<u8>> = TableDefinition::new("device_alias");
const ALIASES_KEY: &str = "aliases";

pub struct DeviceAliasRepositoryImpl {
    pub db: PoolRequest<Database>
}

impl DeviceAliasRepositoryImpl {
    async fn get_db(&self) -> PoolResponse<Database> {
        self.db.retrieve().await.unwrap()
    }
}

#[async_trait::async_trait]
impl DeviceAliasRepository for DeviceAliasRepositoryImpl {
    async fn save_aliases(&self, aliases: Vec<String>) -> Result<(), PersistenceError> {
        let db = self.get_db().await;
        let write_txn = db.begin_write().map_err(|e| PersistenceError::IOError(e.to_string()))?;
        {
            let mut table = write_txn.open_table(DEVICE_ALIAS_TABLE).map_err(|e| PersistenceError::IOError(e.to_string()))?;
            let serialized = bincode::serialize(&aliases).map_err(|e| PersistenceError::IOError(e.to_string()))?;
            table.insert(ALIASES_KEY, serialized).map_err(|e| PersistenceError::IOError(e.to_string()))?;
        }
        write_txn.commit().map_err(|e| PersistenceError::IOError(e.to_string()))?;
        Ok(())
    }

    async fn get_all_aliases(&self) -> Result<Vec<String>, PersistenceError> {
        let db = self.get_db().await;
        let read_txn = db.begin_read().map_err(|e| PersistenceError::IOError(e.to_string()))?;
        let table = match read_txn.open_table(DEVICE_ALIAS_TABLE) {
            Ok(t) => t,
            Err(_) => return Ok(vec![])
        };

        match table.get(ALIASES_KEY) {
            Ok(Some(data)) => {
                let aliases: Vec<String> = bincode::deserialize(data.value().as_slice())
                    .map_err(|e| PersistenceError::IOError(e.to_string()))?;
                Ok(aliases)
            }
            Ok(None) => Ok(vec![]),
            Err(e) => Err(PersistenceError::IOError(e.to_string()))
        }
    }

    async fn clear_all(&self) -> Result<(), PersistenceError> {
        let db = self.get_db().await;
        let write_txn = db.begin_write().map_err(|e| PersistenceError::IOError(e.to_string()))?;
        {
            let mut table = write_txn.open_table(DEVICE_ALIAS_TABLE).map_err(|e| PersistenceError::IOError(e.to_string()))?;
            let _ = table.remove(ALIASES_KEY);
        }
        write_txn.commit().map_err(|e| PersistenceError::IOError(e.to_string()))?;
        Ok(())
    }
}
