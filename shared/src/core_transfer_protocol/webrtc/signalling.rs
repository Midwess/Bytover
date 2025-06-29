use std::sync::Arc;
use async_trait::async_trait;
use matchbox_protocol::PeerId;
use schema::devlog::rpc_signalling::server::{Message, JoinMessage, IceCandidateUpdateMessage, IceCandidate, OfferMessage, AnswerMessage};
use matchbox_socket::{PeerEvent, PeerRequest, PeerSignal, SignalingError, Signaller, SignallerBuilder};
use ulid::Ulid;
use uuid::Uuid;
use crate::core_transfer_protocol::webrtc::signalling_client::SignallingClient;

use super::errors::WebRtcErrors;

#[derive(Debug)]
struct SignallingPeerRequest(Uuid, PeerRequest);

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
                        let ice_msg = IceCandidate::try_from(ice)?;
                        msg.ice_candidate_update = Some(IceCandidateUpdateMessage {
                            ice_candidates: ice_msg
                        });
                    }
                    PeerSignal::Offer(sdp) => {
                        msg.offer = Some(OfferMessage {
                            sdp
                        });
                    }
                    PeerSignal::Answer(sdp) => {
                        msg.answer = Some(AnswerMessage {
                            sdp
                        })
                    }
                };

                Ok(msg)
            },
            PeerRequest::KeepAlive => {
                // The keep alive message will be used to continuously to notify
                // the room about our present
                Ok(Message {
                    from_id: my_id.to_string(),
                    join: Some(JoinMessage {
                        id: my_id.to_string(),
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
        let sender_id: PeerId = PeerId(value.from_id.parse().unwrap());
        if let Some(join_msg) = value.join {
            let peer_id = PeerId(join_msg.id.parse().unwrap());
            return Ok(Self::NewPeer(peer_id))
        }
        else if let Some(ice_msg) = value.ice_candidate_update {
            let ice = ice_msg.ice_candidates.as_string();
            let signal = PeerSignal::IceCandidate(ice);
            return Ok(Self::Signal { sender: sender_id, data: signal })
        }
        else if let Some(offer_msg) = value.offer {
            let offer = offer_msg.sdp;
            let signal = PeerSignal::Offer(offer);
            return Ok(Self::Signal { sender: sender_id, data: signal })
        }
        else if let Some(answer_msg) = value.answer {
            let answer = answer_msg.sdp;
            let signal = PeerSignal::Answer(answer);
            return Ok(Self::Signal { sender: sender_id, data: signal })
        }
        else if let Some(left_msg) = value.left_message {
            let peer_id = PeerId(left_msg.id.parse().unwrap());
            return Ok(Self::PeerLeft(peer_id))
        }

        Err(WebRtcErrors::UnSupportedEventFromSignallingServer)
    }
}

pub struct WebSignaller {
    client: SignallingClient,
    peer_id: Uuid
}

impl WebSignaller {
    pub fn new(client: SignallingClient, peer_id: Uuid) -> Self {
        Self {
            client,
            peer_id
        }
    }

    pub async fn start(&mut self) -> Result<(), WebRtcErrors> {
        self.client.start().await
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl Signaller for WebSignaller {
    async fn send(&mut self, request: PeerRequest) -> Result<(), SignalingError> {
        let request = SignallingPeerRequest(self.peer_id, request);
        log::debug!("Signaller: Sending request: {request:#?}");
        let Ok(message) = TryInto::<Message>::try_into(request) else {
            return Ok(())
        };

        self.client.send(message).await.map_err(Into::<SignalingError>::into)?;
        Ok(())
    }

    async fn next_message(&mut self) -> Result<PeerEvent, SignalingError> {
        let message = self.client.next_message().await.map_err(Into::<SignalingError>::into)?;
        let response = SignallingPeerResponse(message);
        let peer_event = response.try_into().map_err(Into::<SignalingError>::into)?;

        Ok(peer_event)
    }
}

#[derive(Debug)]
pub struct WebSignallerBuilder {}

impl WebSignallerBuilder {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl SignallerBuilder for WebSignallerBuilder {
    async fn new_signaller(
        &self,
        _attempts: Option<u16>,
        socket_url: String,
    ) -> Result<Box<dyn Signaller>, SignalingError> {
        let client = SignallingClient::new(socket_url);
        let id = Ulid::new();
        let mut signaller = WebSignaller::new(client, Uuid::from_bytes(id.to_bytes().into()));
        signaller.start().await.map_err(|it| Into::<SignalingError>::into(it))?;

        Ok(Box::new(signaller))
    }
}
