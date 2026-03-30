use crate::protocol::webrtc::errors::WebRtcErrors;
use anyhow::Context;
use core_services::utils::cancellation::CancellationToken;
use futures::channel::mpsc;
use futures_util::lock::Mutex;
use n0_future::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize)]
pub enum TransferDelimiterShema {
    Start {
        session_id: u64,
        resource_id: u64,
        total_size: Option<u64>,
        compressed: bool
    },
    End {
        session_id: u64,
        resource_id: u64
    },
    Hold(u8)
}

impl TransferDelimiterShema {
    pub fn start(session_id: u64, resource_id: u64, compressed: bool) -> Self {
        Self::Start {
            session_id,
            resource_id,
            total_size: None,
            compressed
        }
    }

    pub fn end(session_id: u64, resource_id: u64, _compressed: bool) -> Self {
        Self::End { session_id, resource_id }
    }

    pub fn hold(counter: u8) -> Self {
        Self::Hold(counter)
    }

    pub fn session_id(&self) -> Option<u64> {
        match self {
            Self::Start { session_id, .. } => Some(*session_id),
            Self::End { session_id, .. } => Some(*session_id),
            Self::Hold(_) => None
        }
    }

    pub fn resource_id(&self) -> Option<u64> {
        match self {
            Self::Start { resource_id, .. } => Some(*resource_id),
            Self::End { resource_id, .. } => Some(*resource_id),
            Self::Hold(_) => None
        }
    }

    pub fn compressed(&self) -> bool {
        match self {
            Self::Start { compressed, .. } => *compressed,
            _ => false
        }
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, WebRtcErrors> {
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

        Ok(buffer)
    }

    pub async fn forward_to_next_resource(rx: &mut mpsc::Receiver<Vec<u8>>, session_id: u64) -> Result<Self, WebRtcErrors> {
        loop {
            let Some(packet) = rx.next().await else {
                return Err(WebRtcErrors::InvalidDelimiter(
                    "No more data to read, channel closed".to_owned()
                ))
            };

            if let Ok(delimiter) = Self::from_start_packet(&packet, session_id) {
                return Ok(delimiter)
            }
        }
    }

    pub fn from_start_packet(data: &[u8], session_id: u64) -> Result<Self, WebRtcErrors> {
        let result = Self::from_bytes(data)?;

        if !matches!(result, Self::Start { .. }) {
            return Err(WebRtcErrors::InvalidDelimiter(format!(
                "Expected Start delimiter but got {:?}",
                result
            )));
        }

        if result.session_id() != Some(session_id) {
            return Err(WebRtcErrors::InvalidDelimiter(
                "Invalid delimiter, session_id does not match".to_owned()
            ));
        }

        Ok(result)
    }

    pub fn from_end_packet(data: &[u8], session_id: u64) -> Result<Self, WebRtcErrors> {
        let result = Self::from_bytes(data)?;

        if !matches!(result, Self::End { .. }) {
            return Err(WebRtcErrors::InvalidDelimiter(format!(
                "Expected End delimiter but got {:?}",
                result
            )));
        }

        if result.session_id() != Some(session_id) {
            return Err(WebRtcErrors::InvalidDelimiter(
                "Invalid delimiter, session_id does not match".to_owned()
            ));
        }

        Ok(result)
    }

    pub fn from_hold_packet(data: &[u8]) -> Result<Self, WebRtcErrors> {
        let result = Self::from_bytes(data)?;

        if !matches!(result, Self::Hold(_)) {
            return Err(WebRtcErrors::InvalidDelimiter(format!(
                "Expected Hold delimiter but got {:?}",
                result
            )));
        }

        Ok(result)
    }

    pub fn hold_counter(&self) -> Option<u8> {
        if let Self::Hold(counter) = self {
            Some(*counter)
        } else {
            None
        }
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, WebRtcErrors> {
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

        Ok(result)
    }
}

#[derive(Debug)]
struct SessionContext {
    session_id: u64,
    rtc_request_id: String,
    token: Arc<Mutex<Option<CancellationToken>>>,
    resource_tokens: Arc<Mutex<HashMap<u64, CancellationToken>>>
}

impl SessionContext {
    pub fn new(session_id: u64, rtc_request_id: String) -> Self {
        Self {
            token: Arc::new(Mutex::new(Some(CancellationToken::new()))),
            session_id,
            rtc_request_id,
            resource_tokens: Arc::new(Mutex::new(HashMap::new()))
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
        if let Some(session) = actives.iter().find(|it| it.session_id == session_id) {
            let mut session_token = session.token.lock().await;
            *session_token = Some(token);
        }
    }

    pub async fn start_transfer(&self, session_id: u64, rtc_request_id: String) {
        let mut actives = self.active_transfers.lock().await;
        actives.push(SessionContext::new(session_id, rtc_request_id));
    }

    pub async fn cancel_transfer(&self, session_id: u64) {
        let mut actives = self.active_transfers.lock().await;
        if let Some(session) = actives.iter().find(|it| it.session_id == session_id) {
            let session_token = session.token.lock().await;
            if let Some(token) = session_token.as_ref() {
                token.cancel();
            }
        }

        actives.retain(|it| it.session_id != session_id);
    }

    pub async fn cancel_all_transfers(&self) {
        let actives = self.active_transfers.lock().await;
        for session in actives.iter() {
            let session_token = session.token.lock().await;
            if let Some(token) = session_token.as_ref() {
                token.cancel();
            }
        }
    }

    pub async fn get_or_create_resource_token(&self, session_id: u64, resource_id: u64) -> CancellationToken {
        let mut actives = self.active_transfers.lock().await;

        if !actives.iter().any(|it| it.session_id == session_id) {
            actives.push(SessionContext::new(session_id, String::new()));
        }

        let session = actives.iter().find(|it| it.session_id == session_id).unwrap();

        let mut resource_tokens = session.resource_tokens.lock().await;

        if let Some(token) = resource_tokens.get(&resource_id) {
            if !token.is_cancelled() {
                return token.clone();
            }
        }

        let session_token = session.token.lock().await;
        let parent_token = session_token.as_ref().cloned().unwrap_or_else(CancellationToken::new);
        let child_token = parent_token.child_token();
        resource_tokens.insert(resource_id, child_token.clone());
        child_token
    }

    pub async fn cancel_resource(&self, session_id: u64, resource_id: u64) {
        let actives = self.active_transfers.lock().await;
        if let Some(session) = actives.iter().find(|it| it.session_id == session_id) {
            let resource_tokens = session.resource_tokens.lock().await;
            if let Some(token) = resource_tokens.get(&resource_id) {
                token.cancel();
            }
        }
    }
}
