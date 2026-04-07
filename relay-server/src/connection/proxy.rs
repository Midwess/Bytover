use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use str0m::channel::ChannelId;
use str0m::Event;

use crate::connection::rtc::{RelayRtcClient, RelayRtcError, PollOutcome};
use schema::devlog::bitbridge::DataChannel;
use core_services::utils::yield_container::{YieldContainer, Yieldable};


const STATS_TICK: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(45);

pub struct ProxyInstance {
    pub session_id: String,
    leg1: YieldContainer<Box<RelayRtcClient>>,
    leg2: YieldContainer<Box<RelayRtcClient>>,
    notify_leg2: Notify,
}

impl ProxyInstance {
    pub fn new(session_id: String) -> Arc<Self> {
        Arc::new(Self {
            session_id,
            leg1: YieldContainer::empty(),
            leg2: YieldContainer::empty(),
            notify_leg2: Notify::new(),
        })
    }

    pub async fn init(
        self: &Arc<Self>,
        sdp_offer: String,
        channels: Vec<DataChannel>,
    ) -> Result<String, RelayRtcError> {
        log::info!("[relay-server] Initializing ProxyInstance for session {}", self.session_id);
        let (client, answer_sdp) = RelayRtcClient::accept_offer(&sdp_offer, channels).await?;
        self.leg1.deposit(client).await.map_err(|_| {
            RelayRtcError::Socket(std::io::Error::new(std::io::ErrorKind::Other, "Already yielded"))
        })?;
        Ok(answer_sdp)
    }

    pub async fn proxy(
        self: &Arc<Self>,
        sdp_offer: String,
        channels: Vec<DataChannel>,
    ) -> Result<String, RelayRtcError> {
        log::info!("[relay-server] Proxying leg 2 for session {}", self.session_id);
        let (client, answer_sdp) = RelayRtcClient::accept_offer(&sdp_offer, channels).await?;
        self.leg2.deposit(client).await.map_err(|_| {
            RelayRtcError::Socket(std::io::Error::new(std::io::ErrorKind::Other, "Already yielded"))
        })?;
        self.notify_leg2.notify_one();
        Ok(answer_sdp)
    }

