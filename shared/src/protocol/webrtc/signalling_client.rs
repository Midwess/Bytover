use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::protocol::webrtc::signalling::SharedContext;
use crate::shell::api::TimeoutReceiver;
use anyhow::anyhow;
use ewebsock::{connect, Options, WsEvent, WsMessage};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::executor::block_on;
use futures_timer::Delay;
use futures_util::lock::Mutex;
use futures_util::SinkExt;
use n0_future::task::{spawn, JoinHandle};
use n0_future::StreamExt;
use once_cell::sync::OnceCell;
use prost::Message as prost_message;
use schema::devlog::rpc_signalling::server::Message;
use std::sync::Arc;
use std::time::Duration;

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

                let drained_messages = signal_receiver.drain().collect::<Vec<_>>().await;
                log::info!("websocket disconnected, draining {} messages", drained_messages.len());
                log::info!("websocket disconnected, notifying all peers to cancel");
                context.remove_all().await;
            }
        });

        let _ = self.signal.set(signal_sender);
        let _ = self.handle.set(handle);
        Ok(())
    }

    pub async fn try_next_message(&self) -> Result<Option<Message>, WebRtcErrors> {
        let Ok(msg) = self.receiver.lock().await.try_next() else {
            return Ok(None)
        };

        if let Some(msg) = msg {
            return Ok(Some(msg))
        }

        Err(WebRtcErrors::SignallingClientError(anyhow!("Channel has been closed")))
    }

    pub async fn send(&self, msg: Message) -> Result<(), WebRtcErrors> {
        if let Some(signal_sender) = self.signal.get() {
            let _ = signal_sender.unbounded_send(msg);
        };

        Ok(())
    }

    pub fn append_msg(&self, msg: Message) -> Result<(), WebRtcErrors> {
        let _ = self.sender.unbounded_send(msg);
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
