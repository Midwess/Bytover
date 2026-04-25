use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum StoreKitError {
    #[error("StoreKit is not supported on this platform")]
    Unsupported,
    #[error("Product not found: {0}")]
    ProductNotFound(String),
    #[error("User cancelled purchase")]
    UserCancelled,
    #[error("Observer not initialized")]
    ObserverNotInitialized,
    #[error("Channel closed before result")]
    ChannelClosed,
    #[error("StoreKit error: {0}")]
    Failed(String),
}
