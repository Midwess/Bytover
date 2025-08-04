use crate::app_gateway::markov::{Markov, MarkovErrors};
use async_trait::async_trait;
use devlog_sdk::grpc_gateway::channel::GrpcGatewayChannel;
use schema::devlog::auth_gateway::rpc::markov_generator_service_client::MarkovGeneratorServiceClient;
use schema::devlog::auth_gateway::rpc::GenerateNameRequest;

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
