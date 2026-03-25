use std::net::SocketAddr;
use std::time::{Duration, Instant};

use str0m::change::SdpOffer;
use str0m::channel::ChannelId;
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, Input, Output, Rtc, RtcConfig};
use thiserror::Error;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::webrtc::ice::IceAgent;
use crate::webrtc::signalling::SignalingClient;
use crate::webrtc::socket::{SyncUdpSocket, SyncUdpSocketError};
use schema::devlog::rpc_signalling::server::OfferMessage;

/// Logical data channels used by the native client.
/// IDs are fixed: a fresh answerer-side Rtc with no pre-allocated channels
/// always receives these ChannelIds in opening order from the matchbox peer.
const TOTAL_CHANNELS: usize = 4;

/// Construct a `ChannelId` from its raw sequential index.
///
/// # Safety
/// `ChannelId` is `ChannelId(usize)` — a single-field tuple struct whose
/// memory layout is identical to `usize`. The values 1–4 are the fixed
/// IDs assigned by str0m on a fresh answerer-side `Rtc` instance.
const fn channel_id(raw: usize) -> ChannelId {
    // SAFETY: single-field tuple struct, same layout as usize.
    unsafe { std::mem::transmute(raw) }
}

const MSG_CHANNEL_ID: ChannelId = channel_id(1);
const RELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(2);
const UNRELIABLE_DATA_CHANNEL_ID: ChannelId = channel_id(3);
const UNORDERED_MSG_CHANNEL_ID: ChannelId = channel_id(4);

