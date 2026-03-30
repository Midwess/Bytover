use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use actix_web::web::Bytes;
use actix_ws::Session;
use futures_util::StreamExt;
use prost::Message as ProstMessage;
use thiserror::Error;
use tokio::sync::{Mutex, oneshot};
use tokio::time::timeout;
use uuid::Uuid;

use crate::turn_manager::TurnManager;

const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("client disconnected")]
    Disconnected,

    #[error("request timed out after {0}s")]
    Timeout(u64),

    #[error("internal error: {0}")]
    Internal(String),
}

pub struct Client {
    pub key: String,
    ws_session: Mutex<Session>,
    pending_requests:
        Mutex<HashMap<String, oneshot::Sender<schema::devlog::rpc_signalling::server::Message>>>,
    turn_manager: Arc<TurnManager>,
}

impl Client {
    pub fn new(key: String, session: Session, turn_manager: Arc<TurnManager>) -> Arc<Self> {
        Arc::new(Self {
            key,
            ws_session: Mutex::new(session),
            pending_requests: Mutex::new(HashMap::new()),
            turn_manager,
        })
    }

    pub async fn request(
        self: &Arc<Self>,
        mut message: schema::devlog::rpc_signalling::server::Message,
    ) -> Result<schema::devlog::rpc_signalling::server::Message, ClientError> {
        let request_id = Uuid::new_v4().to_string();
        message.request_id = Some(request_id.clone());

        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        let mut buf = Vec::new();
        message
            .encode(&mut buf)
            .map_err(|e| ClientError::Internal(e.to_string()))?;

        let mut session = self.ws_session.lock().await;
        session
            .binary(Bytes::from(buf))
            .await
            .map_err(|_| ClientError::Disconnected)?;

        timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS), rx)
            .await
            .map_err(|_| ClientError::Timeout(REQUEST_TIMEOUT_SECS))?
            .map_err(|_| ClientError::Disconnected)
    }

    pub async fn resolve_response(
        self: &Arc<Self>,
        message: schema::devlog::rpc_signalling::server::Message,
    ) {
        if let Some(request_id) = &message.request_id {
            let mut pending = self.pending_requests.lock().await;
            if let Some(tx) = pending.remove(request_id) {
                let _ = tx.send(message);
            }
        }
    }

    pub async fn run(self: Arc<Self>, mut msg_stream: actix_ws::MessageStream) {
        let mut ping_interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                biased;

                msg = msg_stream.next() => {
                    match msg {
                        Some(Ok(actix_ws::Message::Binary(data))) => {
                            if data.len() > 1024 * 32 {
                                continue;
                            }
                            if let Ok(message) =
                                schema::devlog::rpc_signalling::server::Message::decode(&data[..])
                            {
                                self.clone().resolve_response(message).await;
                            }
                        }
                        Some(Ok(actix_ws::Message::Ping(data))) => {
                            let mut session = self.ws_session.lock().await;
                            let _ = session.pong(&data).await;
                        }
                        Some(Ok(actix_ws::Message::Pong(_))) => {}
                        Some(Ok(actix_ws::Message::Close(reason))) => {
                            log::info!(
                                "Client {} WebSocket closed: {:?}",
                                self.key,
                                reason
                            );
                            break;
                        }
                        Some(Err(e)) => {
                            log::error!("Client {} WebSocket error: {}", self.key, e);
                            break;
                        }
                        None => break,
                        _ => {}
                    }
                }
                _ = ping_interval.tick() => {
                    let mut session = self.ws_session.lock().await;
                    if session.ping(b"").await.is_err() {
                        log::warn!("Client {} ping failed, disconnecting", self.key);
                        break;
                    }
                }
            }
        }

        let mut pending = self.pending_requests.lock().await;
        pending.clear();
    }
}
