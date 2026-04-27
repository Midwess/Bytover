use core_services::services::base::Resolve;

use super::service::GatewayService;

#[async_trait::async_trait]
pub trait ApiGatewayClient {
    async fn register(&self, service: GatewayService) -> Resolve<()>;
    async fn get_service(&self, service_name: &str) -> Resolve<Option<GatewayService>>;
    async fn route_to(&self, service: GatewayService) -> Resolve<()>;
    async fn delete_service(&self, service_name: &str) -> Resolve<()>;
}
