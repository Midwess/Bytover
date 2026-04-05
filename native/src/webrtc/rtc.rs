use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};
use socket2::{Domain, Socket, Type};
use str0m::channel::{ChannelConfig, ChannelId};
use str0m::net::{Protocol, Receive};
use str0m::{Event, IceConnectionState, Input, Output, Rtc, RtcConfig};

use schema::devlog::rpc_signalling::server::OfferMessage;

use crate::webrtc::client::{MAX_BUFFER_SIZE, WebRtcClientError};
use crate::webrtc::ice::IceAgent;
use crate::webrtc::signalling::SignallingSender;

pub const RELIABLE_STREAM_ID: u16 = 1;
pub const UNORDERED_MSG_STREAM_ID: u16 = 2;
pub const ORDERED_MSG_STREAM_ID: u16 = 3;

#[derive(Debug, Clone, Copy)]
pub struct ChannelIds {
    pub reliable: ChannelId,
    pub unordered_msg: ChannelId,
    pub ordered_msg: ChannelId
}

/// Events emitted from the RTC thread to the outside world.
pub enum RtcEvent {
    Str0mEvent(Event),
    Error(WebRtcClientError),
    Closed,
}

/// Commands sent from outside into the RTC thread.
pub enum RtcCommand {
    Send { data: Vec<u8>, channel_id: ChannelId },
    SetBufferedAmountLowThreshold { channel_id: ChannelId, threshold: usize },
    Shutdown,
}

/// Async-friendly handle returned from connect(). Communicates with the
/// dedicated OS thread that drives the RTC networking loop.
pub struct RtcHandle {
    event_rx: tokio::sync::mpsc::Receiver<RtcEvent>,
    command_tx: tokio::sync::mpsc::Sender<RtcCommand>,
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
        request_id: &str
    ) -> Result<Self, WebRtcClientError> {
        RtcClient::connect(signalling_id, offer_message, me, signalling, request_id).await
    }

    /// Connect to a peer via relay, spawning the I/O thread on success.
    pub async fn connect_relay(
        signalling_id: &str,
        session_id: &str,
        signalling: SignallingSender,
    ) -> Result<Self, WebRtcClientError> {
        RtcClient::connect_relay(signalling_id, session_id, signalling).await
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
        self.command_tx.try_send(RtcCommand::Send {
            data: data.to_vec(),
            channel_id,
        }).is_ok()
    }

    pub fn set_buffered_amount_low_threshold(&self, channel_id: ChannelId, threshold: usize) {
        let _ = self.command_tx.try_send(RtcCommand::SetBufferedAmountLowThreshold {
            channel_id,
            threshold,
        });
    }

    pub fn channel_ids(&self) -> &ChannelIds {
        &self.channel_ids
    }

    /// The handle is alive if the thread hasn't finished and the event channel is open.
    pub fn is_alive(&self) -> bool {
        !self.event_rx.is_closed()
            && self.thread_handle.as_ref().map_or(false, |h| !h.is_finished())
    }

    pub fn shutdown(&self) {
        let _ = self.command_tx.try_send(RtcCommand::Shutdown);
    }
}

impl Drop for RtcHandle {
    fn drop(&mut self) {
        let _ = self.command_tx.try_send(RtcCommand::Shutdown);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

struct RtcClient {
    rtc: Rtc,
    socket: tokio::net::UdpSocket,
    local_addr: SocketAddr,
    local_v4_addr: Option<SocketAddr>,
    local_v6_addr: Option<SocketAddr>,
    buf: Vec<u8>,
    cached_timeout: Instant,
    channel_ids: ChannelIds,
    pending_transmits: std::collections::VecDeque<(Vec<u8>, str0m::channel::ChannelId)>,
}

impl RtcClient {
    pub async fn connect(
        signalling_id: &str,
        offer_message: OfferMessage,
        me: schema::devlog::bitbridge::PeerMessage,
        signalling: SignallingSender,
        request_id: &str
    ) -> Result<RtcHandle, WebRtcClientError> {
        let config = match signalling.fetch_relay_config(signalling_id).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!(
                    "[rtc-client] Failed to fetch relay config ({}), proceeding without TURN relay",
                    e
                );
                schema::devlog::rpc_signalling::server::IceConfig::default()
            }
        };

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.set_nonblocking(true)?;
        socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into())?;
        let _ = socket.set_send_buffer_size(MAX_BUFFER_SIZE * 2);
        let socket: std::net::UdpSocket = socket.into();
        let socket = tokio::net::UdpSocket::from_std(socket)?;

