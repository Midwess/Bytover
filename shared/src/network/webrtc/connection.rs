use std::{sync::Arc, time::{self, Duration}};

use schema::devlog::rpc_signalling::server::{AnswerMessage, IceCandidate, Message, OfferMessage};
use thiserror::Error;
use tokio::{spawn, sync::{Mutex, OnceCell}, task::JoinHandle};
use webrtc::{api::APIBuilder, data_channel::RTCDataChannel, ice_transport::{ice_candidate::{RTCIceCandidate, RTCIceCandidateInit}, ice_candidate_type::RTCIceCandidateType, ice_protocol::RTCIceProtocol, ice_server::RTCIceServer}, peer_connection::{self, configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription, RTCPeerConnection}, turn::proto::data};

use super::{broadcast::BroadcastWebRtc, signalling::RtcsSignalling};

#[derive(Debug, Error)]
pub enum ConnectionWebRtcErrors {
    #[error("failedServerError to create peer connection {:?}", .0)]
    WebRTCServerError(#[from] webrtc::Error),
}

pub struct ConnectionWebRtc {
    pub id: String,
    pub peer_id: String,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub data_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    pub signalling_client: Arc<RtcsSignalling>,
    pub ns: String,
}

impl PartialEq for ConnectionWebRtc {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.peer_id == other.peer_id
    }
}

impl ConnectionWebRtc {
    pub async fn local(
        id: String,
        peer_id: String,
        signalling_client: Arc<RtcsSignalling>,
    ) -> Self {
        let ns = format!("rtc-m{}-p{}", id, peer_id);
        log::info!(target: ns.as_str(), "Creating local connection to peer {}", peer_id);
        let api = APIBuilder::new()
            .build();

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

        // TODO: Send offer to peer using signalling server
        signalling_client.send(Message {
            from_id: id.clone(),
            to_id: Some(peer_id.clone()),
            offer: Some(OfferMessage { sdp: offer.sdp.clone() }),
            ..Default::default()
        }).unwrap();

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

        me
    }

    pub async fn remote(
        id: String,
        peer_id: String,
        offer: RTCSessionDescription,
        signalling_client: Arc<RtcsSignalling>,
    ) -> Self {
        let ns = format!("rtc-m{}-p{}", id, peer_id);
        log::info!(target: ns.as_str(), "Creating remote connection to peer {}", peer_id);
        let api = APIBuilder::new()
           .build();

        let peer_connection = api.new_peer_connection(Self::create_config()).await.unwrap();
        peer_connection.set_remote_description(offer).await.unwrap();

        let answer = peer_connection.create_answer(None).await.unwrap();

        log::info!(target: ns.as_str(), "Sending answer to peer {}", peer_id);
        signalling_client.send(Message {
            from_id: id.clone(),
            to_id: Some(peer_id.clone()),
            answer: Some(AnswerMessage { sdp: answer.sdp.clone() }),
            ..Default::default()
        }).unwrap();

        peer_connection.set_local_description(answer).await.unwrap();

        let ns = ns.clone();
       
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
            ns,
        };

        me.handle_signalling_message();

        me
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
                    peer_connection.add_ice_candidate(BroadcastWebRtc::parse_ice_candidate(
                        msg.from_id.clone(),
                        candidate.ice_candidates
                    )).await.unwrap();

                    return;
                }

                if let Some(answer) = msg.answer {
                    if msg.to_id.map_or(false, |id| id == my_id) {
                        log::info!(target: "rtc", "Setting remote description from {}", msg.from_id);
                        peer_connection.set_remote_description(RTCSessionDescription::answer(answer.sdp).expect("Answer sdb is wrong")).await.unwrap();
                    }

                    return;
                }
            })
        }));
    }

    pub fn create_config() -> RTCConfiguration {
        RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec![
                    "stun:stun.l.google.com:19302".to_string(),
                ],
                ..Default::default()
            }],
            ..Default::default()
        }
    }
    
}
