use shared::protocol::public_cloud::cloud_service::CloudService;
use shared::protocol::rpc::app_server::AppServer;
use shared::protocol::rpc::cloud_server::CloudServer;
use shared::shell::executor::transfer::{TransferNative, WebRtc};
use std::sync::Arc;
use tonic::transport::Channel;

pub struct TransferNativeImpl {
    pub web_rtc: Arc<WebRtc>,
    pub cloud_service: CloudService<Channel>
}

#[async_trait::async_trait]
impl TransferNative<Channel> for TransferNativeImpl {
    fn web_rtc(&self) -> &Arc<WebRtc> {
        &self.web_rtc
    }

    fn cloud_service(&self) -> &CloudService<Channel> {
        &self.cloud_service
    }

    fn cloud_server(&self) -> &CloudServer<Channel> {
        todo!()
    }

    fn app_server(&self) -> &AppServer<Channel> {
        todo!()
    }
}
