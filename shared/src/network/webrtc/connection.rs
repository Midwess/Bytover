use std::sync::Arc;

use schema::devlog::rpc_signalling::server::{AnswerMessage, IceCandidate, IceCandidateUpdateMessage, Message, OfferMessage};
use thiserror::Error;
use tokio::sync::Mutex;
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
    SignallingServerError(#[from] RtcSignallingErrors)
}

pub struct ConnectionWebRtc {
    pub id: String,
    pub peer_id: String,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub data_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    pub signalling_client: Arc<RtcsSignalling>,
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

        let peer_connection = api.new_peer_connection(Self::create_config()).await.unwrap();
        let data_channel = peer_connection.create_data_channel("data", None).await.unwrap();
        let ns2 = ns.clone();
        data_channel.on_open(Box::new(move || {
            Box::pin(async move {
                log::info!(target: ns2.as_str(), "Data channel opened");
            })
        }));

        let offer = peer_connection.create_offer(None).await.unwrap();
        peer_connection.set_local_description(offer.clone()).await.unwrap();

        let ns = ns.clone();
        let me = Self {
            id,
            peer_id,
            peer_connection: Arc::new(peer_connection),
            data_channel: Arc::new(Mutex::new(Some(data_channel))),
            signalling_client,
            ns
        };

        me.handle_signalling_message();

        me.handle_ice_candidate();

        me.signalling_client
            .send(Message {
                from_id: me.id.clone(),
                to_id: Some(me.peer_id.clone()),
                offer: Some(OfferMessage { sdp: offer.sdp.clone() }),
                ..Default::default()
            })
            .await?;

        Ok(me)
    }

    pub async fn remote(id: String, peer_id: String, offer: RTCSessionDescription, signalling_client: Arc<RtcsSignalling>) -> Result<Self, ConnectionWebRtcErrors> {
        let ns = format!("rtc-m{}-p{}", id, peer_id);
        log::info!(target: ns.as_str(), "Creating remote connection to peer {}", peer_id);
        let api = APIBuilder::new().build();

        let peer_connection = api.new_peer_connection(Self::create_config()).await.unwrap();
        peer_connection.set_remote_description(offer).await.unwrap();

        let answer = peer_connection.create_answer(None).await.unwrap();
 
        peer_connection.set_local_description(answer.clone()).await.unwrap();

        let data_channel_store = Arc::new(Mutex::new(None));

        {
            let ns = ns.clone();
            let data_channel_store = data_channel_store.clone();
            peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
                let data_channel_store = data_channel_store.clone();
                Box::pin(async move {
                    let mut data_channel_store = data_channel_store.lock().await;
                    *data_channel_store = Some(d);
                })
            }));
        }

        let me = Self {
            id,
            peer_id,
            peer_connection: Arc::new(peer_connection),
            data_channel: data_channel_store,
            signalling_client,
            ns
        };

        me.handle_signalling_message();

        me.handle_ice_candidate();

        me.signalling_client
            .send(Message {
                from_id: me.id.clone(),
                to_id: Some(me.peer_id.clone()),
                answer: Some(AnswerMessage { sdp: answer.sdp.clone() }),
                ..Default::default()
            })
            .await?;

        Ok(me)
    }

    pub fn handle_signalling_message(&self) {
        let peer_connection = self.peer_connection.clone();
        let my_id = self.id.clone();
        self.signalling_client.subscribe(Box::new(move |msg| {
            let my_id = my_id.clone();
            let peer_connection = peer_connection.clone();
            Box::pin(async move {
                if msg.from_id == my_id {
                    return;
                }

                if let Some(candidate) = msg.ice_candidate_update {
                    log::info!(target: "rtc", "Adding ice candidate from {}", msg.from_id);
                    let result = peer_connection
                        .add_ice_candidate(BroadcastWebRtc::parse_ice_candidate(
                            msg.from_id.clone(),
                            candidate.ice_candidates
                        ))
                        .await;

                    if let Err(e) = result {
                        log::error!(target: "rtc", "Error adding ice candidate: {:?}", e);
                    }

                    return;
                }

                if let Some(answer) = msg.answer {
                    if msg.to_id.is_some_and(|id| id == my_id) {
                        log::info!(target: "rtc", "Setting remote description from {}", msg.from_id);
                        let result = peer_connection
                            .set_remote_description(RTCSessionDescription::answer(answer.sdp).expect("Answer sdb is wrong"))
                            .await;

                        if let Err(e) = result {
                            log::error!(target: "rtc", "Error setting remote description: {:?}", e);
                        }
                    }
                }
            })
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
