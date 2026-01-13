use super::errors::WebRtcErrors;
use crate::app::operations::p2p::P2POperationOutput;
use crate::entities::finding_scope::FindingScope;
use crate::protocol::webrtc::message_channel::DirectMessageChannel;
use crate::protocol::webrtc::peer::WebRtcPeer;
use crate::protocol::webrtc::signalling_client::SignallingClient;
use crate::shell::api::CoreRequest;
use futures_util::lock::Mutex;
use matchbox_protocol::{PeerId, RtcIceServerConfig};
use matchbox_socket::{PeerEvent, PeerRequest, PeerSignal, SignalingError, Signaller, SignallerBuilder};
use n0_future::time::Instant;
use schema::devlog::bitbridge::peer_message_body::Response;
use schema::devlog::rpc_signalling::server::{
    AnswerMessage,
    IceCandidate,
    IceCandidateUpdateMessage,
    IceConfig,
    JoinMessage,
    LeftMessage,
    Message,
    OfferMessage,
    ScopeState
};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Weak};
use std::time::Duration;

pub enum WebRtcPeerConnectionProcess {
    Connecting(Instant),
    Connected(Weak<WebRtcPeer>)
}

impl WebRtcPeerConnectionProcess {
    pub fn connecting() -> Self {
        Self::Connecting(Instant::now())
    }

    pub fn connected(peer: Weak<WebRtcPeer>) -> Self {
        Self::Connected(peer)
    }

    pub fn get(&self) -> Option<&Weak<WebRtcPeer>> {
        match self {
            Self::Connecting(_) => None,
            Self::Connected(peer) => Some(peer)
        }
    }
}

#[derive(Clone)]
pub struct SharedContext {
    peers: Arc<Mutex<HashMap<PeerId, WebRtcPeerConnectionProcess>>>,
    peer_msg_channels: Arc<Mutex<HashMap<PeerId, DirectMessageChannel>>>,
    finding_scopes: Arc<Mutex<Vec<FindingScope>>>,
    current_id: Arc<Mutex<PeerId>>,
    signaller: Arc<Mutex<Weak<SignallingClient>>>,
    core_request: Arc<Mutex<Option<CoreRequest>>>
}

impl Default for SharedContext {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedContext {
    pub fn new() -> Self {
        Self {
            current_id: Arc::new(Mutex::new(PeerId(Default::default()))),
            finding_scopes: Default::default(),
            peers: Default::default(),
            peer_msg_channels: Default::default(),
            signaller: Default::default(),
            core_request: Arc::new(Mutex::new(None))
        }
    }

    pub async fn signaller(&self) -> Option<Arc<SignallingClient>> {
        self.signaller.lock().await.upgrade()
    }

    pub async fn set_current_id(&self, id: PeerId) {
        *self.current_id.lock().await = id;
    }

    pub async fn get_current_id(&self) -> PeerId {
        *self.current_id.lock().await
    }

    pub async fn set_core_request(&self, core_request: CoreRequest) {
        *self.core_request.lock().await = Some(core_request);
    }

    pub async fn update_finding_scopes(&self, scopes: Vec<FindingScope>) {
        let id = self.get_current_id().await;
        let mut finding_scopes = self.finding_scopes.lock().await;
        log::info!("Updating finding scopes: {scopes:?}");

        finding_scopes.clear();
        finding_scopes.extend(scopes);
        let scopes = finding_scopes.iter().map(|it| it.as_string()).collect::<Vec<_>>();
        drop(finding_scopes);
        if let Some(signaller) = self.signaller().await {
            let _ = signaller
                .send(Message {
                    from_id: id.to_string(),
                    join: Some(JoinMessage {
                        id: id.to_string(),
                        ..Default::default()
                    }),
                    scopes,
                    ..Default::default()
                })
                .await;
        }
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
            return Some(peer.clone());
        }

        None
    }

