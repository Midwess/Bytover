use schema::devlog::bitbridge::relay_service_server::RelayService;
use schema::devlog::bitbridge::{ConnectRequest, ConnectResponse};
use tonic::{async_trait, Request, Response, Status};

pub struct RelayServiceImpl;

impl RelayServiceImpl {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RelayService for RelayServiceImpl {
    async fn connect(&self, request: Request<ConnectRequest>) -> Result<Response<ConnectResponse>, Status> {
        let req = request.get_ref();
        log::info!("Connect request: session_id={}", req.session_id);
        Ok(Response::new(ConnectResponse {
            success: true,
            sdp: None,
            error_message: None,
        }))
    }
}
