use prost::Message as ProstMessage;

use core_services::wasm::http::HttpClient;
use schema::devlog::bitbridge::PeerMessage;
use schema::devlog::rpc_signalling::server::{IceConfig, OfferMessage, OfferRequest, OfferResponse};

#[derive(Debug, Clone)]
pub struct SignalingClient {
    http_url: String,
}

impl SignalingClient {
    pub fn new(_ws_url: impl Into<String>, http_url: impl Into<String>) -> Self {
        Self { http_url: http_url.into() }
    }

    pub async fn send_offer(
        &self,
        peer_id: &str,
        offer_sdp: &str,
        session_id: &str,
        me: PeerMessage,
    ) -> Result<(String, PeerMessage), SignalingError> {
        let url = format!("{}/offer/{}", self.http_url.trim_end_matches('/'), peer_id);

        let offer_req = OfferRequest {
            offer: OfferMessage {
                sdp: offer_sdp.to_string(),
                peer: me.clone(),
            },
            peer: me,
            session_id: Some(session_id.to_string()),
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
}

#[derive(Debug, thiserror::Error)]
pub enum SignalingError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Encoding error: {0}")]
    Encoding(String),

    #[error("Decoding error: {0}")]
    Decoding(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid response")]
    InvalidResponse,
}
