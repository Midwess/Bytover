use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use schema::devlog::rpc_signalling::server::{AnswerMessage, IceCandidate, IceCandidateUpdateMessage, Message, OfferMessage};
use thiserror::Error;
use tokio::spawn;
use tokio::sync::{mpsc, Mutex, OnceCell};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::{OnCloseHdlrFn, RTCDataChannel};
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use core_services::db::repository::abstraction::repository::Repository;
use crate::network::webrtc::message_channel::MessageChannel;
use crate::network::webrtc::peer::PeerCommunication;
use crate::ShellRuntime;
use shared::app::nearby::finding_scope::FindingScope;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::entities::peer::Peer;

use super::peer::PeerErrors;
use super::signalling::{RtcSignalling, RtcSignallingErrors};
use super::throughput::ThroughputController;

#[derive(Debug, Error)]
pub enum ConnectionWebRtcErrors {
    #[error("failedServerError to create peer connection {:?}", .0)]
    WebRTCServerError(#[from] webrtc::Error),
    #[error("failed to send message to signalling server {:?}", .0)]
    SignallingServerError(#[from] RtcSignallingErrors),
    #[error("connection timed out")]
    ConnectionTimeout,
    #[error("failed to encode message {:?}", .0)]
    EncodeError(#[from] prost::EncodeError),
    #[error("Connection corrupted, should consider to close")]
    ConnectionCorrupted,
    #[error("failed to parse response {:?}", .0)]
    ParseError(String),
    #[error("Send request timeout")]
    SendTimeout(tokio::time::error::Elapsed),
    #[error("Receive request timeout")]
    ReceiveTimeout(tokio::time::error::Elapsed),
    #[error("Connection not found")]
    ConnectionNotFound,
    #[error("Upgrade to peer communication, will retry it a few secs")]
    UpgradeError(#[from] Box<PeerErrors>)
}

pub struct ConnectionWebRtc {
    pub current: Peer,
    pub peer_id: String,
    pub finding_scope: FindingScope,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub msg_channel: OnceCell<MessageChannel>,
    pub signalling_client: Arc<RtcSignalling>,
    pub signalling_join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub on_disconnect: OnceCell<Arc<Mutex<OnCloseHdlrFn>>>,
    pub repository: Arc<dyn LocalResourceRepository>,
    pub ns: String,
    pub throughput_controller: Arc<ThroughputController>
}

impl PartialEq for ConnectionWebRtc {
    fn eq(&self, other: &Self) -> bool {
        self.current.id() == other.current.id() && self.peer_id == other.peer_id
    }
}

impl ConnectionWebRtc {
    pub fn channel_config() -> RTCDataChannelInit {
        RTCDataChannelInit {
            ordered: Some(true),
            max_retransmits: Some(3),
            ..Default::default()
        }
    }

    pub fn setting_engine() -> SettingEngine {
        let mut setting_engine = webrtc::api::setting_engine::SettingEngine::default();

        setting_engine.set_ice_timeouts(
            Some(Duration::from_secs(5)),
            Some(Duration::from_secs(15)),
            Some(Duration::from_secs(2))
        );

        setting_engine
    }

    pub fn id(&self) -> String {
        self.current.id()
    }

    pub async fn offer(
        scope: FindingScope,
        current: Peer,
        peer_id: String,
        signalling_client: Arc<RtcSignalling>,
        shell_runtime: Arc<dyn ShellRuntime>,
        throughput_controller: Arc<ThroughputController>,
        repository: Arc<dyn LocalResourceRepository>,
    ) -> Result<Arc<PeerCommunication>, ConnectionWebRtcErrors> {
        let my_id = current.id();
        let ns = format!("rtc-m{my_id}-p{peer_id}");
        log::info!(target: ns.as_str(), "Offering connection to peer {peer_id}");
        let api = APIBuilder::new().with_setting_engine(Self::setting_engine()).build();

        let (notified_msg_channel_ready, mut msg_channel_receiver) = mpsc::channel(1);
        let peer_connection = api.new_peer_connection(Self::create_config()).await?;
        let msg_channel = peer_connection.create_data_channel("message-channel", Some(Self::channel_config())).await?;

        let throughput_controller_cloned = throughput_controller.clone();
        msg_channel.clone().on_open(Box::new(move || {
            let throughput_controller_cloned = throughput_controller_cloned.clone();
            Box::pin(async move {
                let _ = notified_msg_channel_ready
                    .send(MessageChannel::new(msg_channel, throughput_controller_cloned).await)
                    .await;
            })
        }));

        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer.clone()).await?;

        let me = Self {
            current: current.clone(),
            peer_id: peer_id.clone(),
            finding_scope: scope,
            peer_connection: Arc::new(peer_connection),
            msg_channel: OnceCell::new(),
            signalling_client,
            signalling_join_handle: Arc::new(Mutex::new(None)),
            on_disconnect: OnceCell::new(),
            ns: ns.clone(),
            repository: repository.clone(),
            throughput_controller
        };

        log::info!(target: ns.as_str(), "Sending offer to signalling server");

        me.handle_signalling_message().await;

        let _ = spawn({
            let to_id = me.peer_id.clone();
            let from_id = me.id();
            let scope = me.finding_scope.clone();
            let signalling_client = me.signalling_client.clone();
            let ns = ns.clone();
            async move {
                if let Err(e) = signalling_client
                    .send(Message {
                        scopes: vec![scope.as_string()],
                        from_id: from_id.to_string(),
                        to_id: Some(to_id.to_string()),
                        offer: Some(OfferMessage { sdp: offer.sdp.clone() }),
                        ..Default::default()
                    })
                    .await
                {
                    log::error!(target: ns.as_str(), "Failed to send offer: {e:?}");
                }
            }
        });

        let connection_timeout = Duration::from_secs(10);
        log::info!(target: ns.as_str(), "Waiting for answer from signalling server");

        me.handle_ice_candidate();

        let peer_id = peer_id.clone();
        let repository = repository.clone();
        match tokio::time::timeout(connection_timeout, msg_channel_receiver.recv()).await {
            Ok(Some(msg_channel)) => {
                let _ = me.msg_channel.set(msg_channel);
                let throughput_controller = me.throughput_controller.clone();
                Ok(
                    PeerCommunication::upgrade(repository, me, current, peer_id.clone(), shell_runtime, throughput_controller)
                        .await
                        .unwrap()
                )
            }
            _ => {
                log::error!(target: ns.as_str(), "Data channel connection timed out");
                Err(ConnectionWebRtcErrors::ConnectionTimeout)
            }
        }
    }

    pub async fn accept_offer(
        scope: FindingScope,
        current: Peer,
        peer_id: String,
        offer: RTCSessionDescription,
        signalling_client: Arc<RtcSignalling>,
        shell_runtime: Arc<dyn ShellRuntime>,
        throughput_controller: Arc<ThroughputController>,
        repository: Arc<dyn LocalResourceRepository>,
    ) -> Result<Arc<PeerCommunication>, ConnectionWebRtcErrors> {
        let my_id = current.id();
        let ns = format!("rtc-m{my_id}-p{peer_id}");
        log::info!(target: ns.as_str(), "Accepting offer from peer {peer_id}");
        let api = APIBuilder::new().with_setting_engine(Self::setting_engine()).build();

        let peer_connection = api.new_peer_connection(Self::create_config()).await?;
        if let Err(e) = peer_connection.set_remote_description(offer).await {
            log::error!(target: ns.as_str(), "Failed to set remote description: {e:?}");
            return Err(ConnectionWebRtcErrors::WebRTCServerError(e));
        }

        let answer = peer_connection.create_answer(None).await?;

        if let Err(e) = peer_connection.set_local_description(answer.clone()).await {
            log::error!(target: ns.as_str(), "Failed to set local description: {e:?}");
            return Err(ConnectionWebRtcErrors::WebRTCServerError(e));
        }

        let (notified_msg_channel_ready, mut msg_channel_receiver) = mpsc::channel(1);
        let throughput_controller_cloned = throughput_controller.clone();
        peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
            let notified_msg_channel_ready = notified_msg_channel_ready.clone();
            let connection = d.clone();
            let throughput_controller_cloned = throughput_controller_cloned.clone();

            Box::pin(async move {
                let _ = notified_msg_channel_ready
                    .send(MessageChannel::new(connection, throughput_controller_cloned).await)
                    .await;
            })
        }));

        let me = Self {
            current: current.clone(),
            peer_id: peer_id.clone(),
            finding_scope: scope,
            signalling_client,
            signalling_join_handle: Arc::new(Mutex::new(None)),
            peer_connection: Arc::new(peer_connection),
            msg_channel: OnceCell::new(),
            ns: ns.clone(),
            on_disconnect: OnceCell::new(),
            repository: repository.clone(),
            throughput_controller
        };

        me.handle_signalling_message().await;

        let _ = spawn({
            let signalling_client: Arc<RtcSignalling> = me.signalling_client.clone();
            let my_id = me.id();
            let peer_id = me.peer_id.clone();
            let ns = ns.clone();
            let scope = me.finding_scope.clone();
            log::info!(target: ns.as_str(), "Sending answer to signalling server");
            async move {
                if let Err(e) = signalling_client
                    .send(Message {
                        scopes: vec![scope.as_string()],
                        from_id: my_id.to_string(),
                        to_id: Some(peer_id.clone().to_string()),
                        answer: Some(AnswerMessage { sdp: answer.sdp.clone() }),
                        ..Default::default()
                    })
                    .await
                {
                    log::error!(target: ns.as_str(), "Failed to send answer: {e:?}");
                }
            }
        })
        .await;

        me.handle_ice_candidate();

        let connection_timeout = Duration::from_secs(10);
        let repository = repository.clone();
        let result = match tokio::time::timeout(connection_timeout, msg_channel_receiver.recv()).await {
            Ok(Some(msg_channel)) => {
                let _ = me.msg_channel.set(msg_channel);
                let throughput_controller = me.throughput_controller.clone();
                Ok(
                    PeerCommunication::upgrade(repository.clone(), me, current, peer_id.clone(), shell_runtime, throughput_controller)
                        .await
                        .map_err(Box::new)?
                )
            }
            Ok(None) => {
                log::error!(target: ns.as_str(), "Data channel receiver closed without receiving data channel");
                Err(ConnectionWebRtcErrors::ConnectionTimeout)
            }
            Err(_) => {
                log::error!(target: ns.as_str(), "Data channel connection timed out");
                Err(ConnectionWebRtcErrors::ConnectionTimeout)
            }
        };

        let Ok(peer_communication) = result else {
            return Err(ConnectionWebRtcErrors::ConnectionTimeout);
        };

        Ok(peer_communication)
    }