#[derive(Debug, Error)]
pub enum WebRtcClientError {
    #[error("Rtc error: {0}")]
    Rtc(#[from] str0m::error::RtcError),

    #[error("SDP parse error: {0}")]
    SdpParse(String),

    #[error("Signalling error: {0}")]
    Signalling(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Socket error: {0}")]
    Socket(#[from] SyncUdpSocketError),
}

pub struct WebRtcClient {
    rtc: Rtc,
    socket: SyncUdpSocket,
    signalling: SignalingClient,
    local_addr: SocketAddr,

    // Receivers polled in the run loop to forward outbound data into RTC
    msg_rx: UnboundedReceiver<Vec<u8>>,
    reliable_data_rx: UnboundedReceiver<Vec<u8>>,
    unreliable_data_rx: UnboundedReceiver<Vec<u8>>,
    unordered_msg_rx: UnboundedReceiver<Vec<u8>>,

    // Senders exposed to callers to push data into the respective channels
    pub msg_tx: UnboundedSender<Vec<u8>>,
    pub reliable_data_tx: UnboundedSender<Vec<u8>>,
    pub unreliable_data_tx: UnboundedSender<Vec<u8>>,
    pub unordered_msg_tx: UnboundedSender<Vec<u8>>,
}

impl WebRtcClient {
    pub async fn connect(
        offer_message: OfferMessage,
        socket: SyncUdpSocket,
        signalling: SignalingClient,
        request_id: String,
        ice_agent: IceAgent,
    ) -> Result<Self, WebRtcClientError> {
        let mut rtc = RtcConfig::new().build(Instant::now());

        let local_addr = socket.local_addr()?;
        let host_candidate = Candidate::host(local_addr, "udp")
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;
        rtc.add_local_candidate(host_candidate);
        log::info!("[webrtc-client] Added host candidate: {local_addr}");

        ice_agent.gather_candidates(&mut rtc, local_addr).await;

        let offer = SdpOffer::from_sdp_string(&offer_message.sdp)
            .map_err(|e| WebRtcClientError::SdpParse(format!("{e}")))?;

        let answer = rtc
            .sdp_api()
            .accept_offer(offer)
            .map_err(WebRtcClientError::Rtc)?;

        log::info!("[webrtc-client] SDP answer created with all local candidates");

        signalling
            .send_answer(answer.to_sdp_string(), &request_id)
            .await
            .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;

        log::info!("[webrtc-client] Answer sent, waiting for connection and all channels");

        let (msg_tx, msg_rx) = unbounded_channel::<Vec<u8>>();
        let (reliable_data_tx, reliable_data_rx) = unbounded_channel::<Vec<u8>>();
        let (unreliable_data_tx, unreliable_data_rx) = unbounded_channel::<Vec<u8>>();
        let (unordered_msg_tx, unordered_msg_rx) = unbounded_channel::<Vec<u8>>();

        let mut channels_opened: usize = 0;
        let mut is_connected = false;

        let mut buf = vec![0u8; 2000];
        loop {
            let timeout = match rtc.poll_output()? {
                Output::Timeout(t) => t,
                Output::Transmit(t) => {
                    socket.send_to(&t.contents, t.destination).await?;
                    continue;
                }
                Output::Event(e) => {
                    match &e {
                        Event::Connected => {
                            log::info!("[webrtc-client] Connected");
                            is_connected = true;
                        }
                        Event::ChannelOpen(_, label) => {
                            channels_opened += 1;
                            log::info!(
                                "[webrtc-client] Channel {} opened (label: {})",
                                channels_opened,
                                label
                            );
                        }
                        Event::IceConnectionStateChange(state) => {
                            log::info!("[webrtc-client] ICE state: {:?}", state);
                            if matches!(state, IceConnectionState::Disconnected) {
                                return Err(WebRtcClientError::Signalling(
                                    "Peer disconnected during setup".into(),
                                ));
                            }
                        }
                        _ => {}
                    }

                    if is_connected && channels_opened >= TOTAL_CHANNELS {
                        log::info!("[webrtc-client] All channels open, ready");
                        return Ok(Self {
                            rtc,
                            socket,
                            signalling,
                            local_addr,
                            msg_rx,
                            reliable_data_rx,
                            unreliable_data_rx,
                            unordered_msg_rx,
                            msg_tx,
                            reliable_data_tx,
                            unreliable_data_tx,
                            unordered_msg_tx,
                        });
                    }
                    continue;
                }
            };

            let duration = timeout.saturating_duration_since(Instant::now());
            if duration.is_zero() {
                rtc.handle_input(Input::Timeout(Instant::now()))?;
                continue;
            }

            tokio::select! {
                result = socket.recv_any(&mut buf) => {
                    let (n, source) = result?;
                    let receive = Receive::new(Protocol::Udp, source, local_addr, &buf[..n])
                        .map_err(|e| WebRtcClientError::Signalling(format!("{e}")))?;
                    rtc.handle_input(Input::Receive(Instant::now(), receive))?;
                }
                _ = tokio::time::sleep(duration) => {
                    log::warn!("[webrtc-client] Timeout during handshake, sending keepalive");
                    rtc.handle_input(Input::Timeout(Instant::now()))?;
                }
            }
        }
    }

    pub async fn run(&mut self) -> Result<(), WebRtcClientError> {
        let mut buf = vec![0u8; 2000];
        loop {
            if !self.rtc.is_alive() {
                return Ok(());
            }

            // Drain all pending output before waiting for new I/O or outbound data.
            let timeout = loop {
                match self.rtc.poll_output()? {
                    Output::Timeout(t) => break t,
                    Output::Transmit(t) => {
                        self.socket.send_to(&t.contents, t.destination).await?;
                    }
                    Output::Event(e) => {
                        if let Event::IceConnectionStateChange(state) = e {
                            log::info!("[webrtc-client] ICE state: {:?}", state);
                            if matches!(state, IceConnectionState::Disconnected) {
                                self.rtc.disconnect();
                            }
                        }
                    }
                }
            };

            let duration = timeout
                .saturating_duration_since(Instant::now())
                .max(Duration::from_millis(1));

            tokio::select! {
                result = self.socket.recv_any(&mut buf) => {
                    let (n, source) = result?;
                    let Ok(receive) = Receive::new(Protocol::Udp, source, self.local_addr, &buf[..n]) else {
                        continue;
                    };
                    self.rtc.handle_input(Input::Receive(Instant::now(), receive))?;
                }
                _ = tokio::time::sleep(duration) => {
                    self.rtc.handle_input(Input::Timeout(Instant::now()))?;
                }
                Some(data) = self.msg_rx.recv() => {
                    if let Some(mut ch) = self.rtc.channel(MSG_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = self.reliable_data_rx.recv() => {
                    if let Some(mut ch) = self.rtc.channel(RELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = self.unreliable_data_rx.recv() => {
                    if let Some(mut ch) = self.rtc.channel(UNRELIABLE_DATA_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
                Some(data) = self.unordered_msg_rx.recv() => {
                    if let Some(mut ch) = self.rtc.channel(UNORDERED_MSG_CHANNEL_ID) {
                        ch.write(true, &data).ok();
                    }
                }
            }
        }
    }
}
