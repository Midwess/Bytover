use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

use core_services::retry;
use prost::Message as prost_message;
use schema::devlog::bitbridge::peer_message_body::{Request, Response};
use schema::devlog::bitbridge::{PeerErrorsMessage, PeerMessageBody};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::SendError;
use tokio::time::timeout;
use uuid::Uuid;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;

use super::connection::ConnectionWebRtcErrors;

pub struct PeerRequest {
    request: Request,
    pub id: String,
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
            request_id: self.id.clone(),
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

#[derive(Clone)]
pub struct MessageChannel {
    msg_channel: Arc<RTCDataChannel>,
    msg_response_broadcast: broadcast::Sender<Result<(String, Response), String>>,
    msg_request_broadcast: broadcast::Sender<Result<(String, Request), String>>
}

impl MessageChannel {
    pub fn new(msg_channel: Arc<RTCDataChannel>) -> Self {
        let (msg_response_broadcast, _) = broadcast::channel(100);
        let (msg_request_broadcast, _) = broadcast::channel(100);

        let msg_response_broadcast_cloned = msg_response_broadcast.clone();

        {
            let msg_request_broadcast_cloned = msg_request_broadcast.clone();
            let msg_response_broadcast_cloned = msg_response_broadcast.clone();
            msg_channel.on_close(Box::new(move || {
                let msg_request_broadcast = msg_request_broadcast_cloned.clone();
                let msg_response_broadcast_cloned = msg_response_broadcast_cloned.clone();
                Box::pin(async move {
                    log::info!(target: "broadcast", "Connection closed");
                    let _ = msg_request_broadcast.send(Err("Channel closed".to_string()));
                    let _ = msg_response_broadcast_cloned.send(Err("Channel closed".to_string()));
                })
            }));

            let msg_request_broadcast_cloned = msg_request_broadcast.clone();
            let msg_response_broadcast_cloned = msg_response_broadcast.clone();
            msg_channel.on_error(Box::new(move |e| {
                let msg_request_broadcast = msg_request_broadcast_cloned.clone();
                let msg_response_broadcast_cloned = msg_response_broadcast_cloned.clone();
                Box::pin(async move {
                    log::info!(target: "broadcast", "Connection error: {:?}", e);
                    let _ = msg_request_broadcast.send(Err(format!("Channel error: {:?}", e)));
                    let _ = msg_response_broadcast_cloned.send(Err(format!("Channel error: {:?}", e)));
                })
            }));
        }

        let msg_request_broadcast_cloned = msg_request_broadcast.clone();
        msg_channel.on_message(Box::new(move |msg: DataChannelMessage| {
            log::info!(target: "broadcast", "Received message");
            let msg = match PeerMessageBody::decode(msg.data) {
                Ok(msg) => msg,
                Err(e) => {
                    log::error!(target: "rtc", "Failed to decode message {:?}", e);
                    return Box::pin(async move {});
                }
            };

            if let Some(response) = msg.response {
                let msg_response_broadcast_cloned = msg_response_broadcast_cloned.clone();
                return Box::pin(async move {
                    let result = retry!(retries = 10, delay = Duration::from_millis(50), |_e: &SendError<_>| true, {
                        msg_response_broadcast_cloned.send(Ok((msg.request_id.clone(), response.clone())))
                    });

                    if let Err(e) = result {
                        log::error!(target: "rtc", "Failed to broadcast response message {:?}", e);
                    }
                });
            }

            if let Some(request) = msg.request {
                let msg_request_broadcast = msg_request_broadcast_cloned.clone();
                return Box::pin(async move {
                    let result = retry!(retries = 10, delay = Duration::from_millis(50), |_e: &SendError<_>| true, {
                        msg_request_broadcast.send(Ok((msg.request_id.clone(), request.clone())))
                    });

                    if let Err(e) = result {
                        log::error!(target: "rtc", "Failed to broadcast request message {:?}", e);
                    }
                });
            }

            Box::pin(async move {})
        }));

        Self {
            msg_channel,
            msg_response_broadcast,
            msg_request_broadcast
        }
    }

    pub async fn send_and_forget(&self, message: PeerMessageBody) -> Result<(), ConnectionWebRtcErrors> {
        let bytes = Self::encode_msg(&message)?;
        let _ = timeout(Duration::from_secs(5), self.msg_channel.send(&bytes.into())).await?;

        Ok(())
    }

    pub async fn close(&self) {
        let _ = self.msg_channel.close().await;
        let _ = self.msg_request_broadcast.send(Err("Connection closed".to_string()));
        let _ = self.msg_response_broadcast.send(Err("Connection closed".to_string()));
    }

    pub async fn next_request(&self) -> Result<PeerRequest, ConnectionWebRtcErrors> {
        let mut subscription = self.msg_request_broadcast.subscribe();
        let result = subscription.recv().await;
        match result {
            Ok(Ok((request_id, request))) => Ok(PeerRequest::new(request, request_id, self.msg_channel.clone())),
            Ok(Err(e)) => Err(ConnectionWebRtcErrors::ConnectionCorrupted),
            Err(e) => Err(ConnectionWebRtcErrors::ConnectionCorrupted)
        }
    }

    pub async fn send<T: TryFrom<Response, Error = String>>(
        &self,
        msg: Request
    ) -> Result<Result<T, PeerErrorsMessage>, ConnectionWebRtcErrors> {
        let request_id = Self::random_id();
        let message = PeerMessageBody {
            request_id: request_id.clone(),
            request: Some(msg),
            response: None,
            ..Default::default()
        };

        let bytes = Self::encode_msg(&message)?;
        let mut subscription = self.msg_response_broadcast.subscribe();

        let _ = timeout(Duration::from_secs(90), self.msg_channel.send(&bytes.into())).await?;

        while let Ok(Ok((response_id, response))) = timeout(Duration::from_secs(300), subscription.recv()).await? {
            if response_id != request_id {
                continue;
            }

            if let Response::Errors(e) = response {
                if let Ok(error) = PeerErrorsMessage::try_from(e) {
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