    pub async fn handle_signalling_message(&self) {
        let mut signalling_join_handle = self.signalling_join_handle.lock().await;
        if let Some(join_handle) = signalling_join_handle.take() {
            join_handle.abort();
        }

        let peer_connection = self.peer_connection.clone();
        let my_id = self.id();
        let peer_id = self.peer_id.clone();
        let peer_connection = peer_connection.clone();
        let signalling_client = self.signalling_client.clone();
        *signalling_join_handle = Some(tokio::spawn(async move {
            let mut signalling_subscription = signalling_client.subscribe();
            while let Ok(msg) = signalling_subscription.recv().await {
                if msg.from_id != peer_id || msg.to_id.is_some_and(|id| id != my_id) {
                    continue;
                }

                if let Some(answer) = msg.answer {
                    log::info!(target: "rtc", "Setting remote description from {}", msg.from_id);
                    if let Ok(answer) = RTCSessionDescription::answer(answer.sdp) {
                        if let Err(e) = peer_connection.set_remote_description(answer).await {
                            log::error!(target: "rtc", "Invalid answer SDP: {e:?}");
                        }
                    }
                }

                if let Some(candidate) = msg.ice_candidate_update {
                    let result = peer_connection.add_ice_candidate(Self::parse_ice_candidate(candidate.ice_candidates)).await;

                    if let Err(e) = result {
                        log::error!(target: "rtc", "Error adding ice candidate: {e:?}");
                    }
                }
            }
        }));
    }

