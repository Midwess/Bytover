use async_trait::async_trait;
use thiserror::Error;
use tonic::Status;

#[derive(Debug, Error)]
pub enum PaymentGatewayError {
    #[error("Connection error: {0}")]
    ConnectionError(#[from] tonic::transport::Error),
    #[error("Server error: {0}")]
    TonicStatus(#[from] Status),
    #[error("Malformed response from payment gateway")]
    MalformedResponse,
}

#[derive(Debug, Clone)]
pub enum StoreKitVerifyOutcome {
    Completed {
        payment_statement_id: u64,
        transaction_id: String,
        original_transaction_id: String,
        product_id: String,
        amount: u64,
        currency: String,
        duplicate: bool,
    },
    Rejected(StoreKitVerifyRejection),
}

#[derive(Debug, Clone)]
pub struct StoreKitVerifyRejection {
    pub code: StoreKitVerifyRejectionCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKitVerifyRejectionCode {
    Unknown,
    NotFound,
    BundleMismatch,
    InvalidSignature,
    EnvMismatch,
    AppleApiError,
    ProductUnknown,
    ConfigMissing,
    AlreadyConsumed,
}

#[async_trait]
pub trait PaymentGateway: Send + Sync {
    async fn verify_storekit_transaction(
        &self,
        user_order_id: u64,
        transaction_id: &str,
    ) -> Result<StoreKitVerifyOutcome, PaymentGatewayError>;
}