    pub async fn add_peer_place_holder(&self, peer_id: PeerId) {
        let mut peers = self.peers.lock().await;
        peers.insert(peer_id, WebRtcPeerConnectionProcess::connecting());
    }

    async fn disconnect_peer(&self, peer_id: &PeerId) {
        if let Some(signaller) = self.signaller().await {
            let _ = signaller.append_msg(Message {
                left_message: Some(LeftMessage { id: peer_id.0.to_string() }),
                from_id: peer_id.0.to_string(),
                ..Default::default()
            });

            let current_id = self.get_current_id().await;
            let _ = signaller
                .send(Message {
                    left_message: Some(LeftMessage {
                        id: current_id.to_string()
                    }),
                    from_id: current_id.to_string(),
                    to_id: Some(peer_id.to_string()),
                    ..Default::default()
                })
                .await;

            log::info!("Disconnect signal has been emitted: {peer_id:?}");
        }
    }

    pub async fn remove_all(&self) -> Vec<PeerId> {
        let peers = self.peers.lock().await.drain().collect::<Vec<_>>();
        let mut removed_peers = Vec::new();
        for (id, peer) in peers {
            if let Some(peer_weak) = peer.get() {
                self.disconnect_peer(&id).await;
                if let Some(peer) = peer_weak.upgrade() {
                    removed_peers.push(id);
                    peer.peer_disconnected().await;
                }
            }
        }

        removed_peers
    }

    pub async fn remove_peer(&self, peer_id: &PeerId) {
        let peer_weak = self.peers.lock().await.remove(peer_id).and_then(|it| it.get().cloned());
        if let Some(peer_weak) = peer_weak {
            if let Some(peer) = peer_weak.upgrade() {
                self.disconnect_peer(peer_id).await;
                peer.peer_disconnected().await;
            }
        } else {
            log::warn!("Peer not found: {peer_id:?}");
        }
    }

    pub async fn add_peer(&self, peer: Weak<WebRtcPeer>) {
        if let Some(peer) = peer.upgrade() {
            let peer_id = peer.peer.peer_id();
            let mut peers = self.peers.lock().await;
            peers.insert(peer_id, WebRtcPeerConnectionProcess::connected(Arc::downgrade(&peer)));
        }
    }

    // Return true when peer is not connected
    // and not connecting
    pub async fn is_peer_connected_or_connecting(&self, peer_id: &PeerId) -> bool {
        self.peers.lock().await.get(peer_id).is_some()
    }

    pub async fn get_all_connected_peers(&self) -> Vec<Arc<WebRtcPeer>> {
        let peers = self.peers.lock().await;
        peers.values().filter_map(|process| process.get().and_then(|weak| weak.upgrade())).collect()
    }

    pub async fn is_peer_connected(&self, peer_id: &PeerId) -> bool {
        self.peers.lock().await.get(peer_id).and_then(|it| it.get()).is_some()
    }

