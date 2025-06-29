use std::sync::Arc;
use matchbox_protocol::PeerId;
use crate::entities::peer::Peer as PeerEntity;
use schema::devlog::bitbridge::{IntroduceRequestMessage, IntroduceResponseMessage, PeerMessage};
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::peer_message_body::Response::IntroduceResponse;
use crate::core_api::CoreBridge;
use crate::core_transfer_protocol::webrtc::errors::WebRtcErrors;
use crate::core_transfer_protocol::webrtc::message_channel::DirectMessageChannel;

pub struct WebRtcPeer {
    pub msg_channel: DirectMessageChannel,
    pub peer: PeerEntity,
    pub core_bridge: Arc<dyn CoreBridge>
}

impl WebRtcPeer {
    pub async fn new(
        user: PeerEntity,
        peer_id: PeerId,
        msg_channel: DirectMessageChannel,
        core_bridge: Arc<dyn CoreBridge>
    ) -> Result<Self, WebRtcErrors> {
        let introduce_request = IntroduceRequestMessage {
            mine: PeerMessage {
                peer_id: user.id().to_string(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
                device: user.device.clone().into(),
                email: user.email.clone(),
            }
        };

        let IntroduceResponse(response) = msg_channel
            .send(peer_id, Request::IntroduceRequest(introduce_request))
            .await? else {
            return Err(WebRtcErrors::FailedToIntroducePeer)
        };

        let peer: PeerEntity = response.peer.into();

        Ok(Self {
            msg_channel,
            peer,
            core_bridge
        })
    }

    pub async fn from_introduce_request(
        user: PeerEntity,
        peer_id: PeerId,
        msg: IntroduceRequestMessage,
        msg_channel: DirectMessageChannel,
        core_bridge: Arc<dyn CoreBridge>
    ) -> Result<Self, WebRtcErrors> {
        let introduce_response = IntroduceResponse(IntroduceResponseMessage {
            peer: PeerMessage {
                peer_id: user.id().to_string(),
                name: user.name.clone(),
                avatar_url: user.avatar_url.clone(),
                device: user.device.clone().into(),
                email: user.email.clone(),
            }
        });

        msg_channel.send_response(peer_id.clone(), introduce_response).await?;

        Ok(Self {
            msg_channel,
            peer: msg.mine.into(),
            core_bridge
        })
    }

    pub fn process_request(&self, msg: Request) {
       match msg {
           _ => {}
       }
    }
}
