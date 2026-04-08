use shared::entities::device::DeviceInfo;
use shared::entities::peer::Peer;
use shared::errors::CoreError;
use shared::protocol::rpc::app_server::AppServer;
use shared::protocol::rpc::auth_provider::AuthProvider;
use shared::shell::executor::rpc::NativeRpc;
use tonic_web_wasm_client::Client;

use crate::webrtc::signaling::{SignalingClient, SignalingError};

pub struct NativeRpcImpl {
    pub auth_server: AppServer<Client>,
    pub auth_provider: AuthProvider,
    pub signalling_http_url: String,
}

#[async_trait::async_trait(?Send)]
impl NativeRpc<Client> for NativeRpcImpl {
    fn app_server(&self) -> &AppServer<Client> {
        &self.auth_server
    }

    async fn gen_peer(&self, device: DeviceInfo) -> Result<Peer, CoreError> {
        let authorization = self
            .auth_provider
            .authorization_header()
            .await
            .map_err(|error| CoreError::Network(error.to_string()))?;

        SignalingClient::generate_peer(
            &self.signalling_http_url,
            device.into(),
            authorization.as_deref(),
        )
        .await
        .map_err(map_signalling_error)
    }
}

fn map_signalling_error(error: SignalingError) -> CoreError {
    match error {
        SignalingError::Unauthorized(message) => CoreError::Unauthorized(message),
        SignalingError::BadRequest(message) => CoreError::BadRequest(message),
        other => CoreError::Network(other.to_string()),
    }
}
