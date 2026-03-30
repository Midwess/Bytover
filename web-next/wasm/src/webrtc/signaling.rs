use prost::Message as ProstMessage;

use core_services::wasm::http::HttpClient;
use schema::devlog::rpc_signalling::server::{Message, OfferMessage, IceConfig};

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

    pub async fn send_offer(&self, peer_id: &str, offer_sdp: &str) -> Result<String, SignalingError> {
        let url = format!("{}/offer/{}", self.http_url.trim_end_matches('/'), peer_id);

        let offer_msg = Message {
            request_id: None,
            offer: Some(OfferMessage {
                sdp: offer_sdp.to_string()
            }),
            answer: None,
            error: None,
            ice_config: None
        };

        let mut encoded = Vec::new();
        prost::Message::encode(&offer_msg, &mut encoded).map_err(|e| SignalingError::Encoding(e.to_string()))?;

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

        let response_msg = Message::decode(&bytes[..]).map_err(|e| SignalingError::Decoding(format!("{:?}", e)))?;

        response_msg.answer.ok_or(SignalingError::InvalidResponse).map(|a| a.sdp)
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
    InvalidResponse
}
