use super::errors::WebRtcErrors;
use crate::app::nearby::finding_scope::FindingScope;
use crate::core_transfer_protocol::webrtc::message_channel::DirectMessageChannel;
use crate::core_transfer_protocol::webrtc::peer::WebRtcPeer;
use crate::core_transfer_protocol::webrtc::signalling_client::SignallingClient;
use futures_util::lock::Mutex;
use matchbox_protocol::PeerId;
use matchbox_socket::{PeerEvent, PeerRequest, PeerSignal, SignalingError, Signaller, SignallerBuilder};
use n0_future::time::Instant;
use once_cell::sync::OnceCell;
use schema::devlog::bitbridge::peer_message_body::Response;
use schema::devlog::rpc_signalling::server::{
    AnswerMessage,
    IceCandidate,
    IceCandidateUpdateMessage,
    JoinMessage,
    Message,
    OfferMessage
};
use std::collections::HashMap;
use std::sync::{Arc, Weak};

pub enum WebRtcPeerConnectionProcess {
    Connecting(Instant),
    Connected(Arc<WebRtcPeer>)
}

impl WebRtcPeerConnectionProcess {
    pub fn connecting() -> Self {
        Self::Connecting(Instant::now())
    }

    pub fn connected(peer: Arc<WebRtcPeer>) -> Self {
        Self::Connected(peer)
    }

    pub fn get(&self) -> Option<Arc<WebRtcPeer>> {
        match self {
            Self::Connecting(_) => None,
            Self::Connected(peer) => Some(peer.clone())
        }
    }
}

#[derive(Debug, Clone)]
pub struct SharedContext {
    peers: Arc<Mutex<HashMap<PeerId, WebRtcPeerConnectionProcess>>>,
    peer_msg_channels: Arc<Mutex<HashMap<PeerId, DirectMessageChannel>>>,
    finding_scopes: Arc<Mutex<Vec<FindingScope>>>,
    current_id: OnceCell<PeerId>
}

impl Default for SharedContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedContext {
    pub fn new() -> Self {
        Self {
            current_id: Default::default(),
            finding_scopes: Default::default(),
            peers: Default::default(),
            peer_msg_channels: Default::default()
        }
    }

    pub fn set_current_id(&self, id: PeerId) {
        let _ = self.current_id.set(id);
    }

    pub fn get_current_id(&self) -> PeerId {
        *self.current_id.get().unwrap()
    }

    pub async fn update_finding_scopes(&self, scopes: Vec<FindingScope>) {
        let mut finding_scopes = self.finding_scopes.lock().await;
        if scopes.ne(&*finding_scopes) {
            log::info!("Updating finding scopes: {scopes:#?}");
        }

        finding_scopes.clear();
        finding_scopes.extend(scopes);
    }

    pub async fn get_finding_scopes(&self) -> Vec<FindingScope> {
        self.finding_scopes.lock().await.clone()
    }

    pub async fn add_peer_msg_channel(&self, peer_id: &PeerId, channel: &DirectMessageChannel) {
        let mut peer_msg_channels = self.peer_msg_channels.lock().await;
        peer_msg_channels.insert(*peer_id, channel.clone());
    }

    pub async fn notify_peer_response(&self, peer_id: &PeerId, request_id: String, response: Response) {
        let mut peer_msg_channels = self.peer_msg_channels.lock().await;
        if let Some(channel) = peer_msg_channels.get_mut(peer_id) {
            let _ = channel.notify_response(request_id, response).await;
        }
    }

    pub async fn get_peer(&self, peer_id: &PeerId) -> Option<Weak<WebRtcPeer>> {
        let peers = self.peers.lock().await;
        if let Some(peer) = peers.get(peer_id).and_then(|it| it.get()) {
            return Some(Arc::downgrade(&peer));
        }

        None
    }

