use crate::app_gateway::app_info::{AppInfoErrors, AppInfoService};
use crate::app_gateway::markov::{Markov, MarkovErrors};
use async_trait::async_trait;
use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use schema::devlog::app_gateway::models::Application;
use schema::devlog::app_gateway::rpc::application_service_client::ApplicationServiceClient;
use schema::devlog::app_gateway::rpc::markov_generator_service_client::MarkovGeneratorServiceClient;
use schema::devlog::app_gateway::rpc::{GenerateNameRequest, GenerateRandomAvatarRequest, GetApplicationInfoRequest};

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
