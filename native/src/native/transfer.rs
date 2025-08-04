use shared::core_transfer_protocol::public_cloud::cloud_service::CloudService;
use shared::core_transfer_protocol::webrtc::webrtc::WebRtc;
use shared::executor::transfer::TransferNative;
use shared::rpc::auth_server::AuthServer;
use shared::rpc::cloud_server::CloudServer;
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

    fn auth_server(&self) -> &AuthServer<Channel> {
        todo!()
    }
}
