use std::sync::Arc;
use std::time::Duration;

use schema::devlog::rpc_signalling::server::{AnswerMessage, IceCandidate, IceCandidateUpdateMessage, Message, OfferMessage};
use thiserror::Error;
use tokio::spawn;
use tokio::sync::{mpsc, Mutex, OnceCell};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::{OnCloseHdlrFn, RTCDataChannel};
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

use crate::app::transfer::finding_scope::FindingScope;

use super::broadcast::BroadcastWebRtc;
use super::signalling::{RtcSignallingErrors, RtcsSignalling};

#[derive(Debug, Error)]
pub enum ConnectionWebRtcErrors {
    #[error("failedServerError to create peer connection {:?}", .0)]
    WebRTCServerError(#[from] webrtc::Error),
    #[error("failed to send message to signalling server {:?}", .0)]
    SignallingServerError(#[from] RtcSignallingErrors),
    #[error("connection timed out")]
    ConnectionTimeout
}

pub struct ConnectionWebRtc {
    pub id: u128,
    pub peer_id: u128,
    pub finding_scope: FindingScope,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub data_channel: OnceCell<Arc<RTCDataChannel>>,
    pub signalling_client: Arc<RtcsSignalling>,
    pub signalling_join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    pub ns: String
}

impl PartialEq for ConnectionWebRtc {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.peer_id == other.peer_id
    }
}

impl ConnectionWebRtc {
    pub fn channel_config() -> RTCDataChannelInit {
        RTCDataChannelInit {
            ordered: Some(true),
            max_packet_life_time: None,
            max_retransmits: None,
            protocol: None,
            negotiated: None
        }
    }

    // This configuration is much longer than the default ones
    // I assume that, this configuration is more suitable for rule of battery life on Android and iOS
    pub fn setting_engine() -> SettingEngine {
        let mut setting_engine = webrtc::api::setting_engine::SettingEngine::default();
        setting_engine.set_ice_timeouts(
            Some(Duration::from_secs(40)),
            Some(Duration::from_secs(120)),
            Some(Duration::from_secs(15))
        );

        setting_engine
    }

    pub async fn offer(
        scope: FindingScope,
        id: u128,
        peer_id: u128,
        signalling_client: Arc<RtcsSignalling>
    ) -> Result<Self, ConnectionWebRtcErrors> {
        let ns = format!("rtc-m{}-p{}", id, peer_id);
        let clock = Instant::now();
        log::info!(target: ns.as_str(), "Offering connection to peer {}", peer_id);
        let setting_engine = Self::setting_engine();
        let api = APIBuilder::new().with_setting_engine(setting_engine).build();

        let (notified_data_channel, mut data_channel_receiver) = mpsc::channel(1);
        let peer_connection = api.new_peer_connection(Self::create_config()).await?;
        {
            let data_channel = peer_connection.create_data_channel("data", Some(Self::channel_config())).await?;
            data_channel.clone().on_open(Box::new(move || {
                Box::pin(async move {
                    let _ = notified_data_channel.send(data_channel).await;
                })
            }));
        }

        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer.clone()).await?;

        let me = Self {
            id,
            peer_id,
            finding_scope: scope,
            peer_connection: Arc::new(peer_connection),
            data_channel: OnceCell::new(),
            signalling_client,
            signalling_join_handle: Arc::new(Mutex::new(None)),
            ns: ns.clone()
        };

        log::info!(target: ns.as_str(), "Sending offer to signalling server");

        me.handle_signalling_message().await;

