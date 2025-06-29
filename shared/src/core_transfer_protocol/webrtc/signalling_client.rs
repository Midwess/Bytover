use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use ewebsock::{connect, ws_connect, Options, WsEvent, WsMessage};
use schema::devlog::rpc_signalling::server::Message;
use tokio::sync::{mpsc, Mutex};
use crate::core_transfer_protocol::webrtc::errors::WebRtcErrors;
use matchbox_socket::Signaller;
use tokio::signal::unix::Signal;
use tokio::task::{spawn_blocking, spawn_local, JoinHandle};
use prost::Message as prost_message;

pub struct SignallingClient {
    socket_addr: String,
    handle: Option<JoinHandle<()>>,
    sender: mpsc::Sender<Message>,
    receiver: Mutex<mpsc::Receiver<Message>>,
    signal: Option<mpsc::Sender<Message>>,
}

impl SignallingClient {
    pub fn new(socket_addr: String) -> Self {
        let (sender, receiver) = mpsc::channel(64);
        Self {
            socket_addr,
            handle: None,
            sender,
            receiver: Mutex::new(receiver),
            signal: None
        }
    }

    pub async fn start(&mut self) -> Result<(), WebRtcErrors> {
        self.stop().await;

        let (signal_sender, mut signal_receiver) = mpsc::channel::<Message>(10);
        let mut options = Options::default();
        options.read_timeout = Some(Duration::from_secs(60));

        let msg_sender = self.sender.clone();
        let addr = self.socket_addr.clone();
        let handle = tokio::spawn(async move {
            loop {
                let (mut sender, receiver) = match connect(addr.clone(), options.clone()) {
                    Ok(socket) => socket,
                    Err(err) => {
                        log::error!("websocket error, retrying... {:?}", err);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                };

                let receiver = Arc::new(Mutex::new(receiver));

                loop {
                    let receiver = receiver.clone();
                    tokio::select! {
                        Ok(Some(WsEvent::Message(WsMessage::Binary(bytes)))) = spawn_local(async move {
                            receiver.lock().await.try_recv()
                        }) => {
                            let Ok(msg) = Message::decode(&bytes[..]) else {
                                continue;
                            };

                            let _ = msg_sender.send(msg).await;
                        },
                        Some(msg_to_send) = signal_receiver.recv() => {
                            let mut bytes = vec![];
                            let _ = msg_to_send.encode(&mut bytes);
                            if bytes.is_empty() {
                                continue;
                            }

                            let _ = sender.send(WsMessage::Binary(bytes));
                        }
                    }
                }
            }
        });

        self.signal = Some(signal_sender);
        self.handle = Some(handle);
        Ok(())
    }

    pub async fn next_message(&self) -> Result<Message, WebRtcErrors> {
        let Some(msg) = self.receiver.lock().await.recv().await else {
            return Err(WebRtcErrors::SignallingClientError(anyhow!("Channel has been closed")))
        };

        Ok(msg)
    }

    pub async fn send(&self, msg: Message) -> Result<(), WebRtcErrors> {
        if let Some(signal_sender) = self.signal.as_ref() {
            let _ = signal_sender.send(msg).await;
        };

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            self.signal = None;
            let _ = self.sender.closed().await;
        }
    }
}

impl Drop for SignallingClient {
    fn drop(&mut self) {
        let Some(handle) = self.handle.take() else {
            return;
        };

        handle.abort();
    }
}