        let local_addr = socket.local_addr()?;

        let candidates = IceAgent::gather_candidates(&socket, &config)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        let config = RtcConfig::default()
            .set_sctp_max_message_size(256 * 1024)
            .set_sctp_buffer_size(5 * 1024 * 1024);

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

        let offer_sdp = IceAgent::resolve_remote_candidates(&offer_message.sdp);
        log::info!("Received offer sdp: {offer_sdp}");

        let _remote_ips = extract_remote_candidate_ips(&offer_sdp);

        let offer = str0m::change::SdpOffer::from_sdp_string(&offer_sdp).map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

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
            ordered_msg: ordered_msg_id
        };

        let answer = rtc.sdp_api().accept_offer(offer).map_err(WebRtcClientError::Rtc)?;

        signalling
            .send_answer(answer.to_sdp_string(), me, request_id)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        let client = Self {
            rtc,
            socket,
            local_addr,
            local_v4_addr,
            local_v6_addr,
            buf: vec![0u8; 2000],
            cached_timeout: Instant::now(),
            channel_ids,
            pending_transmits: std::collections::VecDeque::new(),
        };

        let connected = false;
        let client = client.wait_for_connected(connected).await?;
        Ok(client.spawn_thread())
    }

    pub async fn connect_relay(
        signalling_id: &str,
        session_id: &str,
        signalling: SignallingSender,
    ) -> Result<RtcHandle, WebRtcClientError> {
        log::info!("Connecting to relay");
        // Fetch relay config which will act as the ICE server config.
        let config = match signalling.fetch_relay_config(signalling_id).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("[rtc-client relay] Failed to fetch relay config: {}", e);
                schema::devlog::rpc_signalling::server::IceConfig::default()
            }
        };

        let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(socket2::Protocol::UDP))?;
        socket.set_only_v6(false)?;
        socket.set_nonblocking(true)?;
        socket.bind(&"[::]:0".parse::<SocketAddr>().unwrap().into())?;
        let _ = socket.set_send_buffer_size(MAX_BUFFER_SIZE * 2);
        let socket: std::net::UdpSocket = socket.into();
        let socket = tokio::net::UdpSocket::from_std(socket)?;
        let local_addr = socket.local_addr()?;

        let candidates = IceAgent::gather_candidates(&socket, &config)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        let rtc_config = RtcConfig::default()
            .set_sctp_max_message_size(256 * 1024)
            .set_sctp_buffer_size(5 * 1024 * 1024);

        let mut rtc = rtc_config.build(Instant::now());
        let mut local_v4_addr = None;
        let mut local_v6_addr = None;
        for candidate in candidates {
            if candidate.addr().is_ipv4() && local_v4_addr.is_none() {
                local_v4_addr = Some(candidate.addr());
            } else if candidate.addr().is_ipv6() && local_v6_addr.is_none() {
                local_v6_addr = Some(candidate.addr());
            }
            rtc.add_local_candidate(candidate);
        }

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

        let mut sdp_api = rtc.sdp_api();
        let reliable_id = sdp_api.add_channel_with_config(ChannelConfig {
            label: "reliable".to_string(),
            ordered: false,
            negotiated: Some(RELIABLE_STREAM_ID),
            ..Default::default()
        });

        let channel_ids = ChannelIds {
            reliable: reliable_id,
            unordered_msg: unordered_msg_id,
            ordered_msg: ordered_msg_id
        };

        let (local_offer, pending) = sdp_api.apply().ok_or_else(|| WebRtcClientError::Signalling("Could not create local offer via apply()".into()))?;
        log::info!("Offer to relay {:?}", local_offer.to_sdp_string());

        let relay_channels = vec![
            schema::devlog::bitbridge::DataChannel {
                max_retransmit: 0,
                ordered: false,
                negotiate: RELIABLE_STREAM_ID as i32,
                label: "reliable".to_string(),
            },
            schema::devlog::bitbridge::DataChannel {
                max_retransmit: 0,
                ordered: false,
                negotiate: UNORDERED_MSG_STREAM_ID as i32,
                label: "unordered_msg".to_string(),
            },
            schema::devlog::bitbridge::DataChannel {
                max_retransmit: 0,
                ordered: true,
                negotiate: ORDERED_MSG_STREAM_ID as i32,
                label: "ordered_msg".to_string(),
            },
        ];

        let sdp_string = local_offer.to_sdp_string();
        let relay_ans = signalling.relay_connect(signalling_id, session_id, &sdp_string, relay_channels)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("Relay connect explicit failure: {e:?}")))?;

        log::info!("Got relay answer sdp {:?}", relay_ans);

        if !relay_ans.success {
            return Err(WebRtcClientError::Signalling(format!("Relay connect failure: {:?}", relay_ans.error_message)));
        }

        let answer_sdp = relay_ans.sdp.ok_or_else(|| WebRtcClientError::Signalling("Missing SDP in successful relay reply".to_string()))?;
        let remote_offer = str0m::change::SdpAnswer::from_sdp_string(&answer_sdp).map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;
        
        rtc.sdp_api().accept_answer(pending, remote_offer).map_err(|e| WebRtcClientError::Rtc(e))?;

        let client = Self {
            rtc,
            socket,
            local_addr,
            local_v4_addr,
            local_v6_addr,
            buf: vec![0u8; 2000],
            cached_timeout: Instant::now(),
            channel_ids,
            pending_transmits: std::collections::VecDeque::new(),
        };

        let connected = false;

        let client = client.wait_for_connected(connected).await?;
        Ok(client.spawn_thread())
    }

    /// Spawn the dedicated OS thread and return an RtcHandle.
    fn spawn_thread(self) -> RtcHandle {
        let channel_ids = self.channel_ids;
        let (event_tx, event_rx) = tokio::sync::mpsc::channel::<RtcEvent>(5);
        let (command_tx, command_rx) = tokio::sync::mpsc::channel::<RtcCommand>(16);

        let thread_handle = std::thread::Builder::new()
            .name("rtc-io".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime for RTC thread");

                rt.block_on(async move {
                    Self::run_loop(self, event_tx, command_rx).await;
                });
            })
            .expect("Failed to spawn RTC I/O thread");

        RtcHandle {
            event_rx,
            command_tx,
            channel_ids,
            thread_handle: Some(thread_handle),
        }
    }

    /// The main event loop running on the dedicated OS thread.
    async fn run_loop(
        mut self,
        event_tx: tokio::sync::mpsc::Sender<RtcEvent>,
        mut command_rx: tokio::sync::mpsc::Receiver<RtcCommand>,
    ) {
        log::info!("[rtc-client] RTC I/O thread started");

        while self.rtc.is_alive() {
            // 0. Drain pending transmits
            while let Some((data, channel_id)) = self.pending_transmits.pop_front() {
                if !self.send(&data, channel_id) {
                    self.pending_transmits.push_front((data, channel_id));
                    break;
                }
            }

            // 1. Drain pending commands (non-blocking), but only up to 16 to preserve mpsc backpressure
            if self.pending_transmits.len() < 16 {
                while let Ok(cmd) = command_rx.try_recv() {
                    if self.handle_command(cmd) {
                        log::info!("[rtc-client] RTC I/O thread shutdown by command");
                        let _ = event_tx.send(RtcEvent::Closed).await;
                        return;
                    }
                    if self.pending_transmits.len() >= 16 {
                        break;
                    }
                }
            }

            // 2. Poll events from str0m, send to outside via event_tx
            match self.poll_event().await {
                Ok(Some(event)) => {
                    if event_tx.send(RtcEvent::Str0mEvent(event)).await.is_err() {
                        log::info!("[rtc-client] Event receiver dropped, stopping RTC I/O thread");
                        return;
                    }
                    // After sending an event, loop again to drain more events before waiting
                    continue;
                }
                Ok(None) => {
                    // Timeout — fall through to wait_for_input
                }
                Err(e) => {
                    log::warn!("[rtc-client] RTC poll_event error: {e:?}");
                    let _ = event_tx.send(RtcEvent::Error(e)).await;
                    let _ = event_tx.send(RtcEvent::Closed).await;
                    return;
                }
            }

            // 3. Wait for input (socket recv) OR new commands
            let timeout = self.timeout_duration();
            tokio::select! {
                result = self.wait_for_input(timeout) => {
                    if let Err(e) = result {
                        log::warn!("[rtc-client] RTC wait_for_input error: {e:?}");
                        let _ = event_tx.send(RtcEvent::Error(e)).await;
                        let _ = event_tx.send(RtcEvent::Closed).await;
                        return;
                    }
                }
                Some(cmd) = command_rx.recv(), if self.pending_transmits.len() < 16 => {
                    if self.handle_command(cmd) {
                        log::info!("[rtc-client] RTC I/O thread shutdown by command");
                        let _ = event_tx.send(RtcEvent::Closed).await;
                        return;
                    }
                }
            }
        }

        log::info!("[rtc-client] RTC connection no longer alive, stopping I/O thread");
        let _ = event_tx.send(RtcEvent::Closed).await;
    }

    /// Handle a command. Returns true if the thread should stop.
    fn handle_command(&mut self, cmd: RtcCommand) -> bool {
        match cmd {
            RtcCommand::Send { data, channel_id } => {
                if self.pending_transmits.is_empty() && self.send(&data, channel_id) {
                    // Sent successfully
                } else {
                    self.pending_transmits.push_back((data, channel_id));
                }
                false
            }
            RtcCommand::SetBufferedAmountLowThreshold { channel_id, threshold } => {
                self.set_buffered_amount_low_threshold(channel_id, threshold);
                false
            }
            RtcCommand::Shutdown => {
                self.rtc.disconnect();
                true
            }
        }
    }

    async fn wait_for_connected(mut self, mut connected: bool) -> Result<Self, WebRtcClientError> {
        loop {
            if let Some(event) = self.poll_event().await? {
                match event {
                    Event::Connected => {
                        log::info!("[rtc-client] DTLS Connected");
                        connected = true;
                    }
                    Event::IceConnectionStateChange(state) => {
                        log::info!("[rtc-client] ICE state: {:?}", state);
                        if matches!(state, IceConnectionState::Disconnected) {
                            return Err(WebRtcClientError::Signalling("Peer disconnected during setup".into()));
                        }
                    }
                    _ => {}
                }
            }

            if connected {
                let ready = [
                    self.channel_ids.reliable,
                    self.channel_ids.unordered_msg,
                    self.channel_ids.ordered_msg
                ]
                .iter()
                .all(|&cid| self.rtc.channel(cid).is_some());

                if ready {
                    log::info!("[rtc-client] Connected, negotiated channels ready");
                    return Ok(self);
                }
            }

            if !self.rtc.is_alive() {
                return Err(WebRtcClientError::Signalling("RTC connection closed".into()));
            }

            let timeout = self.timeout_duration();
            self.wait_for_input(timeout).await?;
        }
    }

    async fn poll_event(&mut self) -> Result<Option<Event>, WebRtcClientError> {
        loop {
            match self.rtc.poll_output()? {
                Output::Timeout(t) => {
                    self.cached_timeout = t;
                    return Ok(None);
                }
                Output::Transmit(t) => {
                    let dest = to_v6_mapped(t.destination);
                    let res = tokio::time::timeout(Duration::from_secs(10), self.socket.send_to(&t.contents, dest)).await;
                    match res {
                        Ok(Err(e)) => {
                            log::warn!("[rtc-client] Failed to send to {}: {}", dest, e);
                            return Err(WebRtcClientError::Signalling(format!("Failed to send packet: {:?}", e)));
                        }
                        Err(_) => {
                            log::error!("[rtc-client] Timeout sending packet to {} after 10s", dest);
                            return Err(WebRtcClientError::Signalling("Send packet timed out".to_string()));
                        }
                        _ => {}
                    }
                }
                Output::Event(e) => {
                    if let Event::IceConnectionStateChange(state) = e {
                        if matches!(state, IceConnectionState::Disconnected) {
                            self.rtc.disconnect();
                        }
                    }
                    return Ok(Some(e));
                }
            }
        }
    }

    fn timeout_duration(&self) -> Duration {
        self.cached_timeout.saturating_duration_since(Instant::now())
    }

    async fn wait_for_input(&mut self, timeout: Duration) -> Result<(), WebRtcClientError> {
        if timeout.is_zero() {
            self.rtc.handle_input(Input::Timeout(Instant::now()))?;
            return Ok(());
        }

        match tokio::time::timeout(timeout, self.socket.recv_from(&mut self.buf[..])).await {
            Ok(Ok((n, source))) => {
                let source = from_v6_mapped(source);
                let local = if source.is_ipv4() {
                    self.local_v4_addr.unwrap_or(self.local_addr)
                } else {
                    self.local_v6_addr.unwrap_or(self.local_addr)
                };
                match Receive::new(Protocol::Udp, source, local, &self.buf[..n]) {
                    Ok(receive) => {
                        if let Err(e) = self.rtc.handle_input(Input::Receive(Instant::now(), receive)) {
                            log::warn!("[rtc-client] Input handle packet drop: {}", e);
                        }
                    }
                    Err(e) => {
                        log::warn!("[rtc-client] Failed to parse Receive: {}", e);
                    }
                }
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => {
                self.rtc.handle_input(Input::Timeout(Instant::now()))?;
            }
        }
        Ok(())
    }

    fn send(&mut self, data: &[u8], channel_id: ChannelId) -> bool {
        if let Some(mut ch) = self.rtc.channel(channel_id) {
            match ch.write(true, data) {
                Ok(true) => true,
                Ok(false) => false, 
                Err(e) => {
                    log::error!("[rtc-client] Failed to write to channel {:?}: {:?}", channel_id, e);
                    false
                }
            }
        }
        else {
            false
        }
    }

    fn set_buffered_amount_low_threshold(&mut self, channel_id: ChannelId, threshold: usize) {
        if let Some(mut ch) = self.rtc.channel(channel_id) {
            ch.set_buffered_amount_low_threshold(threshold);
        }
    }
}

fn to_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::new(v4.ip().to_ipv6_mapped().into(), v4.port()),
        v6 => v6
    }
}

fn from_v6_mapped(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V6(v6) => {
            let octets = v6.ip().octets();
            if octets[0..12] == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff] {
                let v4 = std::net::Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]);
                SocketAddr::new(v4.into(), v6.port())
            } else {
                addr
            }
        }
        _ => addr
    }
}

fn extract_remote_candidate_ips(sdp: &str) -> Vec<IpAddr> {
    let mut ips = Vec::new();
    for line in sdp.lines() {
        if !line.contains("candidate:") {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() > 4 {
            if let Ok(ip) = parts[4].parse::<IpAddr>() {
                if !ips.contains(&ip) {
                    ips.push(ip);
                }
            }
        }
    }
    ips
}