        let _ = spawn({
            let to_id = me.peer_id;
            let from_id = me.id;
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
                    log::error!(target: ns.as_str(), "Failed to send offer: {:?}", e);
                }
            }
        });

        let connection_timeout = Duration::from_secs(15);
        log::info!(target: ns.as_str(), "Waiting for answer from signalling server");

        me.handle_ice_candidate();

        match tokio::time::timeout(connection_timeout, data_channel_receiver.recv()).await {
            Ok(Some(data_channel)) => {
                let _ = me.data_channel.set(data_channel);
            }
            _ => {
                log::error!(target: ns.as_str(), "Data channel connection timed out");
                return Err(ConnectionWebRtcErrors::ConnectionTimeout);
            }
        }

        log::info!(target: ns.as_str(), "Connection established in {:?}", clock.elapsed().as_secs_f32());
        Ok(me)
    }

    pub async fn accept_offer(
        scope: FindingScope,
        id: u128,
        peer_id: u128,
        offer: RTCSessionDescription,
        signalling_client: Arc<RtcsSignalling>
    ) -> Result<Self, ConnectionWebRtcErrors> {
        let ns = format!("rtc-m{}-p{}", id, peer_id);
        let clock = Instant::now();
        log::info!(target: ns.as_str(), "Accepting offer from peer {}", peer_id);
        let setting_engine = Self::setting_engine();

        let api = APIBuilder::new().with_setting_engine(setting_engine).build();

        let peer_connection = api.new_peer_connection(Self::create_config()).await?;

        if let Err(e) = peer_connection.set_remote_description(offer).await {
            log::error!(target: ns.as_str(), "Failed to set remote description: {:?}", e);
            return Err(ConnectionWebRtcErrors::WebRTCServerError(e));
        }

        let answer = peer_connection.create_answer(None).await?;

        if let Err(e) = peer_connection.set_local_description(answer.clone()).await {
            log::error!(target: ns.as_str(), "Failed to set local description: {:?}", e);
            return Err(ConnectionWebRtcErrors::WebRTCServerError(e));
        }

        let (notified_data_channel, mut data_channel_receiver) = mpsc::channel(1);
        {
            peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
                let notified_data_channel = notified_data_channel.clone();
                let connection = d.clone();

                d.on_open(Box::new(move || {
                    Box::pin(async move {
                        let _ = notified_data_channel.send(connection).await;
                    })
                }));

                Box::pin(async move {})
            }));
        }

        let me = Self {
            id,
            peer_id,
            finding_scope: scope,
            signalling_client,
            signalling_join_handle: Arc::new(Mutex::new(None)),
            peer_connection: Arc::new(peer_connection),
            data_channel: OnceCell::new(),
            ns: ns.clone()
        };

        me.handle_signalling_message().await;

        let _ = spawn({
            let signalling_client: Arc<RtcsSignalling> = me.signalling_client.clone();
            let id = me.id;
            let peer_id = me.peer_id;
            let ns = ns.clone();
            let scope = me.finding_scope.clone();
            log::info!(target: ns.as_str(), "Sending answer to signalling server");
            async move {
                if let Err(e) = signalling_client
                    .send(Message {
                        scopes: vec![scope.as_string()],
                        from_id: id.to_string(),
                        to_id: Some(peer_id.to_string()),
                        answer: Some(AnswerMessage { sdp: answer.sdp.clone() }),
                        ..Default::default()
                    })
                    .await
                {
                    log::error!(target: ns.as_str(), "Failed to send answer: {:?}", e);
                }
            }
        })
        .await;

        me.handle_ice_candidate();

        let connection_timeout = Duration::from_secs(15);
        match tokio::time::timeout(connection_timeout, data_channel_receiver.recv()).await {
            Ok(Some(data_channel)) => {
                let _ = me.data_channel.set(data_channel);
            }
            Ok(None) => {
                log::error!(target: ns.as_str(), "Data channel receiver closed without receiving data channel");
                return Err(ConnectionWebRtcErrors::ConnectionTimeout);
            }
            Err(_) => {
                log::error!(target: ns.as_str(), "Data channel connection timed out");
                return Err(ConnectionWebRtcErrors::ConnectionTimeout);
            }
        }

        log::info!(target: ns.as_str(), "Connection established in {:?}", clock.elapsed().as_secs_f32());
        Ok(me)
    }

    pub async fn handle_signalling_message(&self) {
        let mut signalling_join_handle = self.signalling_join_handle.lock().await;
        if let Some(join_handle) = signalling_join_handle.take() {
            join_handle.abort();
        }

        let peer_connection = self.peer_connection.clone();
        let my_id = self.id;
        let peer_id = self.peer_id;
        let peer_connection = peer_connection.clone();
        let signalling_client = self.signalling_client.clone();
        *signalling_join_handle = Some(tokio::spawn(async move {
            let mut signalling_subscription = signalling_client.subscribe();
            while let Ok(msg) = signalling_subscription.recv().await {
                if msg.from_id_number() != peer_id || msg.to_id_number().is_some_and(|id| id != my_id) {
                    continue;
                }

                if let Some(answer) = msg.answer {
                    log::info!(target: "rtc", "Setting remote description from {}", msg.from_id);
                    if let Ok(answer) = RTCSessionDescription::answer(answer.sdp) {
                        if let Err(e) = peer_connection.set_remote_description(answer).await {
                            log::error!(target: "rtc", "Invalid answer SDP: {:?}", e);
                        }
                    }
                }

                if let Some(candidate) = msg.ice_candidate_update {
                    let result = peer_connection
                        .add_ice_candidate(BroadcastWebRtc::parse_ice_candidate(candidate.ice_candidates))
                        .await;

                    if let Err(e) = result {
                        log::error!(target: "rtc", "Error adding ice candidate: {:?}", e);
                    }
                }
            }
        }));
    }

    pub fn handle_ice_candidate(&self) {
        let signaling_client = self.signalling_client.clone();
        let my_id = self.id;
        let peer_id = self.peer_id;
        let scope = self.finding_scope.clone();

        self.peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let signaling_client = signaling_client.clone();
            let my_id = my_id;
            let peer_id = peer_id;
            let finding_scope = scope.clone();

            Box::pin(async move {
                if let Some(candidate) = candidate {
                    let ice_candidate = Self::build_ice_candidate_message(candidate);

                    if finding_scope.is_local() && ice_candidate.is_public() {
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
                        log::error!(target: "rtc", "Error sending ice candidate: {:?}", e);
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
            ice_servers: vec![RTCIceServer {
                urls: vec![
                    // IPv4 version
                    "stun:stun.l.google.com:19302".to_string(),
                    // Add Google's IPv4-specific STUN server
                    "stun:stun4.l.google.com:19302".to_string(),
                    // Try an alternative STUN server
                    "stun:stun.stunprotocol.org:3478".to_string(),
                ],
                ..Default::default()
            }],
            ice_candidate_pool_size: 20,
            ice_transport_policy: webrtc::peer_connection::policy::ice_transport_policy::RTCIceTransportPolicy::All,
            ..Default::default()
        }
    }

    pub fn on_disconnect(&self, callback: OnCloseHdlrFn) {
        let callback = Arc::new(Mutex::new(callback));
        {
            let callback = callback.clone();
            self.peer_connection.on_peer_connection_state_change(Box::new(move |state| {
                let callback = callback.clone();
                Box::pin(async move {
                    if state == webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState::Failed {
                        log::info!(target: "rtc", "Peer connection closed {:?}", state);
                        let mut c = callback.lock().await;
                        c.as_mut()().await;
                    }
                })
            }));
        }
    }
}

impl Drop for ConnectionWebRtc {
    fn drop(&mut self) {
        log::info!(target: "rtc", "Dropping connection to peer {}", self.peer_id);
        let signalling_join_handle = self.signalling_join_handle.clone();
        spawn(async move {
            if let Some(join_handle) = signalling_join_handle.lock().await.take() {
                join_handle.abort();
            }
        });
    }
}
