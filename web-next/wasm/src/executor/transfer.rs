use shared::core_transfer_protocol::public_cloud::cloud_service::CloudService;
use shared::core_transfer_protocol::webrtc::webrtc::WebRtc;
use shared::executor::transfer::TransferNative;
use shared::rpc::auth_server::AuthServer;
use shared::rpc::cloud_server::CloudServer;
use std::sync::Arc;
use tonic_web_wasm_client::Client;

pub struct TransferNativeImpl {
    pub web_rtc: Arc<WebRtc>,
    pub cloud_service: CloudService<Client>,
    pub cloud_server: CloudServer<Client>,
    pub auth_server: AuthServer<Client>
}

#[ async_trait::async_trait(?Send)]
impl TransferNative<Client> for TransferNativeImpl {
    fn web_rtc(&self) -> &Arc<WebRtc> {
        &self.web_rtc
    }

    fn cloud_service(&self) -> &CloudService<Client> {
        &self.cloud_service
    }

    fn cloud_server(&self) -> &CloudServer<Client> {
        &self.cloud_server
    }

    fn auth_server(&self) -> &AuthServer<Client> {
        &self.auth_server
    }
}
