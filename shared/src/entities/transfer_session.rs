use std::fmt::Display;

use crate::app::core::model_events::{SessionLoadError, UpdateAction};
use crate::entities::local_resource::{LocalResource, LocalResourcePath};
use crate::entities::peer::Peer;
use crate::entities::target::{P2PConnectionState, TransferTarget};
use crate::entities::user::User;
use crate::repository::local_resource::LocalResourceId;
use chrono::Utc;
use core_services::db::repository::abstraction::id::DbId;
use serde::{Deserialize, Serialize};
use core_services::utils::cancellation::CancellationToken;
use schema::devlog::bitbridge::P2pTransferSessionMessage;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum TransferType {
    Send,
    Receive
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum TransferSessionStatus {
    Initializing {
        loading_error: Option<String>,
        loading_state: Option<String>
    },
    InProgress { bytes_per_second: u64, percentage: f64 },
    Success,
    Failed(String),
    Canceled
}

impl TransferSessionStatus {
    pub fn is_completed(&self) -> bool {
        matches!(self, TransferSessionStatus::Success | TransferSessionStatus::Failed(_) | TransferSessionStatus::Canceled)
    }
}

impl Display for TransferSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferSessionStatus::Initializing { loading_state: Some(text), loading_error: None } => write!(f, "{text}"),
            TransferSessionStatus::Initializing { loading_error: Some(text), .. } => write!(f, "Error: {text}"),
            TransferSessionStatus::Initializing {.. } => write!(f, "Initializing..."),
            TransferSessionStatus::InProgress { bytes_per_second, .. } => {
                let kb_per_second = *bytes_per_second as f64 / 1000.0;
                if kb_per_second < 100.0 {
                    write!(f, "{kb_per_second:.1} KB/s")
                } else {
                    write!(f, "{:.1} MB/s", kb_per_second / 1024.0)
                }
            }
            TransferSessionStatus::Success => write!(f, "Done ☺️!"),
            TransferSessionStatus::Failed(msg) => write!(f, "Failed 🫨 {msg}"),
            TransferSessionStatus::Canceled => write!(f, "Canceled"),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct TransferSession {
    pub order_id: u64,
    pub resources: Vec<LocalResource>,
    pub progress: Vec<TransferProgress>,
    pub transfer_type: TransferType,
    pub target: TransferTarget,
    pub access_url: String,
    pub alias: String,
    pub from_user: User,
    pub description: Option<String>,
    pub password: Option<String>,
    pub is_required_password: bool,
    #[serde(skip)]
    pub cancellation_token: CancellationToken
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct TransferProgress {
    pub resource_order_id: u64,
    pub file_size: u64,
    total_bytes_counter: u64,
    bytes_per_second: u64,
    start_time_utc_ms: u64,
    bytes_sec_counter: u64,
    last_update_time_ms: u64,
    pub transfer_type: TransferType,
    pub status: TransferStatus,
}

impl TransferProgress {
    pub fn new(resource_order_id: u64, file_size: u64, transfer_type: TransferType) -> Self {
        Self {
            resource_order_id,
            file_size,
            total_bytes_counter: 0,
            bytes_per_second: 0,
            bytes_sec_counter: 0,
            last_update_time_ms: Utc::now().timestamp_millis() as u64,
            start_time_utc_ms: Utc::now().timestamp_millis() as u64,
            transfer_type,
            status: TransferStatus::Pending,
        }
    }

    pub fn complete(&mut self) {
        if self.percentage() == 1.0 {
            self.status = TransferStatus::Success
        } else {
            self.status = TransferStatus::Fail(format!(
                "Data corrupted transfer for resource {} received {}/1.0",
                self.resource_order_id,
                self.percentage()
            ))
        };
    }

    pub fn success(&mut self) {
        self.complete();
        self.total_bytes_counter = self.file_size;
        self.status = TransferStatus::Success;
    }

    pub fn fail(&mut self, msg: String) {
        self.complete();
        self.status = TransferStatus::Fail(msg);
    }

    pub fn percentage(&self) -> f64 {
        (self.total_bytes_counter as f64 / self.file_size as f64).min(1.0)
    }

    pub fn is_completed(&self) -> bool {
        self.is_failed() || self.is_success() || self.is_canceled()
    }

    pub fn is_failed(&self) -> bool {
        matches!(self.status, TransferStatus::Fail(_))
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, TransferStatus::Success)
    }

    pub fn is_canceled(&self) -> bool {
        matches!(self.status, TransferStatus::Canceled)
    }

    pub fn elapsed(&self) -> u64 {
        Utc::now().timestamp_millis() as u64 - self.start_time_utc_ms
    }

    pub fn update_progress(&mut self, bytes_count: u64) {
        self.last_update_time_ms = Utc::now().timestamp_millis() as u64;

        if self.status == TransferStatus::Pending {
            self.status = TransferStatus::InProgress;
        }

        if self.status != TransferStatus::InProgress {
            return;
        }

        let elapsed = self.elapsed();

        self.total_bytes_counter += bytes_count;
        self.bytes_sec_counter += bytes_count;

        if elapsed >= 1000 {
            let secs = elapsed / 1000;
            self.bytes_per_second = self.bytes_sec_counter / secs;
            self.start_time_utc_ms = Utc::now().timestamp_millis() as u64;
            self.bytes_sec_counter = bytes_count;
        }

        if self.percentage() == 1.0 {
            self.bytes_per_second = self.bytes_sec_counter;
            self.complete();
        }
    }

    pub fn speed(&self) -> u64 {
        if self.elapsed() >= 1000 {
            return 0;
        }

        self.bytes_per_second
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Fail(String),
    Success,
    Canceled
}

impl TransferStatus {
    pub fn is_completed(&self) -> bool {
        matches!(
            self,
            TransferStatus::Success | TransferStatus::Fail(_) | TransferStatus::Canceled
        )
    }
}

impl TransferSession {
    pub fn p2p(mut resources: Vec<LocalResource>, password: Option<String>, signalling_key: String, scope: String, alias: String, access_url: String, id: u64) -> Self {
        resources.sort_by(|a, b| a.size.cmp(&b.size));
        let is_required_password = password.is_some();
        Self {
            order_id: id,
            access_url,
            alias,
            progress: resources
                .iter()
                .map(|it| TransferProgress::new(it.order_id, it.size, TransferType::Send))
                .collect(),
            resources,
            description: None,
            transfer_type: TransferType::Send,
            target: TransferTarget::P2P {
                from_peer: None,
                signalling_key,
                scope,
                connection_state: P2PConnectionState::NotConnected,
            },
            from_user: User { id: 0, email: String::new(), name: String::new(), avatar: String::new() },
            password,
            is_required_password,
            cancellation_token: CancellationToken::new()
        }
    }

    pub fn public(current_user: User, password: Option<String>, resources: Vec<LocalResource>, to_emails: Vec<String>) -> Self {
        let is_required_password = password.is_some();
        Self {
            alias: "".to_owned(),
            access_url: "".to_owned(),
            order_id: 0, // It is decided by the backend
            progress: resources.iter().map(|it| TransferProgress::new(it.order_id, it.size, TransferType::Send)).collect(),
            cancellation_token: CancellationToken::new(),
            resources,
            transfer_type: TransferType::Send,
            target: TransferTarget::Internet {
                to_emails
            },
            description: None,
            from_user: current_user,
            password,
            is_required_password,
        }
    }

    pub fn owner_connected(&mut self, peer: Peer) {
        if let TransferTarget::P2P { from_peer, connection_state, .. } = &mut self.target {
            from_peer.replace(peer);
            *connection_state = P2PConnectionState::Connected;
        }
    }

    pub fn owner_disconnected(&mut self) {
        if let TransferTarget::P2P { from_peer, connection_state, .. } = &mut self.target {
            from_peer.take();
            *connection_state = P2PConnectionState::NotConnected;
        }

        self.is_required_password = false;
        self.password.take();
        self.progress.clear();
        self.resources.clear();
    }

    pub fn set_connecting(&mut self) {
        if let TransferTarget::P2P { connection_state, .. } = &mut self.target {
            *connection_state = P2PConnectionState::Connecting;
        }
    }

    pub fn set_connection_failed(&mut self, error: String) {
        if let TransferTarget::P2P { connection_state, .. } = &mut self.target {
            *connection_state = P2PConnectionState::Failed(error);
        }
    }

    pub fn is_p2p_connected(&self) -> bool {
        matches!(
            self.target,
            TransferTarget::P2P {
                connection_state: P2PConnectionState::Connected,
                ..
            }
        )
    }

    pub fn add_resource_from_peer(&mut self, resource: LocalResource, peer: &Peer) -> bool {
        if !peer.is_owned(self) {
            log::warn!(
                "Peer {} is not owner of session {}, ignoring resource",
                peer.id(),
                self.order_id
            );
            return false;
        }

        self.add_resource(resource);
        true
    }

    pub fn from_public_overview(order_id: u64, from_user: User, access_url: String, alias: String, is_required_password: bool) -> Self {
        Self {
            order_id,
            progress: vec![],
            resources: vec![],
            access_url,
            alias,
            transfer_type: TransferType::Receive,
            cancellation_token: CancellationToken::new(),
            target: TransferTarget::Internet {
                to_emails: vec![],
            },
            from_user: from_user,
            description: None,
            password: None,
            is_required_password,
        }
    }

    pub fn add_resource(&mut self, resource: LocalResource) {
        if self.resource_progress(resource.order_id).is_none() {
            self.progress.push(TransferProgress::new(resource.order_id, resource.size, self.transfer_type.clone()));
        }

        if self.resources.iter().any(|it| it.order_id == resource.order_id) {
            return
        }

        self.resources.push(resource);
        self.resources.sort_by(|a, b| a.size.cmp(&b.size));
    }

    pub fn replace_resource(&mut self, resource: LocalResource) {
        if self.resource_progress(resource.order_id).is_none() {
            self.progress.push(TransferProgress::new(resource.order_id, resource.size, self.transfer_type.clone()));
        }

        self.resources.retain(|it| it.order_id != resource.order_id);
        self.resources.push(resource);
        self.resources.sort_by(|a, b| a.size.cmp(&b.size));
    }

    pub fn peer_id(&self) -> Option<String> {
        match &self.target {
            TransferTarget::P2P { from_peer, .. } => from_peer.as_ref().map(|p| p.id().to_string()),
            _ => None
        }
    }

    pub fn peer(&self) -> Option<&Peer> {
        match &self.target {
            TransferTarget::P2P { from_peer, .. } => from_peer.as_ref(),
            _ => None
        }
    }

    pub fn is_keyword_match(&self, keywords: &str) -> bool {
        if keywords.is_empty() {
            return true;
        }

        let from_user = &self.from_user;

        let mut name: String = "".to_string();
        if let Ok(url) = url::Url::parse(&self.access_url) {
            let Some(query) = url.query_pairs().find(|(key, _)| key == "session").map(|it| it.1.to_string()) else {
                return false
            };

            log::info!("Found query key session: {}", query);
            name = query;
        }

        from_user.name.to_lowercase() == keywords.to_lowercase() || name.to_lowercase() == keywords.to_lowercase()
    }

    fn is_initializing(&self) -> bool {
        self.progress.iter().all(|it| it.status == TransferStatus::InProgress && it.bytes_per_second == 0)
    }

    pub fn update_progress(&mut self, progress: TransferProgress) {
        if let Some(index) = self.progress.iter().position(|it| it.resource_order_id == progress.resource_order_id) {
            self.progress[index] = progress;
        } else {
            self.progress.push(progress);
        }
    }

    pub fn force_complete(&mut self, msg: String) {
        self.progress.iter_mut().for_each(|it| {
            if it.status == TransferStatus::InProgress || it.status == TransferStatus::Pending {
                it.status = TransferStatus::Fail(msg.clone());
            }
        });
    }

    pub fn total_progress(&self) -> f64 {
        let total_size = self.resources.iter().map(|it| it.size).sum::<u64>();
        if total_size == 0 {
            return 1.0;
        }

        let total_bytes_sent = self.progress.iter().map(|it| it.total_bytes_counter).sum::<u64>();
        total_bytes_sent as f64 / total_size as f64
    }

    pub fn speed(&self, _interval: u64) -> u64 {
        self.progress.iter().map(|it| it.speed()).sum::<u64>()
    }

    pub fn is_completed(&self) -> bool {
        self.status().is_completed()
    }

    pub fn is_canceled(&self) -> bool {
        if self.cancellation_token.is_cancelled() {
            return true;
        }

        self.progress.iter().any(|it| it.is_canceled())
    }

    pub fn is_failed(&self) -> bool {
        self.progress.iter().any(|it| it.is_failed())
    }

    pub fn is_success(&self) -> bool {
        self.progress.iter().any(|it| it.is_success())
    }

    pub fn cancel(&mut self) {
        self.cancellation_token.cancel();
        self.progress.iter_mut().for_each(|it| {
            if it.status == TransferStatus::InProgress || it.status == TransferStatus::Pending {
                it.status = TransferStatus::Canceled;
            }
        });
    }

    pub fn get_next_transfer_resource(&self) -> Option<&LocalResource> {
        self.resources.iter().find(|resource| {
            self.progress
                .iter()
                .find(|it| it.resource_order_id == resource.order_id)
                .expect("Resource missing progress")
                .status ==
                TransferStatus::Pending
        })
    }

    pub fn status(&self) -> TransferSessionStatus {
        if let TransferTarget::P2P { connection_state, .. } = &self.target {
            return match connection_state {
                P2PConnectionState::NotConnected => {
                    TransferSessionStatus::Initializing { loading_state: Some("Signalling...".to_owned()), loading_error: None }
                }
                P2PConnectionState::Connecting => {
                    TransferSessionStatus::Initializing { loading_state: Some("Dialing...".to_owned()), loading_error: None }
                }
                P2PConnectionState::Failed(msg) => {
                    if self.resources.is_empty() {
                        TransferSessionStatus::Initializing { loading_state: None, loading_error: Some(msg.clone()) }
                    }
                    else {
                        TransferSessionStatus::Failed(msg.clone())
                    }
                }
                P2PConnectionState::Connected => {
                    if self.resources.is_empty() {
                        return TransferSessionStatus::Initializing { loading_state: Some("Waiting for resources...".to_owned()), loading_error: None };
                    }

                    TransferSessionStatus::Success
                }
            }
        }

        if self.is_initializing() {
            return TransferSessionStatus::Initializing { loading_state: Some("Waiting for resources...".to_owned()), loading_error: None };
        }

        let failed_messages = self
            .progress
            .iter()
            .filter_map(|it| match &it.status {
                TransferStatus::Fail(msg) => Some(msg.clone()),
                _ => None
            })
            .collect::<Vec<String>>();

        if !failed_messages.is_empty() {
            return TransferSessionStatus::Failed(failed_messages.join(", "));
        }

        let is_success = self.progress.iter().all(|it| it.is_success());
        if is_success {
            return TransferSessionStatus::Success;
        }

        if self.cancellation_token.is_cancelled() {
            return TransferSessionStatus::Canceled;
        }

        let is_canceled = self.progress.iter().any(|it| it.status == TransferStatus::Canceled);
        if is_canceled {
            self.cancellation_token.cancel();
            return TransferSessionStatus::Canceled;
        }

        TransferSessionStatus::InProgress {
            bytes_per_second: self.speed(1000),
            percentage: self.total_progress()
        }
    }

    pub fn resource_progress(&self, resource_id: u64) -> Option<&TransferProgress> {
        self.progress.iter().find(|it| it.resource_order_id == resource_id)
    }

    pub fn resource_mut_progress(&mut self, resource_id: u64) -> Option<&mut TransferProgress> {
        self.progress.iter_mut().find(|it| it.resource_order_id == resource_id)
    }

    pub fn resource_mut(&mut self, resource_id: u64) -> Option<&mut LocalResource> {
        self.resources.iter_mut().find(|r| r.order_id == resource_id)
    }

    pub fn remove_resource(&mut self, resource_id: &LocalResourceId) {
        self.resources.retain(|r| !resource_id.is_represent(r));
    }

    pub fn token(&self) -> &CancellationToken {
        if self.is_canceled() {
            self.cancellation_token.cancel();
        }

        &self.cancellation_token
    }
}

impl UpdateAction<TransferSession> for TransferProgress {
    fn update(self, data: &mut TransferSession) {
        data.update_progress(self);
    }
}

impl UpdateAction<TransferSession> for LocalResource {
    fn update(self, data: &mut TransferSession) {
        data.replace_resource(self);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThumbnailUpdatedEvent {
    pub resource_id: u64,
    pub path: LocalResourcePath
}

impl UpdateAction<TransferSession> for ThumbnailUpdatedEvent {
    fn update(self, data: &mut TransferSession) {
        if let Some(resource) = data.resources.iter_mut().find(|r| r.order_id == self.resource_id) {
            resource.thumbnail_path = Some(self.path);
        }
    }
}

impl UpdateAction<TransferSession> for P2pTransferSessionMessage {
    fn update(self, data: &mut TransferSession) {
        log::info!(
            "Updated session {} with description={:?}, password_protected={}",
            data.order_id,
            self.description,
            self.password_protected
        );

        data.description = self.description;
        data.is_required_password = self.password_protected;
        let TransferTarget::P2P { connection_state, .. } = &mut data.target else {
            return;
        };

        *connection_state = P2PConnectionState::Connected;
    }
}

impl UpdateAction<TransferSession> for SessionLoadError {
    fn update(self, data: &mut TransferSession) {
        let TransferTarget::P2P { connection_state, .. } = &mut data.target else {
            data.force_complete(self.0);
            return;
        };

        *connection_state = P2PConnectionState::Failed(self.0);
    }
}
