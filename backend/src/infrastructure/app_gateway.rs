use crate::app_gateway::markov::{Markov, MarkovErrors};
use async_trait::async_trait;
use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use schema::devlog::auth_gateway::models::Application;
use schema::devlog::auth_gateway::rpc::application_service_client::ApplicationServiceClient;
use schema::devlog::auth_gateway::rpc::markov_generator_service_client::MarkovGeneratorServiceClient;
use schema::devlog::auth_gateway::rpc::{GenerateNameRequest, GetApplicationInfoRequest};
use crate::app_gateway::app_info::{AppInfoErrors, AppInfoService};

pub struct AppGatewayImpl {
    pub channel: GrpcGatewayChannel
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
        let request = GetApplicationInfoRequest {
            app_name
        };

        let response = client.get_application_info(request).await?;
        let response = response.into_inner();
        Ok(response.app)
    }
}