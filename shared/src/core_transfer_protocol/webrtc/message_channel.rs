use std::sync::Arc;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::SinkExt;
use matchbox_protocol::PeerId;
use matchbox_socket::{Packet, WebRtcChannel};
use n0_future::StreamExt;
use prost::Message;
use tokio::sync::Mutex;
use schema::devlog::bitbridge::{peer_message_body, PeerMessageBody};
use schema::devlog::bitbridge::peer_message_body::Response;
use crate::core_transfer_protocol::webrtc::errors::WebRtcErrors;

#[derive(Clone)]
pub struct DirectMessageChannel {
    inbound_channel: (Arc<Mutex<UnboundedSender<(PeerId, Response)>>>, Arc<Mutex<UnboundedReceiver<(PeerId, Response)>>>),
    outbound_sender: Arc<Mutex<UnboundedSender<(PeerId, Packet)>>>,
}

impl DirectMessageChannel {
    pub fn new(outbound_sender: Arc<Mutex<UnboundedSender<(PeerId, Packet)>>>) -> Self {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        DirectMessageChannel {
            inbound_channel: (Arc::new(Mutex::new(sender)), Arc::new(Mutex::new(receiver))),
            outbound_sender
        }
    }

    pub async fn send_response(&self, peer_id: PeerId, response: Response) -> Result<(), WebRtcErrors> {
        let mut binary = vec![];

        PeerMessageBody {
            request_id: peer_id.to_string(),
            response: Some(response),
            ..Default::default()
        }.encode(&mut binary)?;

        let packet = Packet::from(binary);
        let _ = self.outbound_sender.lock().await.send((peer_id, packet)).await;

        Ok(())
    }

    pub async fn notify_response(&self, peer_id: PeerId, response: Response) {
        let mut sender = self.inbound_channel.0.lock().await;
        sender.send((peer_id, response)).await.unwrap();
    }

    pub async fn send(&self, to_peer_id: PeerId, request: peer_message_body::Request) -> Result<Response, WebRtcErrors> {
        let msg = PeerMessageBody {
            request: Some(request),
            request_id: to_peer_id.to_string(),
            ..Default::default()
        };

        let mut bytes = vec![];
        msg.encode(&mut bytes)?;
        let packet = Packet::from(bytes);

        let mut sender = self.outbound_sender.lock().await;
        sender.send((to_peer_id, packet)).await.map_err(|e| WebRtcErrors::MessageChannelError(format!("{e:?}")))?;

        let response = loop {
            let mut inbound_receiver = self.inbound_channel.1.lock().await;
            let Some((from_peer_id, response)) = inbound_receiver.next().await else {
                continue;
            };

            if to_peer_id != from_peer_id {
                continue;
            }

            break response;
        };

        Ok(response)
    }
}
