use prost::Message as ProstMessage;

use core_services::wasm::http::HttpClient;
use schema::devlog::rpc_signalling::server::{
    GeneratePeerRequest, GeneratePeerResponse, IceConfig, OfferMessage, OfferRequest, OfferResponse,
};
use schema::devlog::bitbridge::PeerMessage;
use shared::entities::peer::Peer;

#[derive(Debug, Clone)]
pub struct SignalingClient {
    http_url: String
}

impl SignalingClient {
    pub fn new(_ws_url: impl Into<String>, http_url: impl Into<String>) -> Self {
        Self {
            http_url: http_url.into()
        }
    }

    pub async fn send_offer(&self, peer_id: &str, offer_sdp: &str, session_id: &str, me: PeerMessage) -> Result<(String, PeerMessage), SignalingError> {
        let url = format!("{}/offer/{}", self.http_url.trim_end_matches('/'), peer_id);

        let offer_req = OfferRequest {
            offer: OfferMessage {
                sdp: offer_sdp.to_string(),
                peer: me.clone(),
            },
            peer: me,
            session_id: Some(session_id.to_string())
        };

        let mut encoded = Vec::new();
        prost::Message::encode(&offer_req, &mut encoded).map_err(|e| SignalingError::Encoding(e.to_string()))?;

        let (status, _headers, bytes) = HttpClient::new()
            .method("POST")
            .url(&url)
            .header("Content-Type", "application/octet-stream")
            .body_bytes(encoded)
            .fetch()
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?
            .bytes()
            .await
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?;

        if status != 200 {
            return Err(SignalingError::Server(String::from_utf8_lossy(&bytes).to_string()));
        }

        let response_msg = OfferResponse::decode(&bytes[..]).map_err(|e| SignalingError::Decoding(format!("{:?}", e)))?;

        Ok((response_msg.answer.sdp, response_msg.peer))
    }

    pub async fn generate_peer(
        http_url: &str,
        device: schema::value::device::RegisteringDevice,
        authorization: Option<&str>,
    ) -> Result<Peer, SignalingError> {
        let url = format!("{}/peer", http_url.trim_end_matches('/'));
        let request = GeneratePeerRequest { device };

        let mut encoded = Vec::new();
        prost::Message::encode(&request, &mut encoded).map_err(|e| SignalingError::Encoding(e.to_string()))?;

        let mut http_client = HttpClient::new()
            .method("POST")
            .url(&url)
            .header("Content-Type", "application/octet-stream");

        if let Some(header_value) = authorization {
            http_client = http_client.header("authorization", header_value);
        }

        let (status, _headers, bytes) = http_client
            .body_bytes(encoded)
            .fetch()
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?
            .bytes()
            .await
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?;

        if status != 200 {
            let message = String::from_utf8_lossy(&bytes).to_string();
            return Err(match status {
                400 => SignalingError::BadRequest(message),
                401 | 403 => SignalingError::Unauthorized(message),
                _ => SignalingError::Server(message),
            });
        }

        let response =
            GeneratePeerResponse::decode(&bytes[..]).map_err(|e| SignalingError::Decoding(format!("{:?}", e)))?;

        let mut peer = Peer::from(response.peer);
        peer.signalling_id = response.signalling_id;
        peer.region_code = response.region_code;
        peer.signalling_route = Some(response.signalling_route);

        Ok(peer)
    }

    pub async fn fetch_relay_config(&self, key: &str) -> Result<IceConfig, SignalingError> {
        let url = format!("{}/relay/{}", self.http_url.trim_end_matches('/'), key);

        let (status, _headers, bytes) = HttpClient::new()
            .method("GET")
            .url(&url)
            .fetch()
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?
            .bytes()
            .await
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?;

        if status != 200 {
            return Err(SignalingError::Server(String::from_utf8_lossy(&bytes).to_string()));
        }

        IceConfig::decode(&bytes[..]).map_err(|e| SignalingError::Decoding(e.to_string()))
    }

    pub async fn relay_connect(&self, key: &str, session_id: &str, sdp: &str, channels: Vec<schema::devlog::bitbridge::DataChannel>) -> Result<schema::devlog::bitbridge::ConnectResponse, SignalingError> {
        let url = format!("{}/relay/{}", self.http_url.trim_end_matches('/'), key);
        let request = schema::devlog::bitbridge::ConnectRequest {
            sdp: sdp.to_string(),
            session_id: session_id.to_string(),
            channels,
        };
        
        let mut encoded = Vec::new();
        prost::Message::encode(&request, &mut encoded).map_err(|e| SignalingError::Encoding(e.to_string()))?;

        let (status, _headers, bytes) = HttpClient::new()
            .method("POST")
            .url(&url)
            .header("Content-Type", "application/octet-stream")
            .body_bytes(encoded)
            .fetch()
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?
            .bytes()
            .await
            .map_err(|e| SignalingError::Network(format!("{:?}", e)))?;

        if status != 200 {
            return Err(SignalingError::Server(String::from_utf8_lossy(&bytes).to_string()));
        }

        schema::devlog::bitbridge::ConnectResponse::decode(&bytes[..]).map_err(|e| SignalingError::Decoding(e.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignalingError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Encoding error: {0}")]
    Encoding(String),

    #[error("Decoding error: {0}")]
    Decoding(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid response")]
    InvalidResponse
}
