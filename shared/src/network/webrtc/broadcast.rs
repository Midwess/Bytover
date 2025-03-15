use std::{sync::Arc, time::{self, Duration}};

use devlog_sdk::distributed_id::gen_id;
use futures_util::lock::Mutex;
use schema::devlog::rpc_signalling::server::{AnswerMessage, IceCandidate, JoinMessage, Message, OfferMessage};
use thiserror::Error;
use tokio::{spawn, sync::OnceCell, task::JoinHandle};
use webrtc::{api::APIBuilder, ice_transport::{ice_candidate::{RTCIceCandidate, RTCIceCandidateInit}, ice_candidate_type::RTCIceCandidateType, ice_protocol::RTCIceProtocol, ice_server::RTCIceServer}, peer_connection::{self, configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription, RTCPeerConnection}};

use super::{connection::ConnectionWebRtc, signalling::{RtcSignallingErrors, RtcsSignalling}};

#[derive(Debug, Error)]
pub enum BroadcastWebRtcErrors {
    #[error("failedServerError to create peer connection {:?}", .0)]
    WebRTCServerError(#[from] webrtc::Error),
    #[error("failed to connect to signalling server {:?}", .0)]
    SignallingServerError(#[from] RtcSignallingErrors),
}

pub struct BroadcastWebRtc {
    scopes: Vec<String>,
    broadcast_handle: Mutex<Option<JoinHandle<()>>>,
    my_id: String,
    connections: Mutex<Vec<ConnectionWebRtc>>,
    signalling_client: OnceCell<Arc<RtcsSignalling>>,
}

impl BroadcastWebRtc {
    pub fn new(scopes: Vec<String>) -> Self {
        Self {
            scopes,
            broadcast_handle: Mutex::new(None),
            my_id: uuid::Uuid::new_v4().to_string(),
            connections: Mutex::new(Vec::new()),
            signalling_client: OnceCell::new(),
        }
    }
    
    pub async fn start(self: &Arc<Self>) -> Result<(), BroadcastWebRtcErrors> {
        log::info!(target: "broadcast", "Starting broadcast");
        let signalling_client = RtcsSignalling::start().await?;
        let _ = self.signalling_client.set(Arc::new(signalling_client));

        self.broadcast().await?;

        let me = self.clone();
        self.signalling_client.get().unwrap().subscribe(Box::new(move |msg| {
            let me = me.clone();
            Box::pin(async move {
                let _ = me.handle_signalling_message(msg).await;
            })
        }));

        Ok(())
    }

    pub async fn broadcast(&self) -> Result<(), BroadcastWebRtcErrors> {
        let mut broadcast_handle = self.broadcast_handle.lock().await;
        if let Some(handle) = broadcast_handle.take() {
            handle.abort();
        }

        log::info!(target: "broadcast", "Starting broadcast");
        let scopes = self.scopes.clone();
        let my_id = self.my_id.clone();
        let signalling_client = self.signalling_client.clone();
        *broadcast_handle = Some(spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            let scopes = scopes.clone();

            loop {
                interval.tick().await;
                log::info!(target: "broadcast", "Broadcasting...");
                let join_message = JoinMessage {
                    id: my_id.clone(),
                };

                let message = Message {
                    scopes: scopes.clone(),
                    from_id: my_id.clone(),
                    join: Some(join_message),
                    ..Default::default()
                };

                let _ = signalling_client.get().unwrap().send(message);
            }
        }));

        Ok(())
    }

    pub async fn handle_signalling_message(self: &Arc<Self>, message: Message) -> Result<(), BroadcastWebRtcErrors> {
        let my_id = self.my_id.clone();
        if message.from_id == self.my_id {
            log::info!(target: "broadcast", "Received message from myself");
            return Ok(());
        }

        let from_id = message.from_id.clone();
        if let Some(_) = message.join {
            log::info!(target: "broadcast", "Received join message from {}", from_id);
            let mut existing_connection = self.connections.lock().await;
            if existing_connection.iter().any(|connection| connection.peer_id.eq(&from_id)) {
                log::info!(target: "broadcast", "Received join message from {} but already connected", from_id);
                return Ok(());
            }

            let connection = ConnectionWebRtc::local(
                my_id.clone(),
                from_id.clone(),
                self.signalling_client.get().unwrap().clone(),
            ).await;

            existing_connection.push(connection);
        }

        if let Some(offer) = message.offer {
            let mut existing_connection = self.connections.lock().await;
            if existing_connection.iter().any(|connection| connection.peer_id == from_id) {
                return Ok(());
            }

            let desc = RTCSessionDescription::offer(offer.sdp)?;
            let connection = ConnectionWebRtc::remote(
                my_id.clone(),
                from_id,
                desc,
                self.signalling_client.get().unwrap().clone(),
            ).await;

            existing_connection.push(connection);
        }

        Ok(())
    }

    pub fn parse_ice_candidate(peer_id: String, candidate: IceCandidate) -> RTCIceCandidateInit {
        // Parse the candidate string to extract needed information
        RTCIceCandidateInit {
            candidate: candidate.candidate,
            sdp_mid: Some(candidate.sdp_mid),
            sdp_mline_index: Some(candidate.sdp_mline_index as u16),
            username_fragment: None,
        }
    }
}
