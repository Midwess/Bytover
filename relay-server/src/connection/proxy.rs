use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use str0m::Event;
use std::collections::HashMap;

use crate::connection::rtc::{RelayRtcClient, RelayRtcError};
use schema::devlog::bitbridge::DataChannel;

pub struct ProxyInstance {
    pub session_id: String,
    leg1: Mutex<Option<RelayRtcClient>>,
    leg2: Mutex<Option<RelayRtcClient>>,
    notify_leg2: Notify,
}

impl ProxyInstance {
    pub fn new(session_id: String) -> Arc<Self> {
        Arc::new(Self {
            session_id,
            leg1: Mutex::new(None),
            leg2: Mutex::new(None),
            notify_leg2: Notify::new(),
        })
    }

    pub async fn init(
        self: &Arc<Self>,
        sdp_offer: String,
        channels: Vec<DataChannel>,
        proxies: Arc<Mutex<HashMap<String, Arc<ProxyInstance>>>>
    ) -> Result<String, RelayRtcError> {
        log::info!("[relay-server] Initializing ProxyInstance for session {}", self.session_id);
        let (client, answer_sdp) = RelayRtcClient::accept_offer(&sdp_offer, channels).await?;
        
        *self.leg1.lock().await = Some(client);

        let self_clone = self.clone();
        tokio::spawn(async move {
            self_clone.run(proxies).await;
        });

        Ok(answer_sdp)
    }

    pub async fn proxy(
        self: &Arc<Self>,
        sdp_offer: String,
        channels: Vec<DataChannel>,
    ) -> Result<String, RelayRtcError> {
        log::info!("[relay-server] Proxying leg 2 for session {}", self.session_id);
        let (client, answer_sdp) = RelayRtcClient::accept_offer(&sdp_offer, channels).await?;
        
        *self.leg2.lock().await = Some(client);
        self.notify_leg2.notify_one();

        Ok(answer_sdp)
    }

    async fn run(self: Arc<Self>, proxies: Arc<Mutex<HashMap<String, Arc<ProxyInstance>>>>) {
        let session_id = self.session_id.clone();
        log::info!("[relay-server] Starting transparent forwarding loop for session {}", session_id);

        let mut leg1 = self.leg1.lock().await.take().expect("Leg 1 must exist on run()");
        
        let timeout = std::time::Duration::from_secs(300); // 5 mins timeout
        if let Err(e) = tokio::time::timeout(timeout, leg1.wait_for_connected()).await {
            log::error!("[relay-server] Leg 1 failed to connect (timeout = {}): {:?}", timeout.as_secs(), e);
            proxies.lock().await.remove(&session_id);
            return;
        }

        log::info!("[relay-server] Leg 1 connected for session {}", session_id);

        // Now we wait for Leg 2 to join. While waiting, we poll Leg 1 to keep it alive.
        let mut leg2 = loop {
            tokio::select! {
                res1 = leg1.process_step() => {
                    match res1 {
                        Ok(Some(Event::ChannelData(_data))) => {
                            log::info!("[relay-server] Dropped early data from leg 1");
                        }
                        Ok(Some(event)) => log::info!("[relay-server] Leg 1 Event: {:?}", event),
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("[relay-server] Leg 1 disconnect/error while waiting for Leg 2: {:?}", e);
                            proxies.lock().await.remove(&session_id);
                            return;
                        }
                    }
                }
                _ = self.notify_leg2.notified() => {
                    log::info!("[relay-server] Leg 2 joined session {}, attaching to run loop", session_id);
                    break self.leg2.lock().await.take().unwrap();
                }
            }
        };

        // Wait for Leg 2 to connect, while simultaneously polling Leg 1
        let leg2_connected = {
            let wait_fut = leg2.wait_for_connected();
            tokio::pin!(wait_fut);
            
            tokio::time::timeout(timeout, async {
                loop {
                    tokio::select! {
                        res1 = leg1.process_step() => {
                            match res1 {
                                Ok(Some(Event::ChannelData(_data))) => {
                                    log::info!("[relay-server] Dropped early data from leg 1");
                                }
                                Ok(Some(event)) => log::info!("[relay-server] Leg 1 Event: {:?}", event),
                                Ok(None) => {}
                                Err(e) => {
                                    log::warn!("[relay-server] Leg 1 disconnect/error while Leg 2 connecting: {:?}", e);
                                    return Err(e);
                                }
                            }
                        }
                        res2_res = &mut wait_fut => {
                            return if let Err(e) = res2_res {
                                Err(e)
                            } else {
                                Ok(())
                            };
                        }
                    }
                }
            }).await
        };

        match leg2_connected {
            Ok(Ok(_)) => {
                log::info!("[relay-server] Leg 2 connected for session {}", session_id);
            }
            res => {
                log::error!("[relay-server] Leg 2 failed to connect or Leg 1 dropped: {:?}", res);
                proxies.lock().await.remove(&session_id);
                return;
            }
        }

        log::info!("[relay-server] Both legs connected. Active forwarding loop starting for {}", session_id);

        loop {
            tokio::select! {
                res1 = leg1.process_step() => {
                    match res1 {
                        Ok(Some(Event::ChannelData(data))) => {
                            if !leg2.send(&data.data, data.id) {
                                log::warn!("[relay-server] Failed to forward data to leg 2 on channel {:?}", data.id);
                            }
                        }
                        Ok(Some(event)) => log::info!("[relay-server] Leg 1 Event: {:?}", event),
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("[relay-server] Leg 1 disconnect/error: {:?}", e);
                            break;
                        }
                    }
                }
                res2 = leg2.process_step() => {
                    match res2 {
                        Ok(Some(Event::ChannelData(data))) => {
                            if !leg1.send(&data.data, data.id) {
                                log::warn!("[relay-server] Failed to forward data to leg 1 on channel {:?}", data.id);
                            }
                        }
                        Ok(Some(event)) => log::info!("[relay-server] Leg 2 Event: {:?}", event),
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("[relay-server] Leg 2 disconnect/error: {:?}", e);
                            break;
                        }
                    }
                }
            }
        }

        log::info!("[relay-server] Tearing down proxy instance {}", session_id);
        proxies.lock().await.remove(&session_id);
    }
}
