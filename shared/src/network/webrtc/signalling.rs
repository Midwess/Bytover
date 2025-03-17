use std::future::Future;
use std::pin::Pin;

use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use schema::devlog::rpc_signalling::server::Message as SignallingMessage;
use tokio::net::TcpStream;
use tokio::spawn;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::config::get_signalling_server_ws_url;
use prost::Message as prost_message;
use thiserror::Error;

pub type OnMessageFn = Box<dyn (FnMut(SignallingMessage) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>) + Send + Sync>;

#[derive(Debug, Error)]
pub enum RtcSignallingErrors {
    #[error("failed to connect to signalling server {:?}", .0)]
    SocketError(#[from] tokio_tungstenite::tungstenite::Error)
}

pub struct RtcsSignalling {
    receive_task: JoinHandle<()>,
    out_sender: broadcast::Sender<SignallingMessage>,
    server_writer: Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>
}

impl RtcsSignalling {
    pub async fn start() -> Result<Self, RtcSignallingErrors> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(get_signalling_server_ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        let (out_sender, mut out_receiver) = broadcast::channel::<SignallingMessage>(16);

        let receive_task = {
            let out_sender = out_sender.clone();
            tokio::spawn(async move {
                while let Some(Ok(message)) = read.next().await {
                    if let Message::Binary(data) = message {
                        let Ok(message) = SignallingMessage::decode(&data[..]) else {
                            continue;
                        };

                        out_sender.send(message).unwrap();
                    }
                }
            })
        };

        Ok(Self {
            receive_task,
            out_sender,
            server_writer: Mutex::new(write)
        })
    }

    pub async fn send(&self, message: SignallingMessage) -> Result<(), RtcSignallingErrors> {
        let mut writer = self.server_writer.lock().await;
        writer.send(Message::Binary(message.encode_to_vec().into())).await?;
        Ok(())
    }

    pub fn subscribe(&self, mut callback: OnMessageFn) -> JoinHandle<()> {
        let mut receiver = self.out_sender.subscribe();

        tokio::spawn(async move {
            while let Ok(message) = receiver.recv().await {
                callback(message).await;
            }
        })
    }
}

impl Drop for RtcsSignalling {
    fn drop(&mut self) {
        self.receive_task.abort();
    }
}
