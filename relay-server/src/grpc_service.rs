use schema::devlog::bitbridge::relay_service_server::RelayService;
use schema::devlog::bitbridge::{ConnectRequest, ConnectResponse};
use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::connection::proxy_manager::ProxyManager;

pub struct RelayServiceImpl {
    server: Arc<ProxyManager>,
}

impl RelayServiceImpl {
    pub fn new(server: Arc<ProxyManager>) -> Self {
        Self { server }
    }
}

#[tonic::async_trait]
impl RelayService for RelayServiceImpl {
    async fn connect(&self, request: Request<ConnectRequest>) -> Result<Response<ConnectResponse>, Status> {
        let req = request.get_ref();

        log::info!("Connect request: session_id={}, channels={:?}", req.session_id, req.channels);

        match self.server.handle_connect(req.session_id.clone(), req.sdp.clone(), req.channels.clone()).await {
            Ok(answer_sdp) => {
                log::info!("[relay] Proxy connection initialized for session {}", req.session_id);
                let response = ConnectResponse {
                    success: true,
                    sdp: Some(answer_sdp),
                    error_message: None,
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                log::error!("[relay] Failed to establish proxy connection: {}", e);
                let response = ConnectResponse {
                    success: false,
                    sdp: None,
                    error_message: Some(e.to_string()),
                };
                Ok(Response::new(response))
            }
        }
    }
}
