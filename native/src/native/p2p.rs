use shared::core_transfer_protocol::webrtc::webrtc::WebRtc;
use shared::executor::p2p::P2PNativeExecutor;
use std::sync::Arc;

pub struct P2PNativeExecutorImpl {
    pub web_rtc: Arc<WebRtc>
}

#[async_trait::async_trait]
impl P2PNativeExecutor for P2PNativeExecutorImpl {
    fn web_rtc(&self) -> &Arc<WebRtc> {
        &self.web_rtc
    }
}