    pub async fn run(self: Arc<Self>) -> String {
        let session_id = self.session_id.clone();
        log::info!("[relay-server] Starting unified run loop for session {}", session_id);

        let mut leg1 = self.leg1.retrieve().await.expect("Leg 1 must exist on run()");
        let mut leg2_opt: Option<Yieldable<Box<RelayRtcClient>>> = None;

        let mut leg1_connected = false;
        let mut leg2_connected = false;
        let mut both_connected = false;

        let (tx_to_2, mut rx_to_2) = tokio::sync::mpsc::unbounded_channel::<(ChannelId, Vec<u8>)>();
        let (tx_to_1, mut rx_to_1) = tokio::sync::mpsc::unbounded_channel::<(ChannelId, Vec<u8>)>();
        let mut depth_to_2: usize = 0;
        let mut depth_to_1: usize = 0;
        let mut pending_to_2: Option<(ChannelId, Vec<u8>)> = None;
        let mut pending_to_1: Option<(ChannelId, Vec<u8>)> = None;

        let mut stats_tick = tokio::time::interval(STATS_TICK);
        stats_tick.tick().await;

        let far_future = Duration::from_secs(3600);
        let mut retry_to_2 = Box::pin(tokio::time::sleep(far_future));
        let mut retry_to_1 = Box::pin(tokio::time::sleep(far_future));

        let connect_deadline = tokio::time::sleep(CONNECT_TIMEOUT);
        tokio::pin!(connect_deadline);

        loop {
            let leg1_alive = leg1.is_alive();
            let leg2_alive = leg2_opt.as_ref().map(|l| l.is_alive()).unwrap_or(true);

            if !leg1_alive || !leg2_alive {
                if !leg1_alive && (leg2_opt.is_none() || !leg2_alive) {
                    log::info!("[relay-server] Both legs finished for session {}", session_id);
                    break;
                }

                if !leg1_alive {
                    if let Some(leg2) = leg2_opt.as_mut() {
                        if leg2.is_alive() {
                            leg2.disconnect();
                        }
                    }
                } else if !leg2_alive {
                    leg1.disconnect();
                }
            }

            let now = Instant::now();
            if leg1.is_alive() {
                let _ = leg1.handle_timeout(now);
            }
            if let Some(leg2) = leg2_opt.as_mut() {
                if leg2.is_alive() {
                    let _ = leg2.handle_timeout(now);
                }
            }

            // Drain pending output (Transmits) BEFORE entering select!.
            if leg1.is_alive() {
                loop {
                    match leg1.poll_output().await {
                        Ok(PollOutcome::Event(event)) => {
                            if let Event::ChannelData(data) = event {
                                depth_to_2 += 1;
                                if tx_to_2.send((data.id, data.data)).is_err() {
                                    log::warn!("[relay-server] tx_to_2 closed");
                                    return session_id;
                                }
                            } else {
                                log::debug!("[relay-server] Leg 1 Event: {:?}", event);
                            }
                            if !leg1_connected && leg1.is_fully_connected() {
                                leg1_connected = true;
                                log::info!("[relay-server] Leg 1 fully connected for session {}", session_id);
                                if leg2_connected {
                                    both_connected = true;
                                    log::info!("[relay-server] Both legs connected for session {}", session_id);
                                }
                            }
                        }
                        Ok(PollOutcome::MorePending) => continue,
                        Ok(PollOutcome::Idle(_)) => break,
                        Err(e) => {
                            log::warn!("[relay-server] Leg 1 drain error: {:?}", e);
                            leg1.disconnect();
                            if let Some(leg2) = leg2_opt.as_mut() {
                                leg2.disconnect();
                            }
                            break;
                        }
                    }
                }
            }

            if let Some(leg2) = leg2_opt.as_mut() {
                if leg2.is_alive() {
                    loop {
                        match leg2.poll_output().await {
                            Ok(PollOutcome::Event(event)) => {
                                if let Event::ChannelData(data) = event {
                                    depth_to_1 += 1;
                                    if tx_to_1.send((data.id, data.data)).is_err() {
                                        log::warn!("[relay-server] tx_to_1 closed");
                                        return session_id;
                                    }
                                } else {
                                    log::debug!("[relay-server] Leg 2 Event: {:?}", event);
                                }
                                if !leg2_connected && leg2.is_fully_connected() {
                                    leg2_connected = true;
                                    log::info!("[relay-server] Leg 2 fully connected for session {}", session_id);
                                    if leg1_connected {
                                        both_connected = true;
                                        log::info!("[relay-server] Both legs connected for session {}", session_id);
                                    }
                                }
                            }
                            Ok(PollOutcome::MorePending) => continue,
                            Ok(PollOutcome::Idle(_)) => break,
                            Err(e) => {
                                log::warn!("[relay-server] Leg 2 drain error: {:?}", e);
                                leg2.disconnect();
                                leg1.disconnect();
                                break;
                            }
                        }
                    }
                }
            }

            tokio::select! {
                res1 = leg1.process_step(), if leg1.is_alive() => {
                    match res1 {
                        Ok(Some(Event::ChannelData(data))) => {
                            depth_to_2 += 1;
                            if tx_to_2.send((data.id, data.data)).is_err() {
                                log::warn!("[relay-server] tx_to_2 closed");
                                break;
                            }

                        }
                        Ok(Some(event)) => log::debug!("[relay-server] Leg 1 Event: {:?}", event),
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("[relay-server] Leg 1 disconnect/error: {:?}", e);
                            leg1.disconnect();
                            if let Some(leg2) = leg2_opt.as_mut() {
                                leg2.disconnect();
                            }
                        }
                    }
                    if !leg1_connected && leg1.is_fully_connected() {
                        leg1_connected = true;
                        log::info!("[relay-server] Leg 1 fully connected for session {}", session_id);
                        if leg2_connected {
                            both_connected = true;
                            log::info!("[relay-server] Both legs connected for session {}", session_id);
                        }
                    }
                }

                res2 = async { leg2_opt.as_mut().unwrap().process_step().await }, if leg2_opt.as_ref().map(|l| l.is_alive()).unwrap_or(false) => {
                    match res2 {
                        Ok(Some(Event::ChannelData(data))) => {
                            depth_to_1 += 1;
                            if tx_to_1.send((data.id, data.data)).is_err() {
                                log::warn!("[relay-server] tx_to_1 closed");
                                break;
                            }

                        }
                        Ok(Some(event)) => log::debug!("[relay-server] Leg 2 Event: {:?}", event),
                        Ok(None) => {}
                        Err(e) => {
                            log::warn!("[relay-server] Leg 2 disconnect/error: {:?}", e);
                            if let Some(leg2) = leg2_opt.as_mut() {
                                leg2.disconnect();
                            }
                            leg1.disconnect();
                        }
                    }
                    if !leg2_connected {
                        if let Some(leg2) = leg2_opt.as_mut() {
                            if leg2.is_fully_connected() {
                                leg2_connected = true;
                                log::info!("[relay-server] Leg 2 fully connected for session {}", session_id);
                                if leg1_connected {
                                    both_connected = true;
                                    log::info!("[relay-server] Both legs connected for session {}", session_id);
                                }
                            }
                        }
                    }
                }

                _ = self.notify_leg2.notified(), if leg2_opt.is_none() => {
                    match self.leg2.retrieve().await {
                        Ok(c) => {
                            log::info!("[relay-server] Leg 2 attached to run loop for session {}", session_id);
                            leg2_opt = Some(c);
                        }
                        Err(_) => {
                            log::warn!("[relay-server] Failed to retrieve leg 2");
                            break;
                        }
                    }
                }

                Some((id, buf)) = rx_to_2.recv(), if leg2_connected && pending_to_2.is_none() && leg2_opt.as_ref().map(|l| l.is_alive()).unwrap_or(false) => {
                    if let Some(leg2) = leg2_opt.as_mut() {
                        if leg2.send(&buf, id) {
                            log::trace!("[relay-server] Forwarded data to leg 2 (ID: {:?}, len: {})", id, buf.len());
                            depth_to_2 = depth_to_2.saturating_sub(1);

                        } else {
                            pending_to_2 = Some((id, buf));
                            retry_to_2.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(3));
                        }
                    }
                }

                () = &mut retry_to_2, if pending_to_2.is_some() && leg2_connected && leg2_opt.as_ref().map(|l| l.is_alive()).unwrap_or(false) => {
                    if let (Some(leg2), Some((id, buf))) = (leg2_opt.as_mut(), pending_to_2.as_ref()) {
                        if leg2.send(buf, *id) {
                            log::trace!("[relay-server] Retried & forwarded data to leg 2 (ID: {:?})", id);
                            pending_to_2 = None;
                            depth_to_2 = depth_to_2.saturating_sub(1);
                            retry_to_2.as_mut().reset(tokio::time::Instant::now() + far_future);
                        } else {
                            retry_to_2.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(3));
                        }
                    }
                }

