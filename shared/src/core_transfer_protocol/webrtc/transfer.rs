use crate::core_transfer_protocol::webrtc::errors::WebRtcErrors;
use futures_util::lock::Mutex;
use matchbox_socket::Packet;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub struct TransferDelimiterShema {
    pub resource_id: u64,
    pub is_start: bool
}

impl TransferDelimiterShema {
    pub fn new(resource_id: u64, is_start: bool) -> Self {
        Self { resource_id, is_start }
    }

    pub fn start(resource_id: u64) -> Self {
        Self::new(resource_id, true)
    }

    pub fn end(resource_id: u64) -> Self {
        Self::new(resource_id, false)
    }

    pub fn as_bytes(&self) -> Result<Packet, WebRtcErrors> {
        let bytes = bincode::serialize(self).unwrap();
        let mut buffer = vec![0u8; 1024];

        let len = bytes.len();
        if len + 2 > 1022 {
            return Err(WebRtcErrors::InvalidDelimiter(
                "Serialized data is larger than buffer size!".to_owned()
            ));
        }

        buffer[0..2].copy_from_slice(&(len as u16).to_le_bytes());

        buffer[2..2 + len].copy_from_slice(&bytes);

        Ok(buffer.into_boxed_slice())
    }

    pub fn from_bytes(data: &Packet) -> Result<Self, WebRtcErrors> {
        if data.len() != 1024 {
            return Err(WebRtcErrors::InvalidDelimiter(format!(
                "Data buffer must be exactly 1024 bytes got {}",
                data.len()
            )))
        }

        let len_bytes = &data[0..2];
        let len = u16::from_le_bytes([
            len_bytes[0],
            len_bytes[1]
        ]) as usize;

        let serialized_data = &data[2..2 + len];

        bincode::deserialize(serialized_data)
            .map_err(|e| WebRtcErrors::InvalidDelimiter(format!("Failed to deserialize delimiter: {e}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionContext {
    session_id: u64,
    rtc_request_id: String
}

impl SessionContext {
    pub fn new(session_id: u64, rtc_request_id: String) -> Self {
        Self {
            session_id,
            rtc_request_id
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransfersContext {
    active_transfers: Arc<Mutex<Vec<SessionContext>>>
}

impl Default for TransfersContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TransfersContext {
    pub fn new() -> Self {
        Self {
            active_transfers: Arc::new(Mutex::new(Vec::new()))
        }
    }

    pub async fn start_transfer(&self, session_id: u64, rtc_request_id: String) {
        let mut actives = self.active_transfers.lock().await;
        if !actives.iter().any(|it| it.session_id == session_id) {
            actives.push(SessionContext::new(session_id, rtc_request_id));
        }
    }

    pub async fn stop_transfer(&self, session_id: u64) {
        let mut actives = self.active_transfers.lock().await;
        actives.retain(|x| x.session_id != session_id);
    }

    pub async fn is_active(&self, session_id: u64) -> bool {
        let actives = self.active_transfers.lock().await;
        actives.iter().any(|it| it.session_id == session_id)
    }

    pub async fn rtc_request_id(&self, session_id: u64) -> Option<String> {
        let actives = self.active_transfers.lock().await;
        actives.iter().find(|it| it.session_id == session_id).map(|it| it.rtc_request_id.clone())
    }

    pub async fn stop_all(&self) {
        let mut actives = self.active_transfers.lock().await;
        actives.clear();
    }
}
