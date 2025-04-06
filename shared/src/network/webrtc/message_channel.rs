use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

use prost::Message as prost_message;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{PeerErrors, PeerMessageBody};
use tokio::spawn;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::timeout;
use uuid::Uuid;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;

use super::connection::ConnectionWebRtcErrors;

pub struct PeerRequest {
    request: Request,
    id: String,
    is_resolved: bool,
    msg_channel: Arc<RTCDataChannel>
}

impl PeerRequest {
    pub fn new(request: Request, id: String, msg_channel: Arc<RTCDataChannel>) -> Self {
        Self {
            request,
            id,
            is_resolved: false,
            msg_channel
        }
    }

    pub fn message(&self) -> &Request {
        &self.request
    }

    pub async fn resolve(mut self, reponse: Response) -> Result<(), ConnectionWebRtcErrors> {
        let message = PeerMessageBody {
            id: self.id.clone(),
            response: Some(reponse),
            ..Default::default()
        };

        let bytes = MessageChannel::encode_msg(&message)?;
        self.msg_channel.send(&bytes.into()).await?;
        self.is_resolved = true;

        Ok(())
    }
}

impl Deref for PeerRequest {
    type Target = Request;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl DerefMut for PeerRequest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.request
    }
}

impl Drop for PeerRequest {
    fn drop(&mut self) {
        if !self.is_resolved {
            log::info!(target: "rtc", "Auto resolving request");
            let message = PeerMessageBody {
                id: self.id.clone(),
                response: Some(Response::Errors(PeerErrors::NoResponse.into())),
                ..Default::default()
            };

            if let Ok(bytes) = MessageChannel::encode_msg(&message) {
                let msg_channel = self.msg_channel.clone();
                spawn(async move {
                    let _ = msg_channel.send(&bytes.into()).await;
                });
            }
        }
    }
}

#[derive(Clone)]
pub struct MessageChannel {
    msg_channel: Arc<RTCDataChannel>,
    msg_response_broadcast: broadcast::Sender<(String, Response)>,
    msg_request_receiver: Arc<Mutex<mpsc::Receiver<(String, Request)>>>
}

impl MessageChannel {
    pub fn new(msg_channel: Arc<RTCDataChannel>) -> Self {
        let (msg_response_broadcast, _) = broadcast::channel(100);
        let (msg_request_sender, msg_request_receiver) = mpsc::channel(100);

        let msg_response_broadcast_cloned = msg_response_broadcast.clone();
        msg_channel.on_message(Box::new(move |msg: DataChannelMessage| {
            let msg = match PeerMessageBody::decode(msg.data) {
                Ok(msg) => msg,
                Err(e) => {
                    log::error!(target: "rtc", "Failed to decode message {:?}", e);
                    return Box::pin(async move {});
                }
            };

            if let Some(response) = msg.response {
                let result = msg_response_broadcast_cloned.send((msg.id.clone(), response));
                if let Err(e) = result {
                    log::error!(target: "rtc", "Failed to send message {:?}", e);
                }
            }

            if let Some(request) = msg.request {
                let msg_request_sender = msg_request_sender.clone();
                return Box::pin(async move {
                    let result = msg_request_sender.send((msg.id, request)).await;
                    if let Err(e) = result {
                        log::error!(target: "rtc", "Failed to send message {:?}", e);
                    }
                });
            }

            Box::pin(async move {})
        }));

        Self {
            msg_channel,
            msg_response_broadcast,
            msg_request_receiver: Arc::new(Mutex::new(msg_request_receiver))
        }
    }

    pub async fn next_request(&self) -> Result<PeerRequest, ConnectionWebRtcErrors> {
        let mut receiver = self.msg_request_receiver.lock().await;
        let Some((request_id, request)) = receiver.recv().await else {
            return Err(ConnectionWebRtcErrors::ConnectionCorrupted);
        };

        Ok(PeerRequest::new(request, request_id, self.msg_channel.clone()))
    }

    pub async fn send<T: TryFrom<Response, Error = String>>(
        &self,
        msg: Request
    ) -> Result<Result<T, PeerErrors>, ConnectionWebRtcErrors> {
        let request_id = Self::random_id();
        let message = PeerMessageBody {
            id: request_id.clone(),
            request: Some(msg),
            response: None,
            ..Default::default()
        };

        let bytes = Self::encode_msg(&message)?;
        let _ = timeout(Duration::from_secs(5), self.msg_channel.send(&bytes.into())).await?;

        let mut subscription = self.msg_response_broadcast.subscribe();
        while let Ok((response_id, response)) = timeout(Duration::from_secs(15), subscription.recv()).await? {
            if response_id != request_id {
                continue;
            }

            if let Response::Errors(e) = response {
                if let Ok(error) = PeerErrors::try_from(e) {
                    return Ok(Err(error));
                }

                return Err(ConnectionWebRtcErrors::ParseError(format!("Not a valid error code {:?}", e)));
            }

            return Ok(Ok(response.try_into().map_err(ConnectionWebRtcErrors::ParseError)?));
        }

        Err(ConnectionWebRtcErrors::ConnectionCorrupted)
    }

    pub fn encode_msg(msg: &PeerMessageBody) -> Result<Vec<u8>, ConnectionWebRtcErrors> {
        let mut buf = Vec::new();
        msg.encode(&mut buf)?;
        Ok(buf)
    }

    pub fn random_id() -> String {
        let uuid = Uuid::new_v4();
        uuid.to_string()
    }
}
