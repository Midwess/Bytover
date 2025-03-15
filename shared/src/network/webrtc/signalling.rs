use std::{future::Future, pin::Pin};

use futures_util::{SinkExt, StreamExt};
use schema::devlog::rpc_signalling::server::Message as SignallingMessage;
use tokio::{sync::broadcast, task::JoinHandle};
use tokio_tungstenite::tungstenite::Message;

use crate::config::get_signalling_server_ws_url;
use prost::Message as prost_message;
use thiserror::Error;

pub type OnMessageFn = Box<
    dyn (FnMut(SignallingMessage) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>)
        + Send
        + Sync,
>;



#[derive(Debug, Error)]
pub enum RtcSignallingErrors {
    #[error("failed to connect to signalling server {:?}", .0)]
    SocketError(#[from] tokio_tungstenite::tungstenite::Error),
}

pub struct RtcsSignalling {
    send_task: JoinHandle<()>,
    receive_task: JoinHandle<()>,
    out_sender: broadcast::Sender<SignallingMessage>,
}

impl RtcsSignalling {
    pub async fn start() -> Result<Self, RtcSignallingErrors> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(get_signalling_server_ws_url()).await?;
        let (mut write, mut read) = ws_stream.split();

        let (mut out_sender, mut out_receiver) = broadcast::channel::<SignallingMessage>(16);

        let send_task = tokio::spawn(async move {
            while let Ok(message) = out_receiver.recv().await {
                write.send(Message::Binary(message.encode_to_vec().into())).await.unwrap();
            }
        });

        let receive_task = {
            let out_sender = out_sender.clone();
            tokio::spawn(async move {
                while let Some(Ok(message)) = read.next().await {
                    match message {
                        Message::Binary(data) => {
                            let Ok(message) = SignallingMessage::decode(&data[..]) else {
                                continue;
                            };

                            out_sender.send(message).unwrap();
                        }
                        _ => {}
                    }
                }
            })
        };

        Ok(Self {
            send_task,
            receive_task,
            out_sender,
        })
    }

    pub fn send(&self, message: SignallingMessage) -> Result<(), RtcSignallingErrors> {
        let _ = self.out_sender.send(message);
        Ok(())
    }

    pub fn subscribe(&self, mut callback: OnMessageFn) -> JoinHandle<()> 
    {
        let mut receiver = self.out_sender.subscribe();
        
        tokio::spawn(async move {
            while let Ok(message) = receiver.recv().await {
                callback(message).await;
            }
        })
    }
}
