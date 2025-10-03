use shared::protocol::public_cloud::cloud_service::CloudService;
use shared::protocol::rpc::auth_server::AuthServer;
use shared::protocol::rpc::cloud_server::CloudServer;
use shared::protocol::webrtc::webrtc::WebRtc;
use shared::shell::executor::transfer::TransferNative;
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