    pub async fn send_scope_state(&self, scope_id: String, state: ScopeState, owner_id: Option<String>) {
        if let Some(core_request) = self.core_request.lock().await.as_ref() {
            let _ = core_request.response(P2POperationOutput::ScopeStateChanged { scope_id, state, owner_id }).await;
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
                    PeerSignal::Offer { offer, .. } => {
                        msg.offer = Some(OfferMessage {
                            sdp: offer,
                            ..Default::default()
                        });
                    }
                    PeerSignal::Answer(sdp) => msg.answer = Some(AnswerMessage { sdp }),
                    PeerSignal::RetryWithRelay => {
                        msg.peer_request_relay_only = Some(Default::default());
                    }
                };

                Ok(msg)
            }
            PeerRequest::KeepAlive => {
                // The keep alive message will be used to continuously to notify
                // the room about our present
                Ok(Message {
                    from_id: my_id.to_string(),
                    join: Some(JoinMessage {
                        id: my_id.to_string(),
                        ..Default::default()
                    }),
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
        let sender_id: PeerId = PeerId(value.from_id.parse()?);
        if let Some(join_msg) = value.join {
            let peer_id = PeerId(join_msg.id.parse()?);
            return Ok(Self::NewPeer {
                id: peer_id,
                ice_config: join_msg.ice_config.map(create_matchbox_ice_config)
            })
        } else if let Some(ice_msg) = value.ice_candidate_update {
            let ice = ice_msg.ice_candidates.as_string();
            let signal = PeerSignal::IceCandidate(ice);
            return Ok(Self::Signal {
                sender: sender_id,
                data: signal
            })
        } else if let Some(offer_msg) = value.offer {
            let offer = offer_msg.sdp;
            let signal = PeerSignal::Offer {
                offer,
                config: offer_msg.ice_config.map(create_matchbox_ice_config)
            };
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
            let peer_id = PeerId(left_msg.id.parse()?);
            return Ok(Self::PeerLeft(peer_id))
        }
        else if let Some(_) = value.peer_request_relay_only {
            let signal = PeerSignal::RetryWithRelay;
            return Ok(Self::Signal {
                sender: sender_id,
                data: signal
            })
        }

        Err(WebRtcErrors::UnSupportedEventFromSignallingServer)
    }
}

pub struct WebSignaller {
    client: Arc<SignallingClient>,
    peer_id: PeerId,
    shared_context: SharedContext
}

impl WebSignaller {
    pub fn new(client: Arc<SignallingClient>, peer_id: PeerId, shared_context: SharedContext) -> Self {
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
                id: self.peer_id.to_string(),
                ..Default::default()
            }),
            scopes: self.shared_context.get_finding_scopes().await.iter().map(|it| it.as_string()).collect::<Vec<_>>(),
            ..Default::default()
        };

        // Send the join msg right after the socket connected
        let result = self.client.start(self.shared_context.clone()).await;
        self.client.send(first_msg).await?;
        result
    }
}

impl Drop for WebSignaller {
    fn drop(&mut self) {
        log::info!("WebSignaller dropped");
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
            let Some(message) = self.client.try_next_message().await.map_err(Into::<SignalingError>::into)? else {
                n0_future::time::sleep(Duration::from_millis(5)).await;
                continue;
            };

            if let Some(scope_state_msg) = message.scope_state_changed {
                let scope_id = scope_state_msg.scope_id.clone();
                let state = scope_state_msg.state();
                let owner_id = scope_state_msg.owner_id.clone();
                log::info!("Received scope state changed: {:?} -> {:?}, owner: {:?}", scope_id, state, owner_id);
                self.shared_context.send_scope_state(scope_id, state, owner_id).await;
                continue;
            }

            let response = SignallingPeerResponse(message);
            let peer_event = response.try_into().map_err(Into::<SignalingError>::into)?;
            if let PeerEvent::NewPeer { ref id, .. } = peer_event {
                if id.0 <= self.peer_id.0 {
                    continue;
                }

                if !self.shared_context.is_peer_connected_or_connecting(id).await {
                    self.shared_context.add_peer_place_holder(*id).await;
                    log::info!("New peer found: {id:?}, connecting...");
                    return Ok(peer_event);
                }
            } else {
                return Ok(peer_event);
            }
        }
    }
}

pub struct WebSignallerBuilder {
    shared_context: SharedContext
}

impl Debug for WebSignallerBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSignallerBuilder").finish()
    }
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
        let client = Arc::new(SignallingClient::new(socket_url));
        *self.shared_context.signaller.lock().await = Arc::downgrade(&client);
        let id = self.shared_context.get_current_id().await;
        let mut signaller = WebSignaller::new(client, id, self.shared_context.clone());
        signaller.start().await.map_err(Into::<SignalingError>::into)?;

        Ok(Box::new(signaller))
    }
}

unsafe impl Send for WebSignallerBuilder {}

fn create_matchbox_ice_config(config: IceConfig) -> RtcIceServerConfig {
    RtcIceServerConfig {
        urls: config.urls,
        username: config.username,
        credential: config.credential
    }
}
