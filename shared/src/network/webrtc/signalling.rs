use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use schema::devlog::rpc_signalling::server::Message as SignallingMessage;
use tokio::net::TcpStream;
use tokio::sync::broadcast::Receiver;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::config::get_signalling_server_ws_url;
use prost::Message as prost_message;
use thiserror::Error;

pub type OnMessageFn = Box<dyn (FnMut(SignallingMessage) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

#[derive(Debug, Error)]
pub enum RtcSignallingErrors {
    #[error("failed to connect to signalling server {:?}", .0)]
    SocketError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("failed to send message to signalling server")]
    SendError(String)
}

pub struct RtcsSignalling {
    msg_broadcast: broadcast::Sender<SignallingMessage>,
    server_writer: Arc<Mutex<Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>>,
    join_handle: JoinHandle<()>
}

impl RtcsSignalling {
    pub async fn start() -> Result<Self, RtcSignallingErrors> {
        let (msg_broadcast, _) = broadcast::channel::<SignallingMessage>(128);

        let server_writer = Arc::new(Mutex::new(None));
        let join_handle = {
            let server_writer = server_writer.clone();
            let msg_broadcast = msg_broadcast.clone();
            tokio::spawn(async move {
                let mut retry_ticker = tokio::time::interval(Duration::from_secs(5));
                loop {
                    let mut new_server_writer = server_writer.lock().await;
                    let Ok(Ok((ws_stream, _))) = timeout(
                        Duration::from_secs(10),
                        tokio_tungstenite::connect_async(get_signalling_server_ws_url())
                    )
                    .await
                    else {
                        log::error!(target: "rtc-signalling", "Socket is not connected, retrying...");
                        drop(new_server_writer);
                        retry_ticker.tick().await;
                        continue;
                    };

                    let (write, mut read) = ws_stream.split();
                    *new_server_writer = Some(write);
                    drop(new_server_writer);

                    log::info!(target: "rtc-signalling", "Connected to signalling server");
                    let mut ping_alive_ticker = tokio::time::interval(Duration::from_secs(3));
                    loop {
                        tokio::select! {
                            _ = ping_alive_ticker.tick() => {
                                let mut server_writer = server_writer.lock().await;
                                if let Some(write) = server_writer.as_mut() {
                                    if let Err(e) = write.send(Message::Ping(Bytes::from_static(&[1]))).await {
                                        log::error!(target: "rtc-signalling", "Failed to send ping to signalling server: {:?}", e);
                                    }
                                }
                            }
                            message = read.next() => {
                                let Some(Ok(message)) = message else {
                                    break;
                                };

                                if let Message::Binary(data) = message {
                                    let Ok(message) = SignallingMessage::decode(&data[..]) else {
                                        log::error!(target: "rtc-signalling", "Failed to decode message");
                                        continue;
                                    };

                                    if let Err(e) = msg_broadcast.send(message) {
                                        log::error!(target: "rtc-signalling", "Failed to send message to broadcast: {:?}", e);
                                    }
                                }
                            }
                        }
                    }

                    let mut server_writer = server_writer.lock().await;
                    *server_writer = None;
                    log::error!(target: "rtc-signalling", "Socket is not connected, retrying...");
                    drop(server_writer);

                    retry_ticker.tick().await;
                }
            })
        };

        Ok(Self {
            join_handle,
            msg_broadcast,
            server_writer
        })
    }

    pub async fn send(&self, message: SignallingMessage) -> Result<(), RtcSignallingErrors> {
        let connection_timeout = Duration::from_secs(10);
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        let clock = tokio::time::Instant::now();
        while clock.elapsed() < connection_timeout {
            let writer = timeout(Duration::from_secs(1), self.server_writer.lock()).await;
            if let Ok(mut writer) = writer {
                if let Some(writer) = writer.as_mut() {
                    if let Err(e) = writer.send(Message::Binary(message.encode_to_vec().into())).await {
                        log::error!(target: "rtc-signalling", "Failed to send message to signalling server: {:?}", e);
                        continue;
                    }

                    return Ok(());
                }

                drop(writer);
            }

            log::info!(target: "rtc-signalling", "Waiting for socket to re-connect");
            ticker.tick().await;
        }

        Err(RtcSignallingErrors::SendError("Socket is not connected".to_string()))
    }

    pub fn subscribe(&self) -> Receiver<SignallingMessage> {
        self.msg_broadcast.subscribe()
    }
}

impl Drop for RtcsSignalling {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}
