use crate::core_transfer_protocol::webrtc::errors::WebRtcErrors;
use anyhow::anyhow;
use ewebsock::{connect, Options, WsEvent, WsMessage};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_timer::Delay;
use futures_util::lock::Mutex;
use futures_util::{SinkExt, StreamExt};
use n0_future::task::{spawn, JoinHandle};
use n0_future::time::Instant;
use prost::Message as prost_message;
use schema::devlog::rpc_signalling::server::Message;
use std::sync::Arc;
use std::time::Duration;
use crate::core_api::TimeoutReceiver;

pub struct SignallingClient {
    socket_addr: String,
    handle: Option<JoinHandle<()>>,
    sender: UnboundedSender<Message>,
    receiver: Mutex<UnboundedReceiver<Message>>,
    signal: Option<UnboundedSender<Message>>
}

impl SignallingClient {
    pub fn new(socket_addr: String) -> Self {
        let (sender, receiver) = unbounded();
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

        let (signal_sender, mut signal_receiver) = unbounded::<Message>();
        let mut options = Options::default();
        options.read_timeout = Some(Duration::from_secs(60));

        let mut msg_sender = self.sender.clone();
        let addr = self.socket_addr.clone();
        let min_keep_alive = Duration::from_secs(3);
        let mut last_keep_alive = None::<Instant>;
        let handle = spawn(async move {
            loop {
                log::info!("Starting signalling client at {addr}");
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
                    Delay::new(Duration::from_millis(50)).await;

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

                            log::info!("received message: {:?}", msg);
                            let _ = msg_sender.send(msg).await;
                            continue;
                        }

                        if let WsEvent::Closed = msg {
                            log::info!("websocket closed");
                            break;
                        }

                        if let WsEvent::Error(err) = msg {
                            log::error!("websocket error: {err:?}");
                            break;
                        }
                    }

                    if !connected {
                        continue;
                    }

                    if let Some(msg_to_send) = signal_receiver.poll_next_now() {
                        if msg_to_send.join.is_some() {
                            if let Some(last) = last_keep_alive {
                                if last.elapsed() <= min_keep_alive {
                                    // We avoid sending too much keep a live message
                                    continue;
                                }
                            }
                        }

                        let mut bytes = vec![];
                        let _ = msg_to_send.encode(&mut bytes);
                        if bytes.is_empty() {
                            continue;
                        }

                        sender.send(WsMessage::Binary(bytes));
                        log::info!("sent message: {:?}", msg_to_send);
                        last_keep_alive = Some(Instant::now());
                    }
                }
            }
        });

        self.signal = Some(signal_sender);
        self.handle = Some(handle);
        Ok(())
    }

    pub async fn next_message(&self) -> Result<Message, WebRtcErrors> {
        let Some(msg) = self.receiver.lock().await.next().await else {
            return Err(WebRtcErrors::SignallingClientError(anyhow!("Channel has been closed")))
        };

        Ok(msg)
    }

    pub async fn send(&self, msg: Message) -> Result<(), WebRtcErrors> {
        if let Some(signal_sender) = self.signal.as_ref() {
            let _ = signal_sender.unbounded_send(msg);
        };

        Ok(())
    }

    pub async fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
            self.signal = None;
            let _ = self.sender.close().await;
            self.sender.close_channel();
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
