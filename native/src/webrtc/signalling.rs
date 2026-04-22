use core_services::utils::yield_container::YieldError;
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use schema::devlog::rpc_signalling::server::{AnswerMessage, IceConfig, IceConfigList, Message};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;

#[derive(Debug, Error)]
pub enum SignallingError {
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Signaling protocol error: {0}")]
    ProtocolError(String),

    #[error("Protobuf decode error: {0}")]
    DecodeFailed(#[from] prost::DecodeError),

    #[error("Not connected")]
    NotConnected,

    #[error("Signalling task stopped")]
    Stopped,

    #[error("HTTP request failed: {0}")]
    HttpFailed(String),

    #[error("Yield error: {0}")]
    YieldError(#[from] YieldError),
}

pub struct SignalingClient {
    ws_url: String,
    http_url: String,
    msg_rx: Option<mpsc::Receiver<Result<Message, SignallingError>>>,
    msg_transfer_tx: Option<mpsc::Sender<Vec<u8>>>,
    run_handle: Option<JoinHandle<()>>,
}

#[derive(Clone)]
pub struct SignallingSender {
    http_url: String,
    msg_transfer_tx: mpsc::Sender<Vec<u8>>,
}

impl SignallingSender {
    pub async fn fetch_relay_config(&self, key: &str) -> Result<IceConfig, SignallingError> {
        if let Some(server) = crate::config::get_relay_server_override() {
            log::info!("[signalling] Using relay server override: {}", server);
            return Ok(IceConfig {
                urls: vec![
                    format!("stun:{}", server),
                    format!("turn:{}?transport=udp", server),
                    format!("turn:{}?transport=tcp", server),
                ],
                username: Some(crate::config::get_relay_turn_username()),
                credential: Some(crate::config::get_relay_turn_password()),
            });
        }

        let url = format!("{}/relay/{}", self.http_url, key);
        let response = reqwest::get(&url).await.map_err(|e| SignallingError::HttpFailed(format!("{e:?}")))?;

        if !response.status().is_success() {
            return Err(SignallingError::HttpFailed(format!(
                "relay endpoint returned {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await.map_err(|e| SignallingError::HttpFailed(format!("{e}")))?;

        let list = IceConfigList::decode(&bytes[..]).map_err(SignallingError::from)?;
        list.configs
            .into_iter()
            .next()
            .ok_or_else(|| SignallingError::HttpFailed("relay endpoint returned empty config list".to_string()))
    }

    pub async fn send_answer(
        &self,
        sdp: String,
        peer: schema::devlog::bitbridge::PeerMessage,
        request_id: &str,
    ) -> Result<(), SignallingError> {
        let msg = Message {
            request_id: Some(request_id.to_string()),
            answer: Some(AnswerMessage { sdp, peer: Some(peer) }),
            ..Default::default()
        };
        self.send_message(&msg).await
    }

    async fn send_message(&self, msg: &Message) -> Result<(), SignallingError> {
        let mut buf = Vec::new();
        msg.encode(&mut buf)
            .map_err(|e| SignallingError::ProtocolError(format!("Failed to encode message: {e}")))?;
        self.msg_transfer_tx.send(buf).await.map_err(|_| SignallingError::NotConnected)?;
        Ok(())
    }
}

impl SignalingClient {
    pub fn new(ws_url: String, http_url: String) -> Self {
        Self {
            ws_url,
            http_url,
            msg_rx: None,
            msg_transfer_tx: None,
            run_handle: None,
        }
    }

    pub fn get_sender(&self) -> Option<SignallingSender> {
        self.msg_transfer_tx.clone().map(|tx| SignallingSender {
            http_url: self.http_url.clone(),
            msg_transfer_tx: tx,
        })
    }

    pub async fn start(&mut self, key: String) {
        if self.run_handle.is_some() {
            log::warn!("[signalling] Already running");
            return;
        }

        let url = format!("{}/server/{}", self.ws_url, key);
        let (tx, new_rx) = mpsc::channel(128);
        let (transfer_tx, transfer_rx) = mpsc::channel(128);
        self.msg_rx = Some(new_rx);
        self.msg_transfer_tx = Some(transfer_tx);

        self.run_handle = Some(tokio::spawn(async move {
            Self::run_loop(url, transfer_rx, tx).await;
        }));
    }

    async fn run_loop(url: String, mut transfer_rx: mpsc::Receiver<Vec<u8>>, msg_tx: mpsc::Sender<Result<Message, SignallingError>>) {
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(30);

        loop {
            log::info!("[signalling] Connecting to {}", url);

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    log::info!("[signalling] Connected");
                    backoff = Duration::from_secs(1);
                    let (sink, mut stream) = ws_stream.split();
                    let mut sink = sink;
                    let msg_tx = msg_tx.clone();

                    let mut ping_interval = tokio::time::interval(Duration::from_secs(30));

                    loop {
                        tokio::select! {
                            msg = stream.next() => {
                                match msg {
                                    Some(Ok(tokio_tungstenite::tungstenite::Message::Binary(data))) => {
                                        match Message::decode(data) {
                                            Ok(m) => {
                                                if msg_tx.send(Ok(m)).await.is_err() {
                                                    break;
                                                }
                                            }
                                            Err(e) => {
                                                let _ = msg_tx.send(Err(SignallingError::DecodeFailed(e))).await;
                                            }
                                        }
                                    }
                                    Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(data))) => {
                                        if sink.send(tokio_tungstenite::tungstenite::Message::Pong(data)).await.is_err() {
                                            break;
                                        }
                                    }
                                    Some(Ok(tokio_tungstenite::tungstenite::Message::Pong(_))) => {}
                                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => {
                                        break;
                                    }
                                    Some(Ok(_)) => {}
                                    Some(Err(e)) => {
                                        log::warn!("[signalling] Stream error: {e}");
                                        break;
                                    }
                                }
                            }
                            data = transfer_rx.recv() => {
                                match data {
                                    Some(buf) => {
                                        if sink.send(tokio_tungstenite::tungstenite::Message::Binary(buf.into())).await.is_err() {
                                            break;
                                        }
                                    }
                                    None => break,
                                }
                            }
                            _ = ping_interval.tick() => {
                                if sink.send(tokio_tungstenite::tungstenite::Message::Ping(vec![].into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    log::info!("[signalling] Connection closed, will reconnect");
                }
                Err(e) => {
                    log::warn!("[signalling] Connection failed: {e}, retrying in {backoff:?}");
                }
            }

            tokio::select! {
                _ = tokio::time::sleep(backoff) => {
                    backoff = (backoff * 2).min(max_backoff);
                }
            }
        }
    }

    pub async fn next(&mut self) -> Result<Message, SignallingError> {
        let msg_rx = self.msg_rx.as_mut().ok_or(SignallingError::NotConnected)?;
        msg_rx.recv().await.ok_or(SignallingError::Stopped)?
    }

    pub fn decode_message(data: &[u8]) -> Result<Message, SignallingError> {
        Message::decode(data).map_err(SignallingError::from)
    }
}

impl Drop for SignalingClient {
    fn drop(&mut self) {
        if let Some(handle) = self.run_handle.take() {
            handle.abort();
        }
    }
}
