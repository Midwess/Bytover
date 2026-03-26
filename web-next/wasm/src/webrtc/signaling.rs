//! HTTP-based WebRTC Signaling for WASM (Protobuf Transport)
//!
//! Sends offer to the signalling server and receives answer via protobuf.

use js_sys::Uint8Array;
use prost::Message as ProstMessage;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

use schema::devlog::rpc_signalling::server::{Message, OfferMessage};

/// Errors that can occur during signaling
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

/// Send offer to signalling server via protobuf and receive answer.
///
/// 1. Encodes offer as protobuf Message { offer: { sdp } }
/// 2. POSTs to `/offer/{peer_id}` with Content-Type: application/octet-stream
/// 3. Decodes response as protobuf Message { answer: { sdp } }
/// 4. Returns the answer SDP
pub async fn send_offer_proto(
    signalling_url: &str,
    peer_id: &str,
    offer_sdp: &str,
) -> Result<String, SignalingError> {
    let url = format!(
        "{}/offer/{}",
        signalling_url.trim_end_matches('/'),
        peer_id
    );

    // Encode offer as protobuf
    let offer_msg = Message {
        request_id: None,
        offer: Some(OfferMessage {
            sdp: offer_sdp.to_string(),
        }),
        answer: None,
        error: None,
        ice_config: None,
    };

    let mut encoded = Vec::new();
    prost::Message::encode(&offer_msg, &mut encoded)
        .map_err(|e| SignalingError::Encoding(e.to_string()))?;

    // Create fetch request with binary body
    let opts = RequestInit::new();
    opts.set_method("POST");

    let body_array = js_sys::Uint8Array::new_with_length(encoded.len() as u32);
    body_array.copy_from(&encoded);
    opts.set_body(&body_array.into());

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| SignalingError::Network(format!("Failed to create request: {:?}", e)))?;

    request
        .headers()
        .set("Content-Type", "application/octet-stream")
        .map_err(|e| SignalingError::Network(format!("Failed to set header: {:?}", e)))?;

    // Make the fetch request
    let window = web_sys::window().ok_or_else(|| SignalingError::Network("No window".to_string()))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| SignalingError::Network(format!("Fetch failed: {:?}", e)))?;

    let response: Response = resp_value
        .dyn_into()
        .map_err(|e| SignalingError::Network(format!("Failed to cast response: {:?}", e)))?;

    // Check status
    let status = response.status();
    if status != 200 {
        let array_buffer: js_sys::ArrayBuffer = JsFuture::from(
            response
                .array_buffer()
                .map_err(|e| SignalingError::Network(format!("Failed to get array buffer: {:?}", e)))?,
        )
        .await
        .map_err(|e| SignalingError::Network(format!("Failed to read error body: {:?}", e)))?
        .dyn_into()
        .map_err(|e| SignalingError::Network(format!("Failed to cast to ArrayBuffer: {:?}", e)))?;
        return Err(SignalingError::Server(
            String::from_utf8_lossy(&js_sys::Uint8Array::new(&array_buffer).to_vec()).to_string(),
        ));
    }

    // Read response body
    let array_buffer: js_sys::ArrayBuffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|e| SignalingError::Network(format!("Failed to get array buffer: {:?}", e)))?,
    )
    .await
    .map_err(|e| SignalingError::Network(format!("Failed to read body: {:?}", e)))?
    .dyn_into()
    .map_err(|e| SignalingError::Network(format!("Failed to cast to ArrayBuffer: {:?}", e)))?;

    let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();

    // Decode protobuf response
    let response_msg = Message::decode(&bytes[..])
        .map_err(|e| SignalingError::Decoding(format!("Failed to decode response: {:?}", e)))?;

    // Extract answer SDP
    let answer_sdp = response_msg
        .answer
        .ok_or_else(|| SignalingError::InvalidResponse)?
        .sdp;

    Ok(answer_sdp)
}
