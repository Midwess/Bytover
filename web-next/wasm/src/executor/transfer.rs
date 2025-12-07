use shared::protocol::public_cloud::cloud_service::CloudService;
use shared::protocol::rpc::app_server::AppServer;
use shared::protocol::rpc::cloud_server::CloudServer;
use shared::protocol::webrtc::webrtc::WebRtc;
use shared::shell::executor::transfer::TransferNative;
use std::sync::Arc;
use tonic_web_wasm_client::Client;

pub struct TransferNativeImpl {
    pub web_rtc: Arc<WebRtc>,
    pub cloud_service: CloudService<Client>,
    pub cloud_server: &'static CloudServer<Client>,
    pub auth_server: AppServer<Client>
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
        self.cloud_server
    }

    fn app_server(&self) -> &AppServer<Client> {
        &self.auth_server
    }
}
