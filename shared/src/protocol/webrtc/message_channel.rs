use crate::protocol::webrtc::errors::WebRtcErrors;
use async_stream::stream;
use futures::channel::mpsc;
use futures::Stream;
use futures_util::lock::Mutex;
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use schema::devlog::bitbridge::peer_message_body::Response;
use schema::devlog::bitbridge::{peer_message_body, PeerMessageBody};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct DirectMessageChannel {
    response_streams: Arc<Mutex<HashMap<String, mpsc::Sender<Response>>>>,
    outbound_sender: mpsc::Sender<Vec<u8>>
}

impl DirectMessageChannel {
    pub fn new(outbound_sender: mpsc::Sender<Vec<u8>>) -> Self {
        DirectMessageChannel {
            response_streams: Arc::new(Mutex::new(HashMap::new())),
            outbound_sender
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

        let packet = binary;
        let _ = self.outbound_sender.clone().send(packet).await;

        Ok(())
    }

    pub async fn notify_response(&self, request_id: String, response: Response) {
        let tx = self.response_streams.lock().await.get_mut(&request_id).cloned();
        if let Some(tx) = tx {
            let _ = tx.clone().send(response).await;
        }
    }

    pub async fn send(&self, request: peer_message_body::Request, request_id: Option<uuid::Uuid>) -> Result<Response, WebRtcErrors> {
        let request_id = request_id.unwrap_or(uuid::Uuid::new_v4()).to_string();
        let msg = PeerMessageBody {
            request: Some(request),
            request_id: request_id.clone(),
            ..Default::default()
        };

        let mut bytes = vec![];
        msg.encode(&mut bytes)?;
        let packet = bytes;

        let (tx, mut rx) = mpsc::channel(1);
        self.response_streams.lock().await.insert(request_id.clone(), tx);

        if let Err(e) = self.outbound_sender.clone().send(packet).await {
            self.response_streams.lock().await.remove(&request_id);
            return Err(WebRtcErrors::MessageChannelError(format!("{e:?}")));
        }
        let Some(response) = rx.next().await else {
            return Err(WebRtcErrors::MessageChannelError("No response".to_string()));
        };

        self.response_streams.lock().await.remove(&request_id);
        Ok(response)
    }

    pub async fn notify(&self, request: peer_message_body::Request) -> Result<String, WebRtcErrors> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let msg = PeerMessageBody {
            request: Some(request),
            request_id: request_id.clone(),
            ..Default::default()
        };

        let mut bytes = vec![];
        msg.encode(&mut bytes)?;
        let packet = bytes;

        self.outbound_sender
            .clone()
            .send(packet)
            .await
            .map_err(|e| WebRtcErrors::MessageChannelError(format!("{e:?}")))?;

        Ok(request_id)
    }

    pub async fn stream(&self, request: peer_message_body::Request) -> Result<impl Stream<Item = Response> + '_, WebRtcErrors> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, mut rx) = mpsc::channel(64);
        self.response_streams.lock().await.insert(request_id.clone(), tx);

        let msg = PeerMessageBody {
            request: Some(request),
            request_id: request_id.clone(),
            ..Default::default()
        };

        let mut bytes = vec![];
        msg.encode(&mut bytes)?;
        let packet = bytes;

        if let Err(e) = self.outbound_sender.clone().send(packet).await {
            self.response_streams.lock().await.remove(&request_id);
            return Err(WebRtcErrors::MessageChannelError(format!("{e:?}")));
        }

        Ok(stream! {
            while let Some(response) = rx.next().await {
                yield response;
            }

            self.response_streams.lock().await.remove(&request_id);
        })
    }

    /// Receive an incoming packet and process it.
    /// If it's a response, delivers it to the appropriate request's response channel.
    /// Returns the decoded message body if it needs further processing (e.g., requests).
    pub async fn receive_packet(&self, packet: Vec<u8>) -> Result<Option<PeerMessageBody>, WebRtcErrors> {
        let msg =
            PeerMessageBody::decode(&*packet).map_err(|e| WebRtcErrors::MessageChannelError(format!("decode error: {:?}", e)))?;

        if msg.response.is_some() {
            log::info!("Received a response {msg:?}");
            self.notify_response(msg.request_id.clone(), msg.response.unwrap()).await;
            return Ok(None);
        }

        Ok(Some(msg))
    }
}
