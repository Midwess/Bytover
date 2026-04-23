use crate::app_gateway::app_info::{AppInfoErrors, AppInfoService};
use crate::app_gateway::markov::{Markov, MarkovErrors};
use crate::app_gateway::payment_gateway::{
    PaymentGateway, PaymentGatewayError, StoreKitVerifyOutcome, StoreKitVerifyRejection, StoreKitVerifyRejectionCode,
};
use async_trait::async_trait;
use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use schema::devlog::app_gateway::models::Application;
use schema::devlog::app_gateway::rpc::application_service_client::ApplicationServiceClient;
use schema::devlog::app_gateway::rpc::markov_generator_service_client::MarkovGeneratorServiceClient;
use schema::devlog::app_gateway::rpc::payment_service_client::PaymentServiceClient;
use schema::devlog::app_gateway::rpc::verify_store_kit_transaction_response::Outcome as VerifyStoreKitOutcomeMsg;
use schema::devlog::app_gateway::rpc::{
    GenerateNameRequest, GenerateRandomAvatarRequest, GetApplicationInfoRequest, VerifyStoreKitErrorCode, VerifyStoreKitTransactionRequest,
};

pub struct AppGatewayImpl {
    pub channel: GrpcGatewayChannel,
}

#[async_trait]
impl Markov for AppGatewayImpl {
    async fn generate_name(&self) -> Result<String, MarkovErrors> {
        let channel = self.channel.connect().await?;
        let mut client = MarkovGeneratorServiceClient::new(channel);
        let request = GenerateNameRequest::default();
        let response = client.generate_name(request).await?;

        Ok(response.into_inner().name)
    }
}

#[async_trait]
impl AppInfoService for AppGatewayImpl {
    async fn get_app_info(&self, app_name: String) -> Result<Option<Application>, AppInfoErrors> {
        let channel = self.channel.connect().await?;
        let mut client = ApplicationServiceClient::new(channel);
        let request = GetApplicationInfoRequest { app_name };

        let response = client.get_application_info(request).await?;
        let response = response.into_inner();
        Ok(response.app)
    }

    async fn random_avatar(&self) -> Result<String, AppInfoErrors> {
        let channel = self.channel.connect().await?;
        let mut client = ApplicationServiceClient::new(channel);
        let request = GenerateRandomAvatarRequest {
            app_name: Some("BitBridge".to_owned()),
        };
        let response = client.get_avatar(request).await?;
        Ok(response.into_inner().avatar.unwrap_or_default())
    }
}

#[async_trait]
impl PaymentGateway for AppGatewayImpl {
    async fn verify_storekit_transaction(
        &self,
        user_order_id: u64,
        transaction_id: &str,
    ) -> Result<StoreKitVerifyOutcome, PaymentGatewayError> {
        let channel = self.channel.connect().await?;
        let mut client = PaymentServiceClient::new(channel);
        let request = VerifyStoreKitTransactionRequest {
            transaction_id: transaction_id.to_owned(),
            user_id: Some(user_order_id),
        };

        let response = client.verify_storekit_transaction(request).await?;
        let outcome = response.into_inner().outcome.ok_or(PaymentGatewayError::MalformedResponse)?;

        match outcome {
            VerifyStoreKitOutcomeMsg::Completed(c) => Ok(StoreKitVerifyOutcome::Completed {
                payment_statement_id: c.payment_statement_id,
                transaction_id: c.transaction_id,
                original_transaction_id: c.original_transaction_id,
                product_id: c.product_id,
                amount: c.amount,
                currency: c.currency,
                duplicate: c.duplicate,
            }),
            VerifyStoreKitOutcomeMsg::Error(e) => {
                let parsed = VerifyStoreKitErrorCode::try_from(e.code).unwrap_or(VerifyStoreKitErrorCode::VerifyStorekitErrorCodeUnknown);
                let code = match parsed {
                    VerifyStoreKitErrorCode::VerifyStorekitErrorCodeUnknown => StoreKitVerifyRejectionCode::Unknown,
                    VerifyStoreKitErrorCode::VerifyStorekitErrorCodeNotFound => StoreKitVerifyRejectionCode::NotFound,
                    VerifyStoreKitErrorCode::VerifyStorekitErrorCodeBundleMismatch => StoreKitVerifyRejectionCode::BundleMismatch,
                    VerifyStoreKitErrorCode::VerifyStorekitErrorCodeInvalidSignature => StoreKitVerifyRejectionCode::InvalidSignature,
                    VerifyStoreKitErrorCode::VerifyStorekitErrorCodeEnvMismatch => StoreKitVerifyRejectionCode::EnvMismatch,
                    VerifyStoreKitErrorCode::VerifyStorekitErrorCodeAppleApiError => StoreKitVerifyRejectionCode::AppleApiError,
                    VerifyStoreKitErrorCode::VerifyStorekitErrorCodeProductUnknown => StoreKitVerifyRejectionCode::ProductUnknown,
                    VerifyStoreKitErrorCode::VerifyStorekitErrorCodeConfigMissing => StoreKitVerifyRejectionCode::ConfigMissing,
                    VerifyStoreKitErrorCode::VerifyStorekitErrorCodeAlreadyConsumed => StoreKitVerifyRejectionCode::AlreadyConsumed,
                };
                Ok(StoreKitVerifyOutcome::Rejected(StoreKitVerifyRejection {
                    code,
                    message: e.message,
                }))
            }
        }
    }
}
