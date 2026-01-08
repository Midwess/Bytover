use crate::repository::errors::PersistenceError;

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait DeviceAliasRepository: Send + Sync {
    async fn save_aliases(&self, aliases: Vec<String>) -> Result<(), PersistenceError>;
    async fn get_all_aliases(&self) -> Result<Vec<String>, PersistenceError>;
    async fn clear_all(&self) -> Result<(), PersistenceError>;
}
