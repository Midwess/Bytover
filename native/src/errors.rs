use shared::core_transfer_protocol::webrtc::errors::WebRtcErrors;
use crate::grpc::errors::NativeGrpcErrors;
use crate::network::cloud::cloud_service::CloudTransferErrors;
use shared::errors::NetworkError;

impl From<CloudTransferErrors> for NetworkError {
    fn from(value: CloudTransferErrors) -> Self {
        Self::Network(value.to_string())
    }
}

impl From<NativeGrpcErrors> for NetworkError {
    fn from(err: NativeGrpcErrors) -> Self {
        NetworkError::Network(format!("{err:?}"))
    }
}
