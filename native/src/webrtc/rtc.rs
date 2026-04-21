use socket2::{Domain, Socket, Type};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{Protocol, Receive};
use str0m::{Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use futures_util::StreamExt;
use schema::devlog::rpc_signalling::server::OfferMessage;

use crate::webrtc::client::{WebRtcClientError, MAX_BUFFER_SIZE};
use crate::webrtc::ice::{strip_candidates_from_sdp, IceAgent};
use crate::webrtc::signalling::SignallingSender;

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

/// Maximum number of consecutive `MorePending` cycles
const MAX_PENDING_SPINS_PER_STEP: usize = 8;

/// Reliable channel low watermark that triggers a refill event from str0m.
const RELIABLE_BUFFERED_AMOUNT_LOW_THRESHOLD: usize = 256 * 1024;

/// Refill reliable buffered data only up to this target to avoid large burst-then-drain cycles.
const RELIABLE_BUFFERED_AMOUNT_REFILL_TARGET: usize = 512 * 1024;

/// Events emitted from the RTC thread to the outside world.
pub enum RtcEvent {
    Str0mEvent(Event),
    Error(WebRtcClientError),
}

/// Outcome of a single poll operation
#[derive(Debug)]
pub enum RtcOutcome {
    Event(Event),
    Idle(Instant),
    MorePending,
}

/// Commands sent from outside into the RTC thread.
// RtcCommand enum removed, replaced with (Vec<u8>, ChannelId) tuple for simplicity

/// Async-friendly handle returned from connect(). Communicates with the
/// dedicated OS thread that drives the RTC networking loop.
pub struct RtcHandle {
    event_rx: tokio::sync::mpsc::Receiver<RtcEvent>,
    data_tx: Option<tokio::sync::mpsc::Sender<(Vec<u8>, ChannelId)>>,
    channel_ids: ChannelIds,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl RtcHandle {
    /// Connect to a peer via direct P2P, spawning the I/O thread on success.
    pub async fn connect(
        signalling_id: &str,
        offer_message: OfferMessage,
        me: schema::devlog::bitbridge::PeerMessage,
        signalling: SignallingSender,
        request_id: &str,
    ) -> Result<Self, WebRtcClientError> {
        RtcClient::connect(signalling_id, offer_message, me, signalling, request_id).await
    }

    /// Await the next event from the RTC thread.
    pub async fn poll_event(&mut self) -> Option<RtcEvent> {
        self.event_rx.recv().await
    }

    /// Try to receive an event without blocking.
    pub fn try_poll_event(&mut self) -> Option<RtcEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Send data on a channel. Returns true if the command was queued.
    pub fn send(&self, data: &[u8], channel_id: ChannelId) -> bool {
        if let Some(ref tx) = self.data_tx {
            tx.try_send((data.to_vec(), channel_id)).is_ok()
        } else {
            false
        }
    }

    pub fn channel_ids(&self) -> &ChannelIds {
        &self.channel_ids
    }

    /// The handle is alive if the thread hasn't finished and the event channel is open.
    pub fn is_alive(&self) -> bool {
        !self.event_rx.is_closed() && self.thread_handle.as_ref().is_some_and(|h| !h.is_finished())
    }

    pub fn shutdown(&mut self) {
        drop(self.data_tx.take());
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
    rtc: Arc<StdMutex<Rtc>>,
    socket: tokio::net::UdpSocket,
    local_addr: SocketAddr,
    local_v4_addr: Option<SocketAddr>,
    local_v6_addr: Option<SocketAddr>,
    buf: Vec<u8>,
    cached_timeout: Instant,
    channel_ids: ChannelIds,
    pending_transmits: VecDeque<(Vec<u8>, str0m::channel::ChannelId)>,
    /// Events received during the connect/handshake phase that need to be
    /// replayed once the run loop starts (e.g. early ChannelData).
    early_events: Vec<Event>,
}

impl RtcClient {
    pub async fn connect(
        signalling_id: &str,
        offer_message: OfferMessage,
        me: schema::devlog::bitbridge::PeerMessage,
        signalling: SignallingSender,
        request_id: &str,
    ) -> Result<RtcHandle, WebRtcClientError> {
        // Fetch relay config (TURN servers) from signalling server.
        // Falls back to P2P-only if unavailable.
        let config = match signalling.fetch_relay_config(signalling_id).await {
            Ok(cfg) => {
                log::info!("[rtc-client] Using relay config with {} URLs", cfg.urls.len());
                cfg
            }
            Err(e) => {
                log::warn!("[rtc-client] Failed to fetch relay config, using P2P only: {}", e);
                schema::devlog::rpc_signalling::server::IceConfig::default()
            }
        };

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.set_nonblocking(true)?;
        socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into())?;
        let _ = socket.set_send_buffer_size(MAX_BUFFER_SIZE);
        let _ = socket.set_recv_buffer_size(MAX_BUFFER_SIZE);
        let socket: std::net::UdpSocket = socket.into();
        let socket = tokio::net::UdpSocket::from_std(socket)?;

        let local_addr = socket.local_addr()?;

        let (candidates, ..) = IceAgent::gather_candidates(&socket, &config)
            .await
            .map_err(|e| WebRtcClientError::Signalling(e.to_string()))?;

        let config = RtcConfig::default().set_sctp_max_message_size(256 * 1024).set_sctp_buffer_size(5 * 1024 * 1024);

        let mut rtc = config.build(Instant::now());
        let mut local_v4_addr = None;
        let mut local_v6_addr = None;
        log::info!("[rtc-client] Adding {} gathered candidates to RTC engine", candidates.len());
        for candidate in candidates {
            log::debug!("[rtc-client] Adding candidate: {}", candidate);
            if candidate.addr().is_ipv4() && local_v4_addr.is_none() {
                local_v4_addr = Some(candidate.addr());
            } else if candidate.addr().is_ipv6() && local_v6_addr.is_none() {
                local_v6_addr = Some(candidate.addr());
            }
            rtc.add_local_candidate(candidate);
        }

        // Strip candidates from SDP to allow early answer without waiting for DNS resolution
        let stripped_offer_sdp = strip_candidates_from_sdp(&offer_message.sdp);
        log::info!("Stripped offer SDP (candidates removed for early answer)");

        let offer = str0m::change::SdpOffer::from_sdp_string(&stripped_offer_sdp).map_err(|e| WebRtcClientError::Sdp(e.to_string()))?;

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

        // Accept offer - candidates stripped so none to add, early answer enabled
        let answer = rtc.sdp_api().accept_offer(offer).map_err(|e| WebRtcClientError::Rtc(e.to_string()))?;

        // Wrap rtc in Arc<Mutex> so spawned task can add candidates
        let rtc = Arc::new(StdMutex::new(rtc));
        let rtc_for_spawn = rtc.clone();

        // Send answer immediately while resolving candidates in background
        let answer_sdp = answer.to_sdp_string();
        let me_clone = me.clone();
        let request_id_owned = request_id.to_string();
        let original_sdp = offer_message.sdp.clone();

        // Spawn candidate resolution to run concurrently with signaling
        tokio::spawn(async move {
            let stream = IceAgent::resolve_remote_candidates_stream(&original_sdp);
            futures_util::pin_mut!(stream);
            while let Some(candidate) = stream.next().await {
                log::debug!("[rtc-client] Resolved remote candidate: {}", candidate);
                rtc_for_spawn.lock().unwrap().add_remote_candidate(candidate);
            }
            log::info!("[rtc-client] Remote candidate resolution complete");
        });

        // Send answer without waiting for candidate resolution
        if let Err(e) = signalling.send_answer(answer_sdp, me_clone, &request_id_owned).await {
            log::warn!("[rtc-client] Failed to send answer: {}", e);
        }
        log::info!("[rtc-client] Answer sent, candidate resolution continuing in background");

        // rtc is Arc<StdMutex<Rtc>> with rtc_for_spawn held by spawned task
        // Store Arc in struct - spawned task will add candidates as they're resolved
        let client = Self {
            rtc,
            socket,
            local_addr,
            local_v4_addr,
            local_v6_addr,
            buf: vec![0u8; 2000],
            cached_timeout: Instant::now(),
            channel_ids,
            pending_transmits: VecDeque::new(),
            early_events: Vec::new(),
        };

        let connected = false;
        let client = client.wait_for_connected(connected).await?;
        log::info!("Connected to p2p");
        Ok(client.spawn_thread())
    }

    /// Spawn the dedicated OS thread and return an RtcHandle.
    fn spawn_thread(self) -> RtcHandle {
        let channel_ids = self.channel_ids;
        let (event_tx, event_rx) = tokio::sync::mpsc::channel::<RtcEvent>(5);
        let (data_tx, data_rx) = tokio::sync::mpsc::channel::<(Vec<u8>, ChannelId)>(16);

        // Use an 8 MB stack to avoid overflow during high-throughput transmit bursts.
        let thread_handle = std::thread::Builder::new()
            .name("rtc-io".to_string())
            .stack_size(8 * 1024 * 1024)
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
            event_rx,
            data_tx: Some(data_tx),
            channel_ids,
            thread_handle: Some(thread_handle),
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

            self.drain_pending_transmits();
            self.fill_transmit_queue(&mut data_rx);
            self.drain_pending_transmits();

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

                    if matches!(&event, Event::ChannelBufferedAmountLow(cid) if *cid == self.channel_ids.reliable) {
                        self.drain_pending_transmits();
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
                res = data_rx.recv() => {
                    match res {
                        Some(cmd) => {
                            self.enqueue_command(cmd);
                            self.drain_pending_transmits();
                        }
                        None => {
                            log::info!("[rtc-client] RTC I/O thread shutdown: data channel closed");
                            self.rtc_mut().disconnect();
                            break;
                        }
                    }
                }
            }
        }

        self.rtc_mut().disconnect();
        log::info!("[rtc-client] RTC connection no longer alive, stopping I/O thread");
    }

    /// Handle a command.
    fn enqueue_command(&mut self, cmd: (Vec<u8>, ChannelId)) {
        self.pending_transmits.push_back(cmd);
    }

    async fn wait_for_connected(mut self, mut connected: bool) -> Result<Self, WebRtcClientError> {
        let connect_deadline = Instant::now() + CONNECT_TIMEOUT;
        loop {
            if let RtcOutcome::Event(event) = self.poll_event().await? {
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
                    _ => {
                        log::debug!("[rtc-client] Other event during connect: {:?}", event);
                    }
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
                let peer_addr = from_v6_mapped(t.destination);
                let socket_dest = to_v6_mapped(peer_addr);

                if let Err(e) = self.socket.send_to(&t.contents, socket_dest).await {
                    log::warn!("[rtc-client] Failed to send to {}: {}", socket_dest, e);
                    return Err(WebRtcClientError::Connection(format!("Failed to send packet: {e}")));
                }

                Ok(RtcOutcome::MorePending)
            }
            Output::Event(e) => {
                log::info!("[rtc-client] str0m Event: {:?}", e);
                Ok(RtcOutcome::Event(e))
            }
        }
    }

    fn timeout_duration(&self) -> Duration {
        self.cached_timeout.saturating_duration_since(Instant::now())
    }

    async fn wait_for_input(&mut self, timeout: Duration) -> Result<(), WebRtcClientError> {
        if timeout.is_zero() {
            self.rtc_mut()
                .handle_input(Input::Timeout(Instant::now()))
                .map_err(|e| WebRtcClientError::Rtc(e.to_string()))?;
            return Ok(());
        }

        match tokio::time::timeout(timeout, self.socket.recv_from(&mut self.buf[..])).await {
            Ok(Ok((n, source))) => {
                let source = from_v6_mapped(source);
                let packet_data = self.buf[..n].to_vec();
                self.handle_str0m_receive(source, &packet_data)?;
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                self.rtc_mut()
                    .handle_input(Input::Timeout(Instant::now()))
                    .map_err(|e| WebRtcClientError::Rtc(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Handle a received packet destined for str0m.
    fn handle_str0m_receive(&mut self, source: SocketAddr, data: &[u8]) -> Result<(), WebRtcClientError> {
        let local = if source.is_ipv4() {
            self.local_v4_addr.unwrap_or(self.local_addr)
        } else {
            self.local_v6_addr.unwrap_or(self.local_addr)
        };
        match Receive::new(Protocol::Udp, source, local, data) {
            Ok(receive) => {
                if let Err(e) = self.rtc_mut().handle_input(Input::Receive(Instant::now(), receive)) {
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
        if let Some(mut ch) = self.rtc_mut().channel(channel_id) {
            match ch.write(true, data) {
                Ok(true) => true,
                Ok(false) => false,
                Err(e) => {
                    log::error!("[rtc-client] Failed to write to channel {:?}: {:?}", channel_id, e);
                    false
                }
            }
        } else {
            false
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

    fn drain_pending_transmits(&mut self) {
        loop {
            let Some((data, channel_id)) = self.pending_transmits.pop_front() else {
                break;
            };

            if self.should_pause_transmit(channel_id) {
                self.pending_transmits.push_front((data, channel_id));
                break;
            }

            if !self.send(&data, channel_id) {
                self.pending_transmits.push_front((data, channel_id));
                break;
            }
        }
    }

    fn fill_transmit_queue(&mut self, data_rx: &mut tokio::sync::mpsc::Receiver<(Vec<u8>, ChannelId)>) {
        while let Ok(cmd) = data_rx.try_recv() {
            self.enqueue_command(cmd);
        }
    }

    fn rtc_mut(&mut self) -> std::sync::MutexGuard<'_, Rtc> {
        self.rtc.lock().unwrap()
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
        SocketAddr::V6(v6) => {
            let octets = v6.ip().octets();
            if octets[0..12]
                == [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff,
                ]
            {
                let v4 = std::net::Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]);
                SocketAddr::new(v4.into(), v6.port())
            } else {
                addr
            }
        }
        _ => addr,
    }
}