    pub async fn add_peer_place_holder(&self, peer_id: PeerId) {
        let mut peers = self.peers.lock().await;
        peers.insert(peer_id, WebRtcPeerConnectionProcess::connecting());
    }

    pub async fn remove_peer(&self, peer_id: &PeerId) {
        log::info!("Removing peer: {peer_id:?}");
        let mut peers = self.peers.lock().await;
        if let Some(peer) = peers.remove(peer_id).and_then(|it| it.get()) {
            log::info!("Peer removed");
            drop(peers);
            peer.peer_disconnected().await;
        } else {
            log::info!("Peer not found {peer_id}");
        }
    }

    pub async fn add_peer(&self, peer: WebRtcPeer) {
        let peer_id = peer.peer.peer_id();
        let mut peers = self.peers.lock().await;
        peers.insert(peer_id, WebRtcPeerConnectionProcess::connected(Arc::new(peer)));
    }

    // Return true when peer is not connected
    // and not connecting
    pub async fn is_peer_connected_or_connecting(&self, peer_id: &PeerId) -> bool {
        self.peers.lock().await.get(peer_id).is_some()
    }

    pub async fn is_peer_connected(&self, peer_id: &PeerId) -> bool {
        self.peers.lock().await.get(peer_id).and_then(|it| it.get()).is_some()
    }

    pub async fn poll_timeout(&self) {
        let peers = self.peers.lock().await;
        let mut peers_to_remove = vec![];
        for (peer_id, peer) in peers.iter() {
            if let WebRtcPeerConnectionProcess::Connecting(connecting_time) = peer {
                if Instant::now().duration_since(*connecting_time).as_secs() > 10 {
                    log::info!("Peer not connected for 10 seconds: {peer_id:?}");
                    peers_to_remove.push(*peer_id);
                }
            }
        }

        for peer_id in peers_to_remove {
            self.remove_peer(&peer_id).await;
        }
    }
}

#[derive(Debug)]
struct SignallingPeerRequest(PeerId, PeerRequest);

#[derive(Debug)]
struct SignallingPeerResponse(Message);

impl TryFrom<SignallingPeerRequest> for Message {
    type Error = WebRtcErrors;

    fn try_from(value: SignallingPeerRequest) -> Result<Self, Self::Error> {
        let my_id = value.0;
        match value.1 {
            PeerRequest::Signal { receiver, data: signal } => {
                let mut msg = Message {
                    to_id: Some(receiver.to_string()),
                    from_id: my_id.to_string(),
                    ..Default::default()
                };

                match signal {
                    PeerSignal::IceCandidate(ice) => {
                        let ice_msg = IceCandidate::from(ice);
                        msg.ice_candidate_update = Some(IceCandidateUpdateMessage { ice_candidates: ice_msg });
                    }
                    PeerSignal::Offer(sdp) => {
                        msg.offer = Some(OfferMessage { sdp });
                    }
                    PeerSignal::Answer(sdp) => msg.answer = Some(AnswerMessage { sdp })
                };

                Ok(msg)
            }
            PeerRequest::KeepAlive => {
                // The keep alive message will be used to continuously to notify
                // the room about our present
                Ok(Message {
                    from_id: my_id.to_string(),
                    join: Some(JoinMessage { id: my_id.to_string() }),
                    ..Default::default()
                })
            }
        }
    }
}

impl TryFrom<SignallingPeerResponse> for PeerEvent {
    type Error = WebRtcErrors;

