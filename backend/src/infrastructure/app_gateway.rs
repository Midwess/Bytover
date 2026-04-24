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
use schema::devlog::app_gateway::rpc::payment_request::Item as PaymentItem;
use schema::devlog::app_gateway::rpc::payment_response::ResponseItem;
use schema::devlog::app_gateway::rpc::payment_service_client::PaymentServiceClient;
use schema::devlog::app_gateway::rpc::{
    GenerateNameRequest, GenerateRandomAvatarRequest, GetApplicationInfoRequest, PaymentRequest, StoreKitPayment,
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
        _user_order_id: u64,
        transaction_id: &str,
        product_id: &str,
    ) -> Result<StoreKitVerifyOutcome, PaymentGatewayError> {
        let channel = self.channel.connect().await?;
        let mut client = PaymentServiceClient::new(channel);

        let request = PaymentRequest {
            idempotency_key: format!("storekit:{transaction_id}"),
            item: Some(PaymentItem::StorekitPayment(StoreKitPayment {
                transaction_id: transaction_id.to_owned(),
                product_id: product_id.to_owned(),
            })),
        };

        let mut stream = client.pay(request).await?.into_inner();
        let mut terminal: Option<StoreKitVerifyOutcome> = None;

        loop {
            match stream.message().await? {
                Some(message) => match message.response_item {
                    Some(ResponseItem::CompletedStatement(payment_statement_id)) => {
                        if terminal.is_none() {
                            terminal = Some(StoreKitVerifyOutcome::Completed {
                                payment_statement_id,
                                transaction_id: transaction_id.to_owned(),
                                original_transaction_id: String::new(),
                                product_id: product_id.to_owned(),
                                amount: 0,
                                currency: String::new(),
                                duplicate: false,
                            });
                        }
                    }
                    Some(ResponseItem::Error(reason)) => {
                        if terminal.is_none() {
                            terminal = Some(StoreKitVerifyOutcome::Rejected(StoreKitVerifyRejection {
                                code: StoreKitVerifyRejectionCode::Unknown,
                                message: reason,
                            }));
                        }
                    }
                    Some(ResponseItem::Redirect(_)) => {
                        return Err(PaymentGatewayError::MalformedResponse);
                    }
                    None => continue,
                },
                None => {
                    return terminal.ok_or(PaymentGatewayError::MalformedResponse);
                }
            }
        }
    }
}
