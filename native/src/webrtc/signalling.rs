use std::sync::{Arc, OnceLock};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use schema::devlog::rpc_signalling::server::{
    AnswerMessage, IceConfig, Message,
};
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, OnceCell};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use core_services::utils::yield_container::{YieldContainer, YieldError};

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

#[derive(Clone)]
pub struct SignalingClient {
    host: String,
    port: u16,
    ssl: bool,
    msg_rx: Arc<OnceCell<YieldContainer<mpsc::Receiver<Result<Message, SignallingError>>>>>,
    msg_transfer_tx: Arc<OnceCell<mpsc::Sender<Vec<u8>>>>,
}

impl SignalingClient {
    pub fn new(host: String, port: u16, ssl: bool) -> Self {
        Self {
            host,
            port,
            ssl,
            msg_rx: Default::default(),
            msg_transfer_tx: Default::default(),
        }
    }

    fn url(&self) -> String {
        format!(
            "{}://{}:{}",
            if self.ssl { "wss" } else { "ws" },
            self.host,
            self.port
        )
    }

    fn http_url(&self) -> String {
        format!(
            "{}://{}:{}",
            if self.ssl { "https" } else { "http" },
            self.host,
            self.port
        )
    }

    pub async fn fetch_relay_config(&self, key: &str) -> Result<IceConfig, SignallingError> {
        let url = format!("{}/relay/{}", self.http_url(), key);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| SignallingError::HttpFailed(format!("{e}")))?;

        if !response.status().is_success() {
            return Err(SignallingError::HttpFailed(
                format!("relay endpoint returned {}", response.status()),
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| SignallingError::HttpFailed(format!("{e}")))?;

        IceConfig::decode(&bytes[..]).map_err(SignallingError::from)
    }

    pub fn start(&self) {
        let url = self.url();
        let (tx, new_rx) = mpsc::channel(128);
        let (transfer_tx, transfer_rx) = mpsc::channel(128);
        let _ = self.msg_rx.set(YieldContainer::new(new_rx));
        let _ = self.msg_transfer_tx.set(transfer_tx);

        tokio::spawn(async move {
            Self::run_loop(url, transfer_rx, tx).await;
        });
    }

    async fn run_loop(
        url: String,
        mut transfer_rx: mpsc::Receiver<Vec<u8>>,
        msg_tx: mpsc::Sender<Result<Message, SignallingError>>,
    ) {
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(30);

        loop {
            log::info!("[signalling] Connecting to {}", url);

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    log::info!("[signalling] Connected");
                    backoff = Duration::from_secs(1);
                    let (sink, stream) = ws_stream.split();
                    Self::read_messages(stream, sink, &mut transfer_rx, &msg_tx).await;
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
                _ = transfer_rx.recv() => {
                    break;
                }
            }
        }

        log::info!("[signalling] Run loop ended");
    }

    async fn read_messages(
        mut stream: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        mut sink: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, tokio_tungstenite::tungstenite::Message>,
        transfer_rx: &mut mpsc::Receiver<Vec<u8>>,
        msg_tx: &mpsc::Sender<Result<Message, SignallingError>>,
    ) {
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
            }
        }
    }

    pub async fn next(&self) -> Result<Message, SignallingError> {
        let msg_rx = self.msg_rx.get().ok_or(SignallingError::NotConnected)?;
        let mut msg_rx = msg_rx.retrieve().await?;
        msg_rx.recv().await.ok_or(SignallingError::Stopped)?
    }

    pub async fn send_answer(
        &self,
        sdp: String,
        request_id: &str,
    ) -> Result<(), SignallingError> {
        let msg = Message {
            request_id: Some(request_id.to_string()),
            answer: Some(AnswerMessage { sdp }),
            ..Default::default()
        };
        self.send_message(&msg).await
    }

    async fn send_message(&self, msg: &Message) -> Result<(), SignallingError> {
        let tx = self.msg_transfer_tx.get().ok_or(SignallingError::NotConnected)?;
        let mut buf = Vec::new();
        msg.encode(&mut buf).map_err(|e| {
            SignallingError::ProtocolError(format!("Failed to encode message: {e}"))
        })?;
        tx.send(buf).await.map_err(|_| SignallingError::NotConnected)?;
        Ok(())
    }

    pub fn decode_message(data: &[u8]) -> Result<Message, SignallingError> {
        Message::decode(data).map_err(SignallingError::from)
    }
}

