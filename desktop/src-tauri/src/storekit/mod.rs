pub mod error;

#[cfg(target_os = "macos")]
mod macos;

pub use error::StoreKitError;

use async_trait::async_trait;

pub const PREMIUM_PRODUCT_ID: &str = "com.midwess.bytover.premium";

#[derive(Debug, Clone)]
pub struct StoreKitTransaction {
    pub transaction_id: String,
    pub product_id: String,
    #[allow(dead_code)]
    pub original_transaction_id: Option<String>,
}

#[async_trait]
pub trait StoreKitClient: Send + Sync {
    async fn purchase(&self, product_id: &str) -> Result<StoreKitTransaction, StoreKitError>;
    async fn unfinished_transactions(&self) -> Result<Vec<StoreKitTransaction>, StoreKitError>;
    async fn finish(&self, transaction_id: &str) -> Result<(), StoreKitError>;
    async fn restore(&self) -> Result<Vec<StoreKitTransaction>, StoreKitError>;
}

#[cfg(target_os = "macos")]
pub fn default_client() -> std::sync::Arc<dyn StoreKitClient> {
    std::sync::Arc::new(macos::MacStoreKitClient::shared())
}

#[cfg(not(target_os = "macos"))]
pub fn default_client() -> std::sync::Arc<dyn StoreKitClient> {
    std::sync::Arc::new(StubStoreKitClient)
}

#[cfg(not(target_os = "macos"))]
struct StubStoreKitClient;

#[cfg(not(target_os = "macos"))]
#[async_trait]
impl StoreKitClient for StubStoreKitClient {
    async fn purchase(&self, _product_id: &str) -> Result<StoreKitTransaction, StoreKitError> {
        Err(StoreKitError::Unsupported)
    }
    async fn unfinished_transactions(&self) -> Result<Vec<StoreKitTransaction>, StoreKitError> {
        Ok(Vec::new())
    }
    async fn finish(&self, _transaction_id: &str) -> Result<(), StoreKitError> {
        Err(StoreKitError::Unsupported)
    }
    async fn restore(&self) -> Result<Vec<StoreKitTransaction>, StoreKitError> {
        Err(StoreKitError::Unsupported)
    }
}
