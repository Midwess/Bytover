use crate::protocol::webrtc::errors::WebRtcErrors;
use async_stream::stream;
use futures::channel::mpsc;
use futures::channel::mpsc::UnboundedSender;
use futures::Stream;
use futures_util::lock::Mutex;
use futures_util::{SinkExt, StreamExt};
use matchbox_protocol::PeerId;
use matchbox_socket::Packet;
use prost::Message;
use schema::devlog::bitbridge::peer_message_body::Response;
use schema::devlog::bitbridge::{peer_message_body, PeerMessageBody};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct DirectMessageChannel {
    to_peer_id: PeerId,
    response_streams: Arc<Mutex<HashMap<String, mpsc::Sender<Response>>>>,
    outbound_sender: UnboundedSender<(PeerId, Packet)>
}

impl DirectMessageChannel {
    pub fn new(peer_id: PeerId, outbound_sender: UnboundedSender<(PeerId, Packet)>) -> Self {
        DirectMessageChannel {
            response_streams: Arc::new(Mutex::new(HashMap::new())),
            outbound_sender,
            to_peer_id: peer_id
        }
    }

    pub async fn send_response(&self, request_id: String, response: Response) -> Result<(), WebRtcErrors> {
        let mut binary = vec![];

        PeerMessageBody {
            request_id,
            response: Some(response),
            ..Default::default()
        }
        .encode(&mut binary)?;

        let packet = Packet::from(binary);
        let _ = self.outbound_sender.unbounded_send((self.to_peer_id, packet));

        Ok(())
    }

    pub async fn notify_response(&self, request_id: String, response: Response) {
        // We clone the tx to drop the lock, make sure it will not blocking
        let tx = self.response_streams.lock().await.get_mut(&request_id).cloned();
        if let Some(tx) = tx {
            let _ = tx.clone().send(response).await;
        }
    }

    pub async fn send(&self, request: peer_message_body::Request, request_id: Option<uuid::Uuid>) -> Result<Response, WebRtcErrors> {
        let request_id = request_id.unwrap_or(uuid::Uuid::now_v7()).to_string();
        let msg = PeerMessageBody {
            request: Some(request),
            request_id: request_id.clone(),
            ..Default::default()
        };

        let mut bytes = vec![];
        msg.encode(&mut bytes)?;
        let packet = Packet::from(bytes);

        self.outbound_sender
            .unbounded_send((self.to_peer_id, packet))
            .map_err(|e| WebRtcErrors::MessageChannelError(format!("{e:?}")))?;

        let (tx, mut rx) = mpsc::channel(1);
        self.response_streams.lock().await.insert(request_id.clone(), tx);
        let Some(response) = rx.next().await else {
            return Err(WebRtcErrors::MessageChannelError("No response".to_string()));
        };

        self.response_streams.lock().await.remove(&request_id);
        Ok(response)
    }

    pub async fn notify(&self, request: peer_message_body::Request) -> Result<String, WebRtcErrors> {
        let request_id = uuid::Uuid::now_v7().to_string();
        let msg = PeerMessageBody {
            request: Some(request),
            request_id: request_id.clone(),
            ..Default::default()
        };

        let mut bytes = vec![];
        msg.encode(&mut bytes)?;
        let packet = Packet::from(bytes);

        self.outbound_sender
            .unbounded_send((self.to_peer_id, packet))
            .map_err(|e| WebRtcErrors::MessageChannelError(format!("{e:?}")))?;

        Ok(request_id)
    }

    pub async fn stream(&self, request: peer_message_body::Request) -> Result<impl Stream<Item = Response> + '_, WebRtcErrors> {
        let request_id = self.notify(request).await?;

        let (tx, mut rx) = mpsc::channel(64);
        self.response_streams.lock().await.insert(request_id.clone(), tx);

        Ok(stream! {
            while let Some(response) = rx.next().await {
                yield response;
            }

            self.response_streams.lock().await.remove(&request_id);
        })
    }
}