                Some((id, buf)) = rx_to_1.recv(), if leg1_connected && pending_to_1.is_none() && leg1.is_alive() => {
                    if leg1.send(&buf, id) {
                        log::trace!("[relay-server] Forwarded data to leg 1 (ID: {:?}, len: {})", id, buf.len());
                        depth_to_1 = depth_to_1.saturating_sub(1);

                    } else {
                        pending_to_1 = Some((id, buf));
                        retry_to_1.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(3));
                    }
                }

                () = &mut retry_to_1, if pending_to_1.is_some() && leg1_connected && leg1.is_alive() => {
                    if let Some((id, buf)) = pending_to_1.as_ref() {
                        if leg1.send(buf, *id) {
                            log::trace!("[relay-server] Retried & forwarded data to leg 1 (ID: {:?})", id);
                            pending_to_1 = None;
                            depth_to_1 = depth_to_1.saturating_sub(1);
                            retry_to_1.as_mut().reset(tokio::time::Instant::now() + far_future);
                        } else {
                            retry_to_1.as_mut().reset(tokio::time::Instant::now() + Duration::from_millis(3));
                        }
                    }
                }

                _ = &mut connect_deadline, if !both_connected => {
                    log::error!(
                        "[relay-server] Connect deadline ({}s) exceeded for session {}. leg1_connected={} leg2_connected={}",
                        CONNECT_TIMEOUT.as_secs(), session_id, leg1_connected, leg2_connected,
                    );
                    break;
                }

                _ = stats_tick.tick() => {
                    let d1 = leg1.download_rate_bps();
                    let u1 = leg1.upload_rate_bps();
                    if let Some(leg2) = leg2_opt.as_mut() {
                        let d2 = leg2.download_rate_bps();
                        let u2 = leg2.upload_rate_bps();
                        log::info!(
                            "[relay-server] stats {session_id} leg1 down={:.0}B/s up={:.0}B/s leg2 down={:.0}B/s up={:.0}B/s q(A->B)={} q(B->A)={}",
                            d1, u1, d2, u2, depth_to_2, depth_to_1,
                        );
                    } else {
                        log::info!(
                            "[relay-server] stats {session_id} leg1 down={:.0}B/s up={:.0}B/s (leg2 pending)",
                            d1, u1,
                        );
                    }
                }
            }
        }

        log::info!("[relay-server] Tearing down proxy instance {}", session_id);
        session_id
    }
}


