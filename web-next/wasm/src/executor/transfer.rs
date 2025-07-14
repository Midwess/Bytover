use crate::ShellRuntime;
use shared::core_transfer_protocol::public_cloud::cloud_service::CloudService;
use shared::core_transfer_protocol::webrtc::webrtc::WebRtc;
use std::sync::Arc;
use tonic_web_wasm_client::Client;
use shared::executor::transfer::TransferNative;

pub struct TransferNativeImpl {
    pub web_rtc: Arc<WebRtc>,
    pub cloud_service: CloudService<Client>,
}

#[ async_trait::async_trait(?Send)]
impl TransferNative<Client> for TransferNativeImpl {
    fn web_rtc(&self) -> &Arc<WebRtc> {
        &self.web_rtc
    }

    fn cloud_service(&self) -> &CloudService<Client> {
        &self.cloud_service
    }
}
