use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::shell::api::TimeoutReceiver;
use anyhow::anyhow;
use ewebsock::{connect, Options, WsEvent, WsMessage};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_timer::Delay;
use futures_util::lock::Mutex;
use futures_util::{SinkExt, StreamExt};
use n0_future::task::{spawn, JoinHandle};
use once_cell::sync::OnceCell;
use prost::Message as prost_message;
use schema::devlog::rpc_signalling::server::{LeftMessage, Message};
use std::sync::Arc;
use std::time::Duration;
use futures::executor::block_on;
use crate::protocol::webrtc::signalling::SharedContext;

pub struct SignallingClient {
    socket_addr: String,
    handle: OnceCell<JoinHandle<()>>,
    sender: UnboundedSender<Message>,
    receiver: Mutex<UnboundedReceiver<Message>>,
    signal: OnceCell<UnboundedSender<Message>>
}

impl SignallingClient {
    pub fn new(socket_addr: String) -> Self {
        let (sender, receiver) = unbounded();
        Self {
            socket_addr,
            sender,
            receiver: Mutex::new(receiver),
            handle: Default::default(),
            signal: Default::default()
        }
    }

    pub async fn start(&self, context: SharedContext) -> Result<(), WebRtcErrors> {
        let (signal_sender, mut signal_receiver) = unbounded::<Message>();
        let mut options = Options::default();
        options.read_timeout = Some(Duration::from_secs(60));

        let mut msg_sender = self.sender.clone();
        let addr = self.socket_addr.clone();
        let mut left_signal_sender = signal_sender.clone();
        let handle = spawn(async move {
            loop {
                let (mut sender, receiver) = match connect(addr.clone(), options.clone()) {
                    Ok(socket) => socket,
                    Err(err) => {
                        log::error!("websocket error, retrying... {err:?}");
                        Delay::new(Duration::from_secs(3)).await;
                        continue;
                    }
                };

                let receiver = Arc::new(Mutex::new(receiver));

                let mut connected = false;
                loop {
                    let receiver = receiver.clone();

                    Delay::new(Duration::from_millis(20)).await;

                    let msg_opt = {
                        let receiver = receiver.lock().await;
                        receiver.try_recv()
                    };

                    if let Some(msg) = msg_opt {
                        if let WsEvent::Opened = msg {
                            connected = true;
                            log::info!("websocket opened");
                            continue;
                        }

                        if let WsEvent::Message(WsMessage::Binary(bytes)) = msg {
                            let Ok(msg) = Message::decode(&bytes[..]) else {
                                continue;
                            };

                            let _ = msg_sender.send(msg).await;
                            continue;
                        }

                        if let WsEvent::Closed = msg {
                            log::info!("websocket closed");
                            Delay::new(Duration::from_secs(3)).await;
                            break;
                        }

                        if let WsEvent::Error(err) = msg {
                            log::error!("websocket error: {err:?}");
                            Delay::new(Duration::from_secs(3)).await;
                            break;
                        }
                    }

                    if !connected {
                        continue;
                    }

                    if let Some(msg_to_send) = signal_receiver.poll_next_now() {
                        let mut bytes = vec![];
                        let _ = msg_to_send.encode(&mut bytes);
                        if bytes.is_empty() {
                            continue;
                        }

                        sender.send(WsMessage::Binary(bytes));
                    }
                }

                // When it goes here, the websocket was already being disconnected, we need to notify all peers to cancel
                log::info!("websocket disconnected, notifying all peers to cancel");
                let removed_peers = context.remove_all().await;
                for peer_id in removed_peers {
                    let _ = left_signal_sender.send(Message {
                        left_message: Some(LeftMessage {
                            id: peer_id.to_string()
                        }),
                        ..Default::default()
                    }).await;
                }
            }
        });

        let _ = self.signal.set(signal_sender);
        let _ = self.handle.set(handle);
        Ok(())
    }

    pub async fn next_message(&self) -> Result<Message, WebRtcErrors> {
        let Some(msg) = self.receiver.lock().await.next().await else {
            return Err(WebRtcErrors::SignallingClientError(anyhow!("Channel has been closed")))
        };

        Ok(msg)
    }

    pub async fn send(&self, msg: Message) -> Result<(), WebRtcErrors> {
        if let Some(signal_sender) = self.signal.get() {
            let _ = signal_sender.unbounded_send(msg);
        };

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            let _ = self.sender.close().await;
            self.sender.close_channel();
        }
    }
}

impl Drop for SignallingClient {
    fn drop(&mut self) {
        log::info!("Signalling client dropped, aborting websocket");
        block_on(async { self.stop().await })
    }
}
