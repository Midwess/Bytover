use futures_util::StreamExt;
use schema::devlog::rpc_signalling::server::{IceConfig, OfferMessage};
use socket2::{Domain, Socket, Type};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{Protocol, Receive};
use str0m::config::TransportConfig as SctpTransportConfig;
use str0m::{Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use stun_proto::agent::Transmit;
use stun_proto::types::TransportType as StunTransportType;
use turn_client_proto::api::{BindChannelError, SendError, TurnClientApi};
use turn_client_proto::udp::{TurnEvent, TurnPollRet, TurnRecvRet};

use crate::config::is_relay_only;
use crate::webrtc::client::{WebRtcClientError, UDP_SOCKET_BUFFER_SIZE};
use crate::webrtc::ice::{strip_candidates_from_sdp, IceAgent};
use crate::webrtc::signalling::SignallingSender;
use crate::webrtc::turn::{stun_now, TurnRelayInfo};

pub const RELIABLE_STREAM_ID: u16 = 1;
pub const UNORDERED_MSG_STREAM_ID: u16 = 2;
pub const ORDERED_MSG_STREAM_ID: u16 = 3;

#[derive(Debug, Clone, Copy)]
pub struct ChannelIds {
    pub reliable: ChannelId,
    pub unordered_msg: ChannelId,
    pub ordered_msg: ChannelId,
}

/// How long ICE can stay in `Disconnected` before we treat it as a real disconnect.
/// str0m never emits Failed/Closed ICE states, so we need this timeout.
const ICE_DISCONNECT_TIMEOUT: Duration = Duration::from_secs(10);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum number of consecutive `MorePending` cycles.
const MAX_PENDING_SPINS_PER_STEP: usize = 8;

const RELIABLE_BUFFERED_AMOUNT_LOW_THRESHOLD: usize = 2 * 1024 * 1024;

const RELIABLE_BUFFERED_AMOUNT_REFILL_TARGET: usize = 4 * 1024 * 1024;

/// Events emitted from the RTC thread to the outside world.
pub enum RtcEvent {
    Str0mEvent(Event),
    Error(WebRtcClientError),
}

/// Outcome of a single poll operation.
#[derive(Debug)]
pub enum RtcOutcome {
    Event(Event),
    Idle(Instant),
    MorePending,
}

pub struct RtcHandle {
    event_rx: Option<tokio::sync::mpsc::Receiver<RtcEvent>>,
    data_tx: Option<tokio::sync::mpsc::Sender<(Vec<u8>, ChannelId)>>,
    channel_ids: ChannelIds,
    thread_handle: Option<std::thread::JoinHandle<()>>,
    data_buffered_amount: Arc<AtomicUsize>,
    is_relay: Arc<AtomicBool>,
}

impl RtcHandle {
    pub async fn connect(
        signalling_id: &str,
        offer_message: OfferMessage,
        me: schema::devlog::bitbridge::PeerMessage,
        signalling: SignallingSender,
        request_id: &str,
    ) -> Result<Self, WebRtcClientError> {
        RtcClient::connect(signalling_id, offer_message, me, signalling, request_id, None).await
    }

    pub async fn connect_with_config(
        signalling_id: &str,
        offer_message: OfferMessage,
        me: schema::devlog::bitbridge::PeerMessage,
        signalling: SignallingSender,
        request_id: &str,
        ice_config: IceConfig,
    ) -> Result<Self, WebRtcClientError> {
        RtcClient::connect(signalling_id, offer_message, me, signalling, request_id, Some(ice_config)).await
    }

    pub fn data_buffered_amount(&self) -> usize {
        self.data_buffered_amount.load(Ordering::Relaxed)
    }

    pub fn is_relay(&self) -> bool {
        self.is_relay.load(Ordering::Relaxed)
    }

    /// Await the next event from the RTC thread.
    pub async fn poll_event(&mut self) -> Option<RtcEvent> {
        let rx = self.event_rx.as_mut()?;
        rx.recv().await
    }

    /// Try to receive an event without blocking.
    pub fn try_poll_event(&mut self) -> Option<RtcEvent> {
        self.event_rx.as_mut().and_then(|rx| rx.try_recv().ok())
    }

    /// Remove and return the event receiver so an external task can own it.
    /// After this call, `poll_event` / `try_poll_event` return `None`.
    pub fn take_event_rx(&mut self) -> Option<tokio::sync::mpsc::Receiver<RtcEvent>> {
        self.event_rx.take()
    }

    /// Send data on a channel. Returns true if the command was queued.
    pub fn send(&self, data: &[u8], channel_id: ChannelId) -> bool {
        self.data_tx
            .as_ref()
            .is_some_and(|tx| tx.try_send((data.to_vec(), channel_id)).is_ok())
    }

    pub fn channel_ids(&self) -> &ChannelIds {
        &self.channel_ids
    }

    /// The handle is alive if the I/O thread is still running and data can still be queued.
    pub fn is_alive(&self) -> bool {
        self.data_tx.as_ref().is_some_and(|tx| !tx.is_closed()) && self.thread_handle.as_ref().is_some_and(|h| !h.is_finished())
    }

    pub fn shutdown(&mut self) {
        drop(self.data_tx.take());
        drop(self.event_rx.take());
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for RtcHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct RtcClient {
    rtc: RefCell<Rtc>,
    socket: tokio::net::UdpSocket,
    local_addr: SocketAddr,
    local_v4_addr: Option<SocketAddr>,
    local_v6_addr: Option<SocketAddr>,
    buf: Vec<u8>,
    cached_timeout: Instant,
    channel_ids: ChannelIds,
    pending_transmit: Option<(Vec<u8>, str0m::channel::ChannelId)>,
    early_events: Vec<Event>,
    pending_remote_candidates: VecDeque<str0m::Candidate>,
    candidate_rx: Option<tokio::sync::mpsc::Receiver<str0m::Candidate>>,
    turn: Option<TurnRelayInfo>,
    data_buffered_amount: Arc<AtomicUsize>,
    is_relay: Arc<AtomicBool>,
}

impl RtcClient {
    pub async fn connect(
        signalling_id: &str,
        offer_message: OfferMessage,
        me: schema::devlog::bitbridge::PeerMessage,
        signalling: SignallingSender,
        request_id: &str,
        ice_config_override: Option<IceConfig>,
    ) -> Result<RtcHandle, WebRtcClientError> {
        let config = match ice_config_override {
            Some(cfg) => {
                log::info!("[rtc-client] Using provided ice config with {} URLs", cfg.urls.len());
                cfg
            }
            None => match signalling.fetch_relay_config(signalling_id).await {
                Ok(cfg) => {
                    log::info!("[rtc-client] Using relay config with {} URLs", cfg.urls.len());
                    cfg
                }
                Err(e) => {
                    log::warn!("[rtc-client] Failed to fetch relay config, using P2P only: {}", e);
                    IceConfig::default()
                }
            },
        };

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.set_nonblocking(true)?;
        socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into())?;
        apply_udp_buffer_size(&socket, UDP_SOCKET_BUFFER_SIZE);
        let socket: std::net::UdpSocket = socket.into();
        let socket = tokio::net::UdpSocket::from_std(socket)?;

        let local_addr = socket.local_addr()?;

        let (candidates, turn_info) = IceAgent::gather_candidates(&socket, &config)
            .await
            .map_err(|e| WebRtcClientError::Signalling(e.to_string()))?;
        match &turn_info {
            Some(info) => log::info!(
                "[rtc-client] TURN relay available, relay addr: {}",
                info.relay_addr
            ),
            None => log::info!("[rtc-client] No TURN relay, operating P2P-only"),
        }

        let sctp_transport = SctpTransportConfig::default()
            .with_max_init_retransmits(None)
            .with_max_data_retransmits(None)
            .with_max_cwnd_bytes(Some(300_000));

        let mut rtc = RtcConfig::default()
            .set_sctp_max_message_size(256 * 1024)
            .set_sctp_buffer_size(5 * 1024 * 1024)
            .set_stats_interval(Some(std::time::Duration::from_secs(10)))
            .set_sctp_transport_config(sctp_transport)
            .build(Instant::now());
        let mut local_v4_addr = None;
        let mut local_v6_addr = None;
        for candidate in &candidates {
            if candidate.kind() != str0m::CandidateKind::Host {
                continue;
            }
            let addr = candidate.addr();
            if addr.is_ipv4() && local_v4_addr.is_none() {
                local_v4_addr = Some(addr);
            } else if addr.is_ipv6() && local_v6_addr.is_none() {
                local_v6_addr = Some(addr);
            }
        }
        for candidate in &candidates {
            if candidate.kind() != str0m::CandidateKind::ServerReflexive {
                continue;
            }
            let addr = candidate.addr();
            if addr.is_ipv4() && local_v4_addr.is_none() {
                local_v4_addr = Some(addr);
            } else if addr.is_ipv6() && local_v6_addr.is_none() {
                local_v6_addr = Some(addr);
            }
        }
        log::info!(
            "[rtc-client] Adding {} gathered candidates to RTC engine (preferred local v4={:?}, v6={:?})",
            candidates.len(),
            local_v4_addr,
            local_v6_addr
        );
        for candidate in candidates {
            log::debug!("[rtc-client] Adding candidate: {}", candidate);
            rtc.add_local_candidate(candidate);
        }

        let stripped_offer_sdp = strip_candidates_from_sdp(&offer_message.sdp);
        log::info!("Stripped offer SDP (candidates removed for early answer)");

        let offer = str0m::change::SdpOffer::from_sdp_string(&stripped_offer_sdp)
            .map_err(|e| WebRtcClientError::Sdp(e.to_string()))?;

        let reliable_id = rtc.direct_api().create_data_channel(ChannelConfig {
            label: "reliable".to_string(),
            ordered: false,
            negotiated: Some(RELIABLE_STREAM_ID),
            ..Default::default()
        });
        let unordered_msg_id = rtc.direct_api().create_data_channel(ChannelConfig {
            label: "unordered_msg".to_string(),
            ordered: false,
            negotiated: Some(UNORDERED_MSG_STREAM_ID),
            ..Default::default()
        });
        let ordered_msg_id = rtc.direct_api().create_data_channel(ChannelConfig {
            label: "ordered_msg".to_string(),
            ordered: true,
            negotiated: Some(ORDERED_MSG_STREAM_ID),
            ..Default::default()
        });
        let channel_ids = ChannelIds {
            reliable: reliable_id,
            unordered_msg: unordered_msg_id,
            ordered_msg: ordered_msg_id,
        };

        let answer = rtc
            .sdp_api()
            .accept_offer(offer)
            .map_err(|e| WebRtcClientError::Rtc(e.to_string()))?;

        let answer_sdp = answer.to_sdp_string();

        if is_relay_only() {
            let candidate_lines: Vec<&str> = answer_sdp.lines().filter(|l| l.starts_with("a=candidate:")).collect();
            log::info!("[rtc-client] Answer SDP contains {} candidate line(s):", candidate_lines.len());
            for line in &candidate_lines {
                log::info!("[rtc-client]   {}", line);
            }
            if candidate_lines.is_empty() {
                log::warn!("[rtc-client] WARNING: Answer SDP has NO candidates — offerer won't be able to connect!");
            }
        }

        let answer_sdp = if is_relay_only() {
            answer_sdp.replace("a=ice-options:trickle\r\n", "a=ice-options:trickle relay-only\r\n")
        } else {
            answer_sdp
        };
        let me_clone = me.clone();
        let request_id_owned = request_id.to_string();
        let original_sdp = offer_message.sdp.clone();

        let ip_candidates = IceAgent::parse_ip_based_candidates(&original_sdp);

        let (candidate_tx, candidate_rx) = tokio::sync::mpsc::channel::<str0m::Candidate>(16);

        tokio::spawn(async move {
            let stream = IceAgent::resolve_remote_candidates_stream(&original_sdp);
            futures_util::pin_mut!(stream);
            while let Some(candidate) = stream.next().await {
                log::debug!("[rtc-client] Resolved remote candidate: {}", candidate);
                if candidate_tx.send(candidate).await.is_err() {
                    break;
                }
            }
            log::info!("[rtc-client] Remote candidate resolution complete");
        });

        let mut client = Self {
            rtc: RefCell::new(rtc),
            socket,
            local_addr,
            local_v4_addr,
            local_v6_addr,
            buf: vec![0u8; 2000],
            cached_timeout: Instant::now(),
            channel_ids,
            pending_transmit: None,
            early_events: Vec::with_capacity(8),
            pending_remote_candidates: VecDeque::with_capacity(8),
            candidate_rx: Some(candidate_rx),
            turn: turn_info,
            data_buffered_amount: Arc::new(AtomicUsize::new(0)),
            is_relay: Arc::new(AtomicBool::new(false)),
        };

        for candidate in ip_candidates {
            client.add_or_defer_remote_candidate(candidate, "IP-based");
        }

        if let Err(e) = signalling.send_answer(answer_sdp, me_clone, &request_id_owned).await {
            log::warn!("[rtc-client] Failed to send answer: {}", e);
        }

        log::info!("[rtc-client] Answer sent, candidate resolution continuing in background");

        let connected = false;
        let client = client.wait_for_connected(connected).await?;
        log::info!("Connected to p2p");
        Ok(client.spawn_thread())
    }

    fn spawn_thread(self) -> RtcHandle {
        let channel_ids = self.channel_ids;
        let data_buffered_amount = Arc::clone(&self.data_buffered_amount);
        let is_relay = Arc::clone(&self.is_relay);
        let (event_tx, event_rx) = tokio::sync::mpsc::channel::<RtcEvent>(5);
        let (data_tx, data_rx) = tokio::sync::mpsc::channel::<(Vec<u8>, ChannelId)>(16);
        let thread_handle = std::thread::Builder::new()
            .name("rtc-io".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime for RTC thread");

                rt.block_on(async move {
                    Self::run_loop(self, event_tx, data_rx).await;
                });
            })
            .expect("Failed to spawn RTC I/O thread");

        RtcHandle {
            event_rx: Some(event_rx),
            data_tx: Some(data_tx),
            channel_ids,
            thread_handle: Some(thread_handle),
            data_buffered_amount,
            is_relay,
        }
    }

    /// The main event loop running on the dedicated OS thread.
    async fn run_loop(
        mut self,
        event_tx: tokio::sync::mpsc::Sender<RtcEvent>,
        mut data_rx: tokio::sync::mpsc::Receiver<(Vec<u8>, ChannelId)>,
    ) {
        log::info!("[rtc-client] RTC I/O thread started");

        for event in std::mem::take(&mut self.early_events) {
            log::info!("[rtc-client] Replaying early event: {:?}", event);
            if event_tx.send(RtcEvent::Str0mEvent(event)).await.is_err() {
                log::info!("[rtc-client] Event receiver dropped during early event replay");
                return;
            }
        }

        let mut ice_disconnected_since: Option<Instant> = None;

        let mut pending_spins = 0usize;
        while self.rtc_mut().is_alive() {
            if let Some(since) = ice_disconnected_since {
                if since.elapsed() >= ICE_DISCONNECT_TIMEOUT {
                    log::info!("[rtc-client] ICE disconnected for {:?}, tearing down", ICE_DISCONNECT_TIMEOUT);
                    self.rtc_mut().disconnect();
                    break;
                }
            }

            self.flush_pending_transmit();

            let outcome = match self.poll_event().await {
                Ok(o) => o,
                Err(e) => {
                    log::warn!("[rtc-client] RTC poll_event error: {e:?}");
                    let _ = event_tx.send(RtcEvent::Error(e)).await;
                    return;
                }
            };

            match outcome {
                RtcOutcome::Event(event) => {
                    pending_spins = 0;
                    if let Event::IceConnectionStateChange(state) = &event {
                        match state {
                            IceConnectionState::Disconnected => {
                                if ice_disconnected_since.is_none() {
                                    log::info!("[rtc-client] ICE disconnected, starting {:?} timeout", ICE_DISCONNECT_TIMEOUT);
                                    ice_disconnected_since = Some(Instant::now());
                                }
                            }
                            IceConnectionState::Connected | IceConnectionState::Completed => {
                                if ice_disconnected_since.is_some() {
                                    log::info!("[rtc-client] ICE recovered, clearing disconnect timer");
                                }
                                ice_disconnected_since = None;
                            }
                            _ => {}
                        }
                    }

                    if let Event::PeerStats(s) = &event {
                        log::info!(
                            "[rtc-client] peer-stats peer_bytes_tx={} peer_bytes_rx={} bwe_tx={:?} egress_loss={:?} ingress_loss={:?} rtt={:?}",
                            s.peer_bytes_tx,
                            s.peer_bytes_rx,
                            s.bwe_tx,
                            s.egress_loss_fraction,
                            s.ingress_loss_fraction,
                            s.rtt,
                        );
                        if let Some(pair) = &s.selected_candidate_pair {
                            let relayed = self
                                .turn
                                .as_ref()
                                .is_some_and(|turn| pair.local.addr == turn.relay_addr);
                            let prev = self.is_relay.swap(relayed, Ordering::Relaxed);
                            if prev != relayed {
                                log::info!(
                                    "[rtc-client] selected pair relay={} local={} remote={}",
                                    relayed,
                                    pair.local.addr,
                                    pair.remote.addr,
                                );
                            }
                        }
                    }

                    if matches!(&event, Event::ChannelBufferedAmountLow(cid) if *cid == self.channel_ids.reliable) {
                        self.flush_pending_transmit();
                    }

                    if event_tx.send(RtcEvent::Str0mEvent(event)).await.is_err() {
                        log::info!("[rtc-client] Event receiver dropped, stopping RTC I/O thread");
                        return;
                    }
                    continue;
                }
                RtcOutcome::MorePending => {
                    pending_spins += 1;
                    if pending_spins >= MAX_PENDING_SPINS_PER_STEP {
                        pending_spins = 0;
                    }

                    continue;
                }
                RtcOutcome::Idle(_) => {
                    pending_spins = 0;
                }
            }

            let timeout = if ice_disconnected_since.is_some() {
                self.timeout_duration().min(Duration::from_secs(1))
            } else {
                self.timeout_duration()
            };

            tokio::select! {
                result = self.wait_for_input(timeout) => {
                    if let Err(e) = result {
                        log::warn!("[rtc-client] RTC wait_for_input error: {e:?}");
                        let _ = event_tx.send(RtcEvent::Error(e)).await;
                        return;
                    }
                }
                res = data_rx.recv(), if self.pending_transmit.is_none() => {
                    let Some(cmd) = res else {
                        log::info!("[rtc-client] RTC I/O thread shutdown: data channel closed");
                        self.rtc_mut().disconnect();
                        break;
                    };
                    if self.should_pause_transmit(cmd.1) || !self.send(&cmd.0, cmd.1) {
                        self.pending_transmit = Some(cmd);
                    }
                }
            }
        }

        self.rtc_mut().disconnect();
        log::info!("[rtc-client] RTC connection no longer alive, stopping I/O thread");
    }

    async fn wait_for_connected(mut self, mut connected: bool) -> Result<Self, WebRtcClientError> {
        let connect_deadline = Instant::now() + CONNECT_TIMEOUT;
        let mut pending_spins = 0usize;
        loop {
            self.drive_turn().await?;
            self.promote_ready_remote_candidates();

            if let Some(mut rx) = self.candidate_rx.take() {
                let mut closed = false;
                loop {
                    match rx.try_recv() {
                        Ok(candidate) => {
                            self.add_or_defer_remote_candidate(candidate, "resolved");
                        }
                        Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                            log::info!("[rtc-client] Candidate resolution channel closed");
                            closed = true;
                            break;
                        }
                    }
                }
                if !closed {
                    self.candidate_rx = Some(rx);
                }
            }
            self.promote_ready_remote_candidates();

            match self.poll_event().await? {
                RtcOutcome::Event(event) => {
                    pending_spins = 0;
                    match event {
                        Event::Connected => {
                            log::info!("[rtc-client] DTLS Connected");
                            connected = true;
                        }
                        Event::IceConnectionStateChange(state) => {
                            log::info!("[rtc-client] ICE state: {:?}", state);
                            if matches!(state, IceConnectionState::Disconnected) {
                                return Err(WebRtcClientError::Connection("Peer disconnected during setup".into()));
                            }
                        }
                        Event::ChannelOpen(cid, _) => {
                            log::info!("[rtc-client] Channel {:?} opened during connect phase", cid);
                        }
                        Event::ChannelData(_) => {
                            log::info!("[rtc-client] Buffering early ChannelData during connect phase");
                            self.early_events.push(event);
                        }
                        Event::PeerStats(s) => {
                            if let Some(pair) = &s.selected_candidate_pair {
                                log::info!(
                                    "[rtc-client] Selected candidate pair -- local: {} ({:?}), remote: {} ({:?})",
                                    pair.local.addr,
                                    pair.protocol,
                                    pair.remote.addr,
                                    pair.protocol,
                                );
                            }
                        }
                        _ => {
                            log::debug!("[rtc-client] Other event during connect: {:?}", event);
                        }
                    }
                    continue;
                }
                RtcOutcome::MorePending => {
                    pending_spins += 1;
                    if pending_spins >= MAX_PENDING_SPINS_PER_STEP {
                        pending_spins = 0;
                    } else {
                        continue;
                    }
                }
                RtcOutcome::Idle(_) => {
                    pending_spins = 0;
                }
            }

            if connected {
                let ready = [
                    self.channel_ids.reliable,
                    self.channel_ids.unordered_msg,
                    self.channel_ids.ordered_msg,
                ]
                .iter()
                .all(|&cid| self.rtc_mut().channel(cid).is_some());

                if ready {
                    self.configure_channel_watermarks();
                    log::info!("[rtc-client] Connected, negotiated channels ready");
                    return Ok(self);
                }
            }

            if !self.rtc_mut().is_alive() {
                return Err(WebRtcClientError::Connection("RTC connection closed".into()));
            }

            if Instant::now() >= connect_deadline {
                return Err(WebRtcClientError::Connection("Connection timeout during handshake".into()));
            }

            let timeout = self.timeout_duration().min(connect_deadline - Instant::now());
            self.wait_for_input(timeout).await?;

            self.promote_ready_remote_candidates();
        }
    }

    async fn poll_event(&mut self) -> Result<RtcOutcome, WebRtcClientError> {
        let output = self.rtc_mut().poll_output().map_err(|e| WebRtcClientError::Rtc(e.to_string()))?;
        match output {
            Output::Timeout(t) => {
                self.cached_timeout = t;
                Ok(RtcOutcome::Idle(t))
            }
            Output::Transmit(t) => {
                let dest = from_v6_mapped(t.destination);

                if let Some(ref mut turn) = self.turn {
                    let source = from_v6_mapped(t.source);
                    if source == turn.relay_addr {
                        let now = stun_now(turn.stun_base);
                        let payload: &[u8] = &t.contents;
                        match turn.client.send_to(StunTransportType::Udp, dest, payload, now) {
                            Ok(Some(transmit_build)) => {
                                let built = transmit_build.build();
                                let send_addr = to_v6_mapped(built.to);
                                if let Err(e) = self.socket.send_to(&built.data, send_addr).await {
                                    log::warn!("[rtc-client] TURN relay send error to {}: {}", send_addr, e);
                                }
                            }
                            Ok(None) => {}
                            Err(SendError::NoPermission) => {
                                if turn.try_mark_channel_attempt(dest) {
                                    match turn.client.bind_channel(StunTransportType::Udp, dest, now) {
                                        Ok(()) | Err(BindChannelError::AlreadyExists) => {
                                            log::debug!(
                                                "[rtc-client] Lazy-binding TURN channel for peer-reflexive dest {}",
                                                dest
                                            );
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "[rtc-client] Lazy TURN channel bind for {} failed: {}",
                                                dest,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("[rtc-client] TURN send_to failed for {}: {}", dest, e);
                            }
                        }
                        self.flush_turn_transmits().await?;
                        return Ok(RtcOutcome::MorePending);
                    }
                }

                let socket_dest = to_v6_mapped(dest);
                if let Err(e) = self.socket.send_to(&t.contents, socket_dest).await {
                    log::warn!("[rtc-client] Failed to send to {}: {}", socket_dest, e);
                    return Err(WebRtcClientError::Connection(format!("Failed to send packet: {e}")));
                }

                Ok(RtcOutcome::MorePending)
            }
            Output::Event(e) => Ok(RtcOutcome::Event(e)),
        }
    }

    fn timeout_duration(&self) -> Duration {
        let now = Instant::now();
        let str0m_timeout = self.cached_timeout.saturating_duration_since(now);
        self.turn
            .as_ref()
            .map_or(str0m_timeout, |turn| str0m_timeout.min(turn.cached_timeout.saturating_duration_since(now)))
    }

    async fn wait_for_input(&mut self, timeout: Duration) -> Result<(), WebRtcClientError> {
        if timeout.is_zero() {
            self.drive_turn().await?;
            self.rtc_mut()
                .handle_input(Input::Timeout(Instant::now()))
                .map_err(|e| WebRtcClientError::Rtc(e.to_string()))?;
            return Ok(());
        }

        match tokio::time::timeout(timeout, self.socket.recv_from(&mut self.buf[..])).await {
            Ok(Ok((n, source))) => {
                let source = from_v6_mapped(source);

                if let Some(ref mut turn) = self.turn {
                    if source == turn.server_addr {
                        let now = stun_now(turn.stun_base);
                        let local_addr = turn.client.local_addr();
                        let transmit = Transmit::new(&self.buf[..n], StunTransportType::Udp, source, local_addr);
                        match turn.client.recv(transmit, now) {
                            TurnRecvRet::PeerData(peer_data) => {
                                let peer_addr = peer_data.peer;
                                let relay_addr = turn.relay_addr;
                                let data = peer_data.data();
                                match Receive::new(Protocol::Udp, peer_addr, relay_addr, data) {
                                    Ok(receive) => {
                                        if let Err(e) = self.rtc.borrow_mut().handle_input(Input::Receive(Instant::now(), receive)) {
                                            log::warn!("[rtc-client] str0m handle relayed input: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!("[rtc-client] Failed to parse Receive for relayed data: {}", e);
                                    }
                                }
                                self.flush_turn_transmits().await?;
                                return Ok(());
                            }
                            TurnRecvRet::Handled => {
                                log::debug!("[rtc-client] TURN control message handled from {}", source);
                                self.flush_turn_transmits().await?;
                                return Ok(());
                            }
                            TurnRecvRet::Ignored(reason) => {
                                log::debug!(
                                    "[rtc-client] Packet from TURN server {} not a TURN message, passing to str0m. Reason: {:?}",
                                    source, reason
                                );
                            }
                            TurnRecvRet::PeerIcmp { peer, .. } => {
                                log::debug!("[rtc-client] TURN ICMP from peer {}", peer);
                                return Ok(());
                            }
                        }
                    }
                }

                self.handle_str0m_receive(source, n)?;
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                self.drive_turn().await?;
                self.rtc_mut()
                    .handle_input(Input::Timeout(Instant::now()))
                    .map_err(|e| WebRtcClientError::Rtc(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Handle a received packet destined for str0m.
    fn handle_str0m_receive(&mut self, source: SocketAddr, buf_len: usize) -> Result<(), WebRtcClientError> {
        let local = if source.is_ipv4() {
            self.local_v4_addr.unwrap_or(self.local_addr)
        } else {
            self.local_v6_addr.unwrap_or(self.local_addr)
        };
        match Receive::new(Protocol::Udp, source, local, &self.buf[..buf_len]) {
            Ok(receive) => {
                if let Err(e) = self.rtc.borrow_mut().handle_input(Input::Receive(Instant::now(), receive)) {
                    log::warn!("[rtc-client] Input handle packet drop: {}", e);
                }
            }
            Err(e) => {
                log::warn!("[rtc-client] Failed to parse Receive: {}", e);
            }
        }
        Ok(())
    }

    fn send(&mut self, data: &[u8], channel_id: ChannelId) -> bool {
        let mut rtc = self.rtc_mut();
        let Some(mut ch) = rtc.channel(channel_id) else {
            return false;
        };
        match ch.write(true, data) {
            Ok(result) => result,
            Err(e) => {
                log::error!("[rtc-client] Failed to write to channel {:?}: {:?}", channel_id, e);
                false
            }
        }
    }

    fn configure_channel_watermarks(&mut self) {
        let channel_id = self.channel_ids.reliable;
        if let Some(mut channel) = self.rtc_mut().channel(channel_id) {
            channel.set_buffered_amount_low_threshold(RELIABLE_BUFFERED_AMOUNT_LOW_THRESHOLD);
        }
    }

    fn channel_buffered_amount(&mut self, channel_id: ChannelId) -> Option<usize> {
        self.rtc_mut().channel(channel_id).map(|mut ch| ch.buffered_amount())
    }

    fn should_pause_transmit(&mut self, channel_id: ChannelId) -> bool {
        channel_id == self.channel_ids.reliable
            && self
                .channel_buffered_amount(channel_id)
                .is_some_and(|buffered| buffered >= RELIABLE_BUFFERED_AMOUNT_REFILL_TARGET)
    }

    fn flush_pending_transmit(&mut self) {
        if let Some((data, channel_id)) = self.pending_transmit.take() {
            if self.should_pause_transmit(channel_id) || !self.send(&data, channel_id) {
                self.pending_transmit = Some((data, channel_id));
            }
        }
        self.refresh_data_buffered_amount();
    }

    fn refresh_data_buffered_amount(&mut self) {
        let channel_id = self.channel_ids.reliable;
        let amount = self.channel_buffered_amount(channel_id).unwrap_or(0);
        self.data_buffered_amount.store(amount, Ordering::Relaxed);
    }

    fn add_or_defer_remote_candidate(&mut self, candidate: str0m::Candidate, source: &str) {
        log::debug!("[rtc-client] Received {source} remote candidate: {}", candidate);

        let wait_for_channel = self.turn.as_mut().is_some_and(|turn| {
            request_turn_channel(turn, &candidate, source);
            should_wait_for_turn_channel(turn, &candidate)
        });

        if wait_for_channel {
            log::debug!(
                "[rtc-client] Deferring {source} remote candidate until TURN channel is bound for {}",
                remote_candidate_permission_addr(&candidate)
            );
            self.pending_remote_candidates.push_back(candidate);
            return;
        }

        self.rtc_mut().add_remote_candidate(candidate);
    }

    fn promote_ready_remote_candidates(&mut self) {
        if self.pending_remote_candidates.is_empty() {
            return;
        }

        let mut pending = std::mem::take(&mut self.pending_remote_candidates);
        while let Some(candidate) = pending.pop_front() {
            let wait_for_channel = self
                .turn
                .as_ref()
                .is_some_and(|turn| should_wait_for_turn_channel(turn, &candidate));
            if wait_for_channel {
                self.pending_remote_candidates.push_back(candidate);
                continue;
            }

            log::debug!(
                "[rtc-client] Activating deferred remote candidate after TURN channel bind: {}",
                candidate
            );
            self.rtc_mut().add_remote_candidate(candidate);
        }
    }

    /// Flush pending outgoing TURN protocol messages to the socket.
    ///
    /// Also drives the TURN state machine via `poll()` so that pending
    /// transactions (CreatePermission, Refresh, etc.) are advanced before
    /// we drain the transmit queue.
    async fn flush_turn_transmits(&mut self) -> Result<(), WebRtcClientError> {
        let Some(ref mut turn) = self.turn else {
            return Ok(());
        };
        let now = stun_now(turn.stun_base);
        match turn.client.poll(now) {
            TurnPollRet::WaitUntil(deadline) => {
                let wait = deadline.checked_duration_since(now).unwrap_or(Duration::ZERO);
                turn.cached_timeout = Instant::now() + wait;
            }
            TurnPollRet::Closed => {
                log::warn!("[rtc-client] TURN client closed during flush");
                self.turn = None;
                return Ok(());
            }
            TurnPollRet::TcpClose { .. } | TurnPollRet::AllocateTcpSocket { .. } => {}
        }
        let now = stun_now(turn.stun_base);
        while let Some(transmit) = turn.client.poll_transmit(now) {
            let send_addr = to_v6_mapped(transmit.to);
            if let Err(e) = self.socket.send_to(transmit.data.as_ref(), send_addr).await {
                log::warn!(
                    "[rtc-client] TURN transmit send error to {}: kind={:?}, err={}",
                    send_addr,
                    e.kind(),
                    e
                );
            }
        }
        while let Some(event) = turn.client.poll_event() {
            log::info!("[rtc-client] TURN event: {:?}", event);
            match event {
                TurnEvent::ChannelCreated(_, peer_addr) => {
                    turn.mark_channel_bound(peer_addr);
                }
                TurnEvent::ChannelCreateFailed(_, peer_addr) => {
                    turn.mark_channel_unbound(peer_addr);
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn drive_turn(&mut self) -> Result<(), WebRtcClientError> {
        if self.turn.is_none() {
            return Ok(());
        }
        self.flush_turn_transmits().await
    }

    fn rtc_mut(&mut self) -> std::cell::RefMut<'_, Rtc> {
        self.rtc.borrow_mut()
    }
}

fn apply_udp_buffer_size(socket: &Socket, requested: usize) {
    let _ = socket.set_send_buffer_size(requested);
    let _ = socket.set_recv_buffer_size(requested);
    if let Ok(actual) = socket.send_buffer_size() {
        if actual < requested / 2 {
            log::warn!(
                "[rtc-client] UDP send buf clamped: requested={requested} actual={actual} (raise kern.ipc.maxsockbuf or net.core.wmem_max)"
            );
        }
    }
    if let Ok(actual) = socket.recv_buffer_size() {
        if actual < requested / 2 {
            log::warn!(
                "[rtc-client] UDP recv buf clamped: requested={requested} actual={actual} (raise kern.ipc.maxsockbuf or net.core.rmem_max)"
            );
        }
    }
}

fn to_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::new(v4.ip().to_ipv6_mapped().into(), v4.port()),
        v6 => v6,
    }
}

fn from_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V6(v6) => match v6.ip().to_ipv4_mapped() {
            Some(v4) => SocketAddr::new(v4.into(), v6.port()),
            None => addr,
        },
        _ => addr,
    }
}

fn remote_candidate_permission_addr(candidate: &str0m::Candidate) -> SocketAddr {
    from_v6_mapped(candidate.addr())
}

fn request_turn_channel(turn: &mut TurnRelayInfo, candidate: &str0m::Candidate, source: &str) {
    let peer_addr = remote_candidate_permission_addr(candidate);
    if !turn.try_mark_channel_attempt(peer_addr) {
        return;
    }
    let now = stun_now(turn.stun_base);
    match turn.client.bind_channel(StunTransportType::Udp, peer_addr, now) {
        Ok(()) | Err(BindChannelError::AlreadyExists) => {}
        Err(e) => {
            log::debug!("[rtc-client] TURN channel bind for {source} candidate {peer_addr}: {e}");
        }
    }
}

fn should_wait_for_turn_channel(turn: &TurnRelayInfo, candidate: &str0m::Candidate) -> bool {
    is_relay_only()
        && candidate.kind() == str0m::CandidateKind::Relayed
        && !turn.have_bound_channel(remote_candidate_permission_addr(candidate))
}

#[cfg(test)]
mod tests {
    use super::{from_v6_mapped, remote_candidate_permission_addr};
    use str0m::{Candidate, CandidateKind};

    #[test]
    fn relay_channel_peer_addr_uses_candidate_address_not_raddr() {
        let candidate = Candidate::from_parts(
            "relay".to_string(),
            1,
            str0m::net::Protocol::Udp,
            1,
            "127.0.0.1:54336".parse().unwrap(),
            CandidateKind::Relayed,
            Some("[::1]:50560".parse().unwrap()),
            None,
            None,
        );

        assert_eq!(
            remote_candidate_permission_addr(&candidate),
            "127.0.0.1:54336".parse::<std::net::SocketAddr>().unwrap()
        );
    }

    #[test]
    fn relay_channel_peer_addr_normalizes_ipv6_mapped_ipv4_addresses() {
        let candidate = Candidate::from_parts(
            "relay".to_string(),
            1,
            str0m::net::Protocol::Udp,
            1,
            "[::ffff:127.0.0.1]:54336".parse().unwrap(),
            CandidateKind::Relayed,
            None,
            None,
            None,
        );

        assert_eq!(
            remote_candidate_permission_addr(&candidate),
            "127.0.0.1:54336".parse::<std::net::SocketAddr>().unwrap()
        );
    }

    #[test]
    fn from_v6_mapped_passes_through_plain_ipv4() {
        let addr: std::net::SocketAddr = "192.0.2.1:8080".parse().unwrap();
        assert_eq!(from_v6_mapped(addr), addr);
    }

    #[test]
    fn from_v6_mapped_passes_through_non_mapped_v6() {
        let addr: std::net::SocketAddr = "[2001:db8::1]:443".parse().unwrap();
        assert_eq!(from_v6_mapped(addr), addr);
    }

    #[test]
    fn from_v6_mapped_unwraps_v4_mapped_v6() {
        let v6: std::net::SocketAddr = "[::ffff:192.0.2.1]:9000".parse().unwrap();
        let expected: std::net::SocketAddr = "192.0.2.1:9000".parse().unwrap();
        assert_eq!(from_v6_mapped(v6), expected);
    }
}
