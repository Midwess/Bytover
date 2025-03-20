use std::sync::Arc;
use std::time::Duration;

use schema::devlog::rpc_signalling::server::{AnswerMessage, IceCandidate, IceCandidateUpdateMessage, Message, OfferMessage};
use thiserror::Error;
use tokio::spawn;
use tokio::sync::{mpsc, oneshot, Mutex, OnceCell};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use webrtc::api::APIBuilder;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

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
    pub id: String,
    pub peer_id: String,
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
    pub async fn local(id: String, peer_id: String, signalling_client: Arc<RtcsSignalling>) -> Result<Self, ConnectionWebRtcErrors> {
        let ns = format!("rtc-m{}-p{}", id, peer_id);
        log::info!(target: ns.as_str(), "Creating local connection to peer {}", peer_id);
        let api = APIBuilder::new().build();

        let (notified_data_channel, mut data_channel_receiver) = mpsc::channel(1);
        let peer_connection = api.new_peer_connection(Self::create_config()).await?;
        {
            let data_channel = peer_connection.create_data_channel("data", None).await?;
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
            peer_connection: Arc::new(peer_connection),
            data_channel: OnceCell::new(),
            signalling_client,
            signalling_join_handle: Arc::new(Mutex::new(None)),
            ns: ns.clone()
        };
        
        me.handle_ice_candidate();

        log::info!(target: ns.as_str(), "Sending offer to signalling server");
        let _ = spawn({
            let to_id = me.peer_id.clone();
            let from_id = me.id.clone();
            let signalling_client = me.signalling_client.clone();
            let ns = ns.clone();
            async move {
                if let Err(e) = signalling_client
                    .send(Message {
                        from_id,
                        to_id: Some(to_id),
                        offer: Some(OfferMessage { sdp: offer.sdp.clone() }),
                        ..Default::default()
                    })
                    .await {
                        log::error!(target: ns.as_str(), "Failed to send offer: {:?}", e);
                    }
            }
        }); 

        me.handle_signalling_message().await;

        let clock = Instant::now();
        let connection_timeout = Duration::from_secs(50);
        let mut signalling_subscription = me.signalling_client.subscribe();
        log::info!(target: ns.as_str(), "Waiting for answer from signalling server");

        let mut answer_received = false;
        while let Ok(msg) = signalling_subscription.recv().await {
            if clock.elapsed() > connection_timeout {
                log::error!(target: ns.as_str(), "Connection timed out waiting for answer");
                return Err(ConnectionWebRtcErrors::ConnectionTimeout);
            }

            if let Some(answer) = msg.answer {
                if msg.to_id.is_some_and(|id| id == me.id) && msg.from_id == me.peer_id {
                    log::info!(target: ns.as_str(), "Setting remote description from {}", msg.from_id);
                    if let Err(e) = me.peer_connection.set_remote_description(
                        RTCSessionDescription::answer(answer.sdp).map_err(|e| {
                            log::error!(target: ns.as_str(), "Invalid answer SDP: {:?}", e);
                            ConnectionWebRtcErrors::WebRTCServerError(e)
                        })?
                    ).await {
                        log::error!(target: ns.as_str(), "Failed to set remote description: {:?}", e);
                        return Err(ConnectionWebRtcErrors::WebRTCServerError(e));
                    }
                    answer_received = true;
                    break;
                }
            }
        }

        if !answer_received {
            log::error!(target: ns.as_str(), "No answer received within timeout");
            return Err(ConnectionWebRtcErrors::ConnectionTimeout);
        }

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

    pub async fn remote(id: String, peer_id: String, offer: RTCSessionDescription, signalling_client: Arc<RtcsSignalling>) -> Result<Self, ConnectionWebRtcErrors> {
        let ns = format!("rtc-m{}-p{}", id, peer_id);
        let clock = Instant::now();
        log::info!(target: ns.as_str(), "Creating remote connection to peer {}", peer_id);
        let api = APIBuilder::new().build();

        let peer_connection = api.new_peer_connection(Self::create_config()).await.unwrap();
        peer_connection.set_remote_description(offer).await.unwrap();

        let answer = peer_connection.create_answer(None).await.unwrap();
 
        peer_connection.set_local_description(answer.clone()).await.unwrap();

        let (notified_data_channel, mut data_channel_receiver) = mpsc::channel(1);
        {
            peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
                let notified_data_channel = notified_data_channel.clone();
                let connection = d.clone();
                d.on_open(Box::new(move || {
                    log::info!(target: "tiendang-debug", "Data channel opened");
                    Box::pin(async move {
                        let _ = notified_data_channel.send(connection).await;
                    })
                }));

                Box::pin(async move {})
            }));
        }

        let _ = spawn({
            let signalling_client: Arc<RtcsSignalling> = signalling_client.clone();
            let id = id.clone();
            let peer_id = peer_id.clone();
            log::info!(target: ns.as_str(), "Sending answer to signalling server");
            async move {
                let _ = signalling_client
                    .send(Message {
                        from_id: id,
                        to_id: Some(peer_id),
                        answer: Some(AnswerMessage { sdp: answer.sdp.clone() }),
                        ..Default::default()
                    })
                    .await;
            }
        });

        let me = Self {
            id,
            peer_id,
            signalling_client,
            signalling_join_handle: Arc::new(Mutex::new(None)),
            peer_connection: Arc::new(peer_connection),
            data_channel: OnceCell::new(),
            ns: ns.clone()
        };

        me.handle_ice_candidate(); 

        let connection_timeout = Duration::from_secs(35);
        
        me.handle_signalling_message().await;

        match tokio::time::timeout(connection_timeout, data_channel_receiver.recv()).await {
            Ok(Some(data_channel)) => {
                let _ = me.data_channel.set(data_channel);
            }
            _ => {
                log::error!(target: ns.as_str(), "Connection timed out");
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
        let my_id = self.id.clone();
        let peer_connection = peer_connection.clone();
        let mut signalling_subscription = self.signalling_client.subscribe();
        *signalling_join_handle = Some(tokio::spawn(async move {
            while let Ok(msg) = signalling_subscription.recv().await {
                if msg.from_id == my_id || msg.to_id.is_some_and(|id| id != my_id) {
                    return;
                }
                
                if let Some(candidate) = msg.ice_candidate_update {
                    log::info!(target: "rtc", "Adding ice candidate from {}", msg.from_id);
                    let result = peer_connection
                        .add_ice_candidate(BroadcastWebRtc::parse_ice_candidate(
                            candidate.ice_candidates
                        ))
                        .await;

                    if let Err(e) = result {
                        log::error!(target: "rtc", "Error adding ice candidate: {:?}", e);
                    }

                    return;
                }
            }
        }));
    }

    pub fn handle_ice_candidate(&self) {
        let signaling_client = self.signalling_client.clone();
        let my_id = self.id.clone();
        let peer_id = self.peer_id.clone();

        self.peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let signaling_client = signaling_client.clone();
            let my_id = my_id.clone();
            let peer_id = peer_id.clone();

            Box::pin(async move {
                if let Some(candidate) = candidate {
                    log::info!(target: "rtc", "Sending ice candidate to {}", peer_id);
                    let result = signaling_client.send(Message {
                        from_id: my_id.clone(),
                        to_id: Some(peer_id.clone()),
                        ice_candidate_update: Some(IceCandidateUpdateMessage { ice_candidates: Self::build_ice_candidate_message(candidate) }),
                        ..Default::default()
                    }).await;

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
            sdp_mline_index: candidate_init.sdp_mline_index.unwrap_or_default() as i32,
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
            ..Default::default()
        }
    }
}

impl Drop for ConnectionWebRtc {
    fn drop(&mut self) {
        let mut signalling_join_handle = self.signalling_join_handle.clone();
        spawn(async move {
            if let Some(join_handle) = signalling_join_handle.lock().await.take() {
                join_handle.abort();
            }
        });
    }
}
