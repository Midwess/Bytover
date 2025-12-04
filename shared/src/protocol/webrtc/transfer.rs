use std::cell::OnceCell;
use crate::protocol::webrtc::errors::WebRtcErrors;
use futures::channel::mpsc;
use futures_util::lock::Mutex;
use matchbox_socket::Packet;
use n0_future::StreamExt;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use anyhow::Context;
use core_services::utils::cancellation::CancellationToken;

#[derive(Debug, Deserialize, Serialize)]
pub struct TransferDelimiterShema {
    pub session_id: u64,
    pub resource_id: u64,
    pub is_start: bool
}

impl TransferDelimiterShema {
    pub fn new(session_id: u64, resource_id: u64, is_start: bool) -> Self {
        Self {
            resource_id,
            is_start,
            session_id
        }
    }

    pub fn start(session_id: u64, resource_id: u64) -> Self {
        Self::new(session_id, resource_id, true)
    }

    pub fn end(session_id: u64, resource_id: u64) -> Self {
        Self::new(session_id, resource_id, false)
    }

    pub fn as_bytes(&self) -> Result<Packet, WebRtcErrors> {
        let bytes = bincode::serialize(self).context("Cannot serialize delimiter shema to bytes")?;
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

    pub async fn forward_to_next_resource(rx: &mut mpsc::Receiver<Packet>, session_id: u64) -> Result<Self, WebRtcErrors> {
        loop {
            let Some(packet) = rx.next().await else {
                return Err(WebRtcErrors::InvalidDelimiter(
                    "No more data to read, channel closed".to_owned()
                ))
            };

            if let Ok(delimiter) = Self::from_bytes(&packet, session_id, true) {
                return Ok(delimiter)
            }
        }
    }

    pub fn from_end_packet(data: &Packet, session_id: u64) -> Result<Self, WebRtcErrors> {
        Self::from_bytes(data, session_id, false)
    }

    pub fn from_bytes(data: &Packet, session_id: u64, is_start: bool) -> Result<Self, WebRtcErrors> {
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

        if len > 1022 {
            return Err(WebRtcErrors::InvalidDelimiter(
                "Serialized data is larger than buffer size!".to_owned()
            ));
        }

        let serialized_data = &data[2..2 + len];

        let result: Self = bincode::deserialize(serialized_data)
            .map_err(|e| WebRtcErrors::InvalidDelimiter(format!("Failed to deserialize delimiter: {e}")))?;

        if result.is_start != is_start {
            return Err(WebRtcErrors::InvalidDelimiter(
                "Invalid delimiter, is_start does not match".to_owned()
            ));
        }

        if result.session_id != session_id {
            return Err(WebRtcErrors::InvalidDelimiter(
                "Invalid delimiter, session_id does not match".to_owned()
            ));
        }

        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionContext {
    session_id: u64,
    rtc_request_id: String,
    token: OnceCell<CancellationToken>
}

impl SessionContext {
    pub fn new(session_id: u64, rtc_request_id: String) -> Self {
        Self {
            token: OnceCell::new(),
            session_id,
            rtc_request_id,
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

    pub async fn rtc_request_id(&self, session_id: u64) -> Option<String> {
        let actives = self.active_transfers.lock().await;
        actives.iter().find(|it| it.session_id == session_id).map(|it| it.rtc_request_id.clone())
    }

    pub async fn add_token(&self, session_id: u64, token: CancellationToken) {
        let actives = self.active_transfers.lock().await;
        actives.iter().find(|it| it.session_id == session_id).map(|it| it.token.set(token));
    }

    pub async fn start_transfer(&self, session_id: u64, rtc_request_id: String) {
        let mut actives = self.active_transfers.lock().await;
        actives.push(SessionContext::new(session_id, rtc_request_id));
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        let actives = self.active_transfers.lock().await;
        let item = actives.iter().find(|it| it.session_id == session_id).and_then(|it| it.token.get());
        if let Some(token) = item {
            token.cancel();
        }
    }

    pub async fn cancel_all_transfers(&self) {
        let actives = self.active_transfers.lock().await;
        for item in actives.iter() {
            item.token.get().map(|it| it.cancel());
        }
    }
}