    pub fn handle_ice_candidate(&self) {
        let signaling_client = self.signalling_client.clone();
        let my_id = self.id();
        let peer_id = self.peer_id.clone();
        let scope = self.finding_scope.clone();

        self.peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let signaling_client = signaling_client.clone();
            let my_id = my_id.clone();
            let peer_id = peer_id.clone();
            let finding_scope = scope.clone();

            Box::pin(async move {
                if let Some(candidate) = candidate {
                    let ice_candidate = Self::build_ice_candidate_message(candidate);

                    if finding_scope.is_local_network_only() && ice_candidate.is_public() {
                        return;
                    }

                    let result = signaling_client
                        .send(Message {
                            scopes: vec![finding_scope.as_string()],
                            from_id: my_id.to_string(),
                            to_id: Some(peer_id.to_string()),
                            ice_candidate_update: Some(IceCandidateUpdateMessage {
                                ice_candidates: ice_candidate
                            }),
                            ..Default::default()
                        })
                        .await;

                    if let Err(e) = result {
                        log::error!(target: "rtc", "Error sending ice candidate: {e:?}");
                    }
                }
            })
        }));
    }

    pub fn build_ice_candidate_message(candidate: RTCIceCandidate) -> IceCandidate {
        let candidate_init = candidate.to_json().unwrap();

        IceCandidate {
            candidate: candidate_init.candidate,
            sdp_mid: candidate_init.sdp_mid.unwrap_or_default().to_string(),
            sdp_mline_index: candidate_init.sdp_mline_index.unwrap_or_default() as i32
        }
    }

    pub fn create_config() -> RTCConfiguration {
        RTCConfiguration {
            ice_servers: vec![RTCIceServer { ..Default::default() }],
            ..Default::default()
        }
    }

    pub fn on_disconnect(&self, callback: OnCloseHdlrFn) {
        let callback = Arc::new(Mutex::new(callback));
        let _ = self.on_disconnect.set(callback.clone());
        let msg_channel = self.msg_channel.get().cloned();
        self.peer_connection.on_peer_connection_state_change(Box::new(move |state| {
            let callback = callback.clone();
            let msg_channel = msg_channel.clone();
            Box::pin(async move {
                if state == RTCPeerConnectionState::Failed || state == RTCPeerConnectionState::Closed {
                    if let Some(msg_channel) = msg_channel {
                        let _ = msg_channel.close().await;
                    }

                    let mut callback = callback.lock().await;
                    log::info!(target: "rtc", "Peer connection closed {state:?}");
                    callback().await;
                }
            })
        }));
    }

    pub fn parse_ice_candidate(candidate: IceCandidate) -> RTCIceCandidateInit {
        // Parse the candidate string to extract needed information
        RTCIceCandidateInit {
            candidate: candidate.candidate,
            sdp_mid: Some(candidate.sdp_mid),
            sdp_mline_index: Some(candidate.sdp_mline_index as u16),
            username_fragment: None
        }
    }

    pub async fn close(&self) {
        if self.peer_connection.connection_state() == RTCPeerConnectionState::Closed {
            log::warn!(target: "peer", "The peer connection is already closed");
            return;
        }

        if let Some(msg_channel) = self.msg_channel.get() {
            msg_channel.close().await;
        }

        let peer_connection = self.peer_connection.clone();
        // Webrtc having a bug that cause the close connection hangup
        // we need to spawn a task to allow the current connection to be dropped properly

        let result = timeout(Duration::from_secs(3), peer_connection.close()).await;
        if result.is_err() {
            if let Some(callback) = self.on_disconnect.get().cloned() {
                let _ = callback.lock().await.as_mut()().await;
            }
        }

        log::info!(target: "peer", "The peer connection is closed with closing process status = {result:?}");
    }
}

impl Deref for ConnectionWebRtc {
    type Target = MessageChannel;

    fn deref(&self) -> &Self::Target {
        self.msg_channel.get().expect("Message channel not set")
    }
}

impl Drop for ConnectionWebRtc {
    fn drop(&mut self) {
        let signalling_join_handle = self.signalling_join_handle.clone();
        let connection = self.peer_connection.clone();
        let mut on_disconnect = self.on_disconnect.get().cloned();
        spawn(async move {
            let result = timeout(Duration::from_secs(3), connection.close()).await;
            log::info!(target: "rtc", "The peer connection is closed with closing process status = {result:?}");

            if let Some(join_handle) = signalling_join_handle.lock().await.take() {
                join_handle.abort();
            }

            if let Some(callback) = on_disconnect.take() {
                let _ = callback.lock().await.as_mut()().await;
            }
        });
    }
}
