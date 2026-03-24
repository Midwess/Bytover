use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use schema::devlog::rpc_signalling::server::{
    AnswerMessage, IceCandidate, IceCandidateUpdateMessage, Message,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
pub type SignallingSink = SplitSink<WsStream, tungstenite::Message>;
pub type SignallingStream = SplitStream<WsStream>;

#[derive(Debug, Error)]
pub enum SignallingError {
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(#[from] tungstenite::Error),

    #[error("Signaling protocol error: {0}")]
    ProtocolError(String),

    #[error("Protobuf decode error: {0}")]
    DecodeFailed(#[from] prost::DecodeError),

    #[error("Not connected")]
    NotConnected,

    #[error("Signalling task stopped")]
    Stopped,
}

pub struct SignalingClient {
    host: String,
    port: u16,
    ssl: bool,
    sink: Arc<Mutex<Option<SignallingSink>>>,
    msg_rx: Mutex<mpsc::Receiver<Result<Message, SignallingError>>>,
    shutdown_tx: Arc<tokio::sync::watch::Sender<bool>>,
}

impl SignalingClient {
    pub fn new(host: String, port: u16, ssl: bool) -> Self {
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let (msg_tx, msg_rx) = mpsc::channel(128);
        let msg_rx = Mutex::new(msg_rx);
        Self {
            host,
            port,
            ssl,
            sink: Arc::new(Mutex::new(None)),
            msg_rx,
            shutdown_tx: Arc::new(shutdown_tx),
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

    pub fn start(&self) {
        let url = self.url();
        let shutdown_rx = Arc::clone(&self.shutdown_tx).subscribe();
        let sink = Arc::clone(&self.sink);
        let msg_tx = {
            let mut guard = self.msg_rx.lock().unwrap();
            let (tx, new_rx) = mpsc::channel(128);
            *guard = new_rx;
            tx
        };

        tokio::spawn(async move {
            Self::run_loop(url, shutdown_rx, sink, msg_tx).await;
        });
    }

    async fn run_loop(
        url: String,
        mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
        sink: Arc<Mutex<Option<SignallingSink>>>,
        msg_tx: mpsc::Sender<Result<Message, SignallingError>>,
    ) {
        let mut backoff = Duration::from_secs(1);
        let max_backoff = Duration::from_secs(30);

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        log::info!("[signalling] Shutdown signal received");
                        break;
                    }
                }
            }

            log::info!("[signalling] Connecting to {}", url);

            match connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    log::info!("[signalling] Connected");
                    backoff = Duration::from_secs(1);

                    let (write, read) = ws_stream.split();
                    *sink.lock().unwrap() = Some(write);

                    Self::read_messages(read, &msg_tx, &sink).await;
                    *sink.lock().unwrap() = None;
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
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
            }
        }

        log::info!("[signalling] Run loop ended");
    }

    async fn read_messages(
        mut stream: SignallingStream,
        msg_tx: &mpsc::Sender<Result<Message, SignallingError>>,
        _sink: &Arc<Mutex<Option<SignallingSink>>>,
    ) {
        loop {
            tokio::select! {
                msg = stream.next() => {
                    match msg {
                        Some(Ok(tungstenite::Message::Binary(data))) => {
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
                        Some(Ok(tungstenite::Message::Close(_))) | None => {
                            break;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            log::warn!("[signalling] Stream error: {e}");
                            break;
                        }
                    }
                }
            }
        }
    }

    pub async fn next(&self) -> Result<Message, SignallingError> {
        let mut rx = self.msg_rx.lock().unwrap();
        rx.recv().await.ok_or(SignallingError::Stopped)?
    }

    pub async fn send_answer(
        &self,
        to_id: String,
        sdp: String,
        scopes: Vec<String>,
        version: String,
        from_id: String,
    ) -> Result<(), SignallingError> {
        let msg = Message {
            scopes,
            from_id,
            to_id: Some(to_id),
            answer: Some(AnswerMessage { sdp }),
            version,
            ..Default::default()
        };
        self.send_message(&msg).await
    }

    pub async fn send_ice_candidate(
        &self,
        to_id: String,
        candidate: String,
        scopes: Vec<String>,
        version: String,
        from_id: String,
    ) -> Result<(), SignallingError> {
        let msg = Message {
            scopes,
            from_id,
            to_id: Some(to_id),
            ice_candidate_update: Some(IceCandidateUpdateMessage {
                ice_candidates: IceCandidate { candidate },
            }),
            version,
            ..Default::default()
        };
        self.send_message(&msg).await
    }

    async fn send_message(&self, msg: &Message) -> Result<(), SignallingError> {
        let mut buf = Vec::new();
        msg.encode(&mut buf).map_err(|e| {
            SignallingError::ProtocolError(format!("Failed to encode message: {e}"))
        })?;

        let mut sink_guard = self.sink.lock().unwrap();
        let mut sink = sink_guard.take().ok_or(SignallingError::NotConnected)?;
        drop(sink_guard);

        sink.send(tungstenite::Message::Binary(buf.into())).await?;

        let mut guard = self.sink.lock().unwrap();
        *guard = Some(sink);

        Ok(())
    }

    pub fn decode_message(data: &[u8]) -> Result<Message, SignallingError> {
        Message::decode(data).map_err(SignallingError::from)
    }
}

impl Drop for SignalingClient {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(true);
    }
}