    fn try_from(value: SignallingPeerResponse) -> Result<Self, Self::Error> {
        let value = value.0;
        let sender_id: PeerId = PeerId(value.from_id.parse().unwrap());
        if let Some(join_msg) = value.join {
            let peer_id = PeerId(join_msg.id.parse().unwrap());
            return Ok(Self::NewPeer(peer_id))
        } else if let Some(ice_msg) = value.ice_candidate_update {
            let ice = ice_msg.ice_candidates.as_string();
            let signal = PeerSignal::IceCandidate(ice);
            return Ok(Self::Signal {
                sender: sender_id,
                data: signal
            })
        } else if let Some(offer_msg) = value.offer {
            let offer = offer_msg.sdp;
            let signal = PeerSignal::Offer(offer);
            return Ok(Self::Signal {
                sender: sender_id,
                data: signal
            })
        } else if let Some(answer_msg) = value.answer {
            let answer = answer_msg.sdp;
            let signal = PeerSignal::Answer(answer);
            return Ok(Self::Signal {
                sender: sender_id,
                data: signal
            })
        } else if let Some(left_msg) = value.left_message {
            let peer_id = PeerId(left_msg.id.parse().unwrap());
            return Ok(Self::PeerLeft(peer_id))
        }

        Err(WebRtcErrors::UnSupportedEventFromSignallingServer)
    }
}

pub struct WebSignaller {
    client: SignallingClient,
    peer_id: PeerId,
    shared_context: SharedContext
}

impl WebSignaller {
    pub fn new(client: SignallingClient, peer_id: PeerId, shared_context: SharedContext) -> Self {
        Self {
            client,
            peer_id,
            shared_context
        }
    }

    pub async fn start(&mut self) -> Result<(), WebRtcErrors> {
        let first_msg = Message {
            from_id: self.peer_id.to_string(),
            join: Some(JoinMessage {
                id: self.peer_id.to_string()
            }),
            ..Default::default()
        };

        // Send the join msg right after the socket connected
        self.client.send(first_msg).await.map_err(Into::<WebRtcErrors>::into)?;
        self.client.start().await
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl Signaller for WebSignaller {
    async fn send(&mut self, request: PeerRequest) -> Result<(), SignalingError> {
        let request = SignallingPeerRequest(self.peer_id, request);
        let mut message = match TryInto::<Message>::try_into(request) {
            Ok(msg) => msg,
            Err(err) => {
                log::error!("Signaller: Failed to convert request to message: {err:#?}");
                return Err(err.into());
            }
        };

        message.scopes = self.shared_context.get_finding_scopes().await.iter().map(|it| it.as_string()).collect::<Vec<_>>();

        self.client.send(message).await.map_err(Into::<SignalingError>::into)?;
        Ok(())
    }

    async fn next_message(&mut self) -> Result<PeerEvent, SignalingError> {
        loop {
            let message = self.client.next_message().await.map_err(Into::<SignalingError>::into)?;
            let response = SignallingPeerResponse(message);
            let peer_event = response.try_into().map_err(Into::<SignalingError>::into)?;
            if let PeerEvent::NewPeer(ref peer_id) = peer_event {
                if peer_id.0 >= self.peer_id.0 {
                    continue;
                }

                if !self.shared_context.is_peer_connected_or_connecting(peer_id).await {
                    self.shared_context.add_peer_place_holder(*peer_id).await;
                    return Ok(peer_event);
                }
            } else {
                if matches!(peer_event, PeerEvent::PeerLeft(_)) {
                    log::info!("Peer left: {peer_event:?}");
                }

                return Ok(peer_event);
            }
        }
    }
}

#[derive(Debug)]
pub struct WebSignallerBuilder {
    shared_context: SharedContext
}

impl WebSignallerBuilder {
    pub fn new(context: SharedContext) -> Self {
        Self { shared_context: context }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl SignallerBuilder for WebSignallerBuilder {
    async fn new_signaller(&self, _attempts: Option<u16>, socket_url: String) -> Result<Box<dyn Signaller>, SignalingError> {
        let client = SignallingClient::new(socket_url);
        let id = self.shared_context.get_current_id();
        let mut signaller = WebSignaller::new(client, id, self.shared_context.clone());
        signaller.start().await.map_err(Into::<SignalingError>::into)?;

        Ok(Box::new(signaller))
    }
}

unsafe impl Send for WebSignallerBuilder {}
