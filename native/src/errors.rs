use crate::grpc::errors::NativeGrpcErrors;
use crate::network::cloud::cloud_service::CloudTransferErrors;
use crate::network::webrtc::connection::ConnectionWebRtcErrors;
use crate::network::webrtc::peer::PeerErrors;
use crate::network::webrtc::web_rtc::WebRtcErrors;
use shared::errors::NetworkError;

impl From<WebRtcErrors> for NetworkError {
    fn from(err: WebRtcErrors) -> Self {
        match err {
            WebRtcErrors::ConnectionError(e) => NetworkError::Network(e.to_string()),
            WebRtcErrors::SignallingServerError(e) => NetworkError::Network(e.to_string()),
            WebRtcErrors::TransferError(e) => NetworkError::Network(e.to_string()),
            WebRtcErrors::WebRTCServerError(e) => NetworkError::Network(e.to_string())
        }
    }
}

impl From<CloudTransferErrors> for NetworkError {
    fn from(value: CloudTransferErrors) -> Self {
        Self::Network(value.to_string())
    }
}

impl From<ConnectionWebRtcErrors> for NetworkError {
    fn from(err: ConnectionWebRtcErrors) -> Self {
        NetworkError::Network(format!("{err:?}"))
    }
}

impl From<PeerErrors> for NetworkError {
    fn from(err: PeerErrors) -> Self {
        NetworkError::Network(format!("{err:?}"))
    }
}

impl From<NativeGrpcErrors> for NetworkError {
    fn from(err: NativeGrpcErrors) -> Self {
        NetworkError::Network(format!("{err:?}"))
    }
}
