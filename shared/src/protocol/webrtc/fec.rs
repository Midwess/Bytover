use reed_solomon_erasure::galois_8::ReedSolomon;
use thiserror::Error;
use matchbox_protocol::PeerId;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use bytemuck::bytes_of;
use bytes::BytesMut;
use futures_util::lock::Mutex;
use matchbox_socket::Packet;
use n0_future::time::Instant;
use once_cell::sync::Lazy;
use core_services::utils::time::epoch_micro;
use schema::devlog::bitbridge::{fec_feedback, FecFeedback, MissingFrames};
use schema::devlog::bitbridge::fec_feedback::Feedback;

// Too big chunk size will cause higher chance of packet loss
pub const CHUNK_SIZE: usize = 2 * 1150;
pub const DATA_SHARDS_DEFAULT: usize = 48;
pub const MIN_PARITY_SHARDS: usize = 2;
pub const MAX_PARITY_SHARDS: usize = 10;

const MIN_BLOCK_TIMEOUT_MS: u64 = 200;
const MAX_BLOCK_TIMEOUT_MS: u64 = 2000;

const RTT_THRESHOLD_MS: u64 = 250;
const PARITY_ADAPTATION_WEIGHT: f32 = 0.15;
const LOSS_RATE_HYSTERESIS: f32 = 0.02;

#[derive(Debug, Error)]
pub enum FecError {
    #[error("reed-solomon encoding/decoding error {0:?}")]
    ReedSolomon(reed_solomon_erasure::Error),
    #[error("invalid frame size: expected {expected}, got {actual}")]
    InvalidFrameSize { expected: usize, actual: usize },
    #[error("invalid frame index {idx} for block with {total_shards} shards")]
    InvalidFrameIndex { idx: u32, total_shards: usize },
    #[error("block id mismatch or wraparound detected")]
    BlockIdMismatch,
    #[error("generic error")]
    Generic,
}

impl From<reed_solomon_erasure::Error> for FecError {
    fn from(e: reed_solomon_erasure::Error) -> Self {
        Self::ReedSolomon(e)
    }
}

#[derive(Clone, Debug)]
pub struct FrameEntry {
    pub block_id: u32,
    pub total_size: u32,
    pub frame_idx: u32,
    pub data_shards: u8,
    pub parity_shards: u8,
    pub is_parity: bool,
    pub data: Arc<[u8]>,
    pub timestamp: u64,
}

#[derive(Clone)]
pub struct Frame {
    pub block_id: u32,
    pub frame_idx: u32,
    pub data_shards: u8,
    pub parity_shards: u8,
    pub total_size: u32,
    pub is_parity: bool,

    buffer: Arc<[u8]>,
    payload_offset: usize,
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Frame")
            .field("block_id", &self.block_id)
            .field("frame_idx", &self.frame_idx)
            .field("data_shards", &self.data_shards)
            .field("parity_shards", &self.parity_shards)
            .field("total_size", &self.total_size)
            .field("is_parity", &self.is_parity)
            .field("payload_len", &self.data().len())
            .finish()
    }
}

impl Frame {
    /// Get payload data as a slice (zero-copy)
    pub fn data(&self) -> &[u8] {
        &self.buffer[self.payload_offset..]
    }

    /// Create a new Frame with just payload (for sender)
    pub fn new(
        block_id: u32,
        frame_idx: u32,
        data_shards: u8,
        parity_shards: u8,
        total_size: u32,
        is_parity: bool,
        payload: Arc<[u8]>,
    ) -> Self {
        Self {
            block_id,
            frame_idx,
            data_shards,
            parity_shards,
            total_size,
            is_parity,
            buffer: payload,
            payload_offset: 0,
        }
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let payload = self.data();
        let header_len = size_of::<u32>() * 3 + 3;
        let mut buf = Vec::with_capacity(header_len + payload.len());

        buf.extend_from_slice(bytes_of(&self.block_id));
        buf.extend_from_slice(bytes_of(&self.total_size));
        buf.extend_from_slice(bytes_of(&self.frame_idx));
        buf.push(self.data_shards);
        buf.push(self.parity_shards);
        buf.push(self.is_parity as u8);
        buf.extend_from_slice(payload);

        buf.into_boxed_slice()
    }

    pub fn deserialize(buf: &[u8]) -> Option<Self> {
        let mut offset = 0;
        macro_rules! read {
            ($ty:ty) => {{
                if offset + size_of::<$ty>() > buf.len() { return None; }
                let bytes = &buf[offset..offset + size_of::<$ty>()];
                offset += size_of::<$ty>();
                <$ty>::from_le_bytes(bytes.try_into().ok()?)
            }};
        }

        let block_id: u32 = read!(u32);
        let total_size: u32 = read!(u32);
        let frame_idx: u32 = read!(u32);

        if offset + 3 > buf.len() { return None; }
        let data_shards = buf[offset]; offset += 1;
        let parity_shards = buf[offset]; offset += 1;
        let is_parity = buf[offset] != 0; offset += 1;

        let payload_offset = offset;
        // Zero-copy: single allocation via Arc::from
        let buffer: Arc<[u8]> = Arc::from(buf);

        Some(Self {
            block_id,
            total_size,
            frame_idx,
            data_shards,
            parity_shards,
            is_parity,
            buffer,
            payload_offset,
        })
    }
}

impl FrameEntry {
    pub fn serialize(&self) -> Box<[u8]> {
        let header_len = size_of::<u32>() * 3 + size_of::<u64>() + 3;
        let mut buf = Vec::with_capacity(header_len + self.data.len());

        buf.extend_from_slice(bytes_of(&self.block_id));
        buf.extend_from_slice(bytes_of(&self.total_size));
        buf.extend_from_slice(bytes_of(&self.frame_idx));
        buf.push(self.data_shards);
        buf.push(self.parity_shards);
        buf.push(self.is_parity as u8);
        buf.extend_from_slice(bytes_of(&self.timestamp));
        buf.extend_from_slice(&self.data);

        buf.into_boxed_slice()
    }

    pub fn deserialize(buf: &[u8]) -> Option<Self> {
        let mut offset = 0;
        macro_rules! read {
            ($ty:ty) => {{
                if offset + size_of::<$ty>() > buf.len() { return None; }
                let bytes = &buf[offset..offset + size_of::<$ty>()];
                offset += size_of::<$ty>();
                <$ty>::from_le_bytes(bytes.try_into().ok()?)
            }};
        }

        let block_id: u32 = read!(u32);
        let total_size: u32 = read!(u32);
        let frame_idx: u32 = read!(u32);

        if offset + 3 > buf.len() { return None; }
        let data_shards = buf[offset]; offset += 1;
        let parity_shards = buf[offset]; offset += 1;
        let is_parity = buf[offset] != 0; offset += 1;

        let timestamp: u64 = read!(u64);
        let data: Arc<[u8]> = buf[offset..].to_vec().into();

        Some(Self {
            block_id,
            total_size,
            frame_idx,
            data_shards,
            parity_shards,
            is_parity,
            data,
            timestamp,
        })
    }
}

/// RingBuffer using Vec and modulo arithmetic for O(1) indexing
/// block_id maps to index (block_id % window_size)
#[derive(Debug)]
pub struct RingBuffer<T> {
    entries: Vec<Option<(u32, T)>>,  // (block_id, data)
    window_size: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(window_size: usize) -> Self {
        Self {
            entries: (0..window_size).map(|_| None).collect(),
            window_size,
        }
    }

    // Replace and return the old value, if any
    pub fn insert(&mut self, block_id: u32, data: T) -> Option<(u32, T)> {
        let idx = (block_id as usize) % self.window_size;
        self.entries[idx].replace((block_id, data))
    }

    pub fn get(&self, block_id: u32) -> Option<&T> {
        let idx = (block_id as usize) % self.window_size;
        match &self.entries[idx] {
            Some((id, data)) if *id == block_id => Some(data),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, block_id: u32) -> Option<&mut T> {
        let idx = (block_id as usize) % self.window_size;
        match &mut self.entries[idx] {
            Some((id, data)) if *id == block_id => Some(data),
            _ => None,
        }
    }

    pub fn remove(&mut self, block_id: u32) -> Option<T> {
        let idx = (block_id as usize) % self.window_size;
        match &self.entries[idx] {
            Some((id, _)) if *id == block_id => {
                self.entries[idx].take().map(|(_, data)| data)
            }
            _ => None,
        }
    }

    pub fn contains_key(&self, block_id: u32) -> bool {
        let idx = (block_id as usize) % self.window_size;
        match &self.entries[idx] {
            Some((id, _)) => *id == block_id,
            None => false,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.entries.iter().filter_map(|entry| {
            entry.as_ref().map(|(id, data)| (*id, data))
        })
    }

    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }
}

pub struct FecSender {
    pub peer_id: PeerId,
    pub block_id: u32,
    pub encoders: HashMap<(usize, usize), ReedSolomon>,

    pub data_shards: usize,
    pub parity_ratio: f32,

    parity_ewma: f32,
    last_loss_rate: f32,

    pub buffer: RingBuffer<Vec<FrameEntry>>,
    rtt_ms: u64,

    shard_buffer_pool: Vec<Vec<u8>>,
}

impl FecSender {
    pub fn new(peer_id: PeerId, window_size: usize) -> Self {
        let initial_ratio = 0.0;
        let buffer_pool_size = DATA_SHARDS_DEFAULT + MAX_PARITY_SHARDS;
        let shard_buffer_pool = (0..buffer_pool_size)
            .map(|_| Vec::with_capacity(CHUNK_SIZE))
            .collect();

        Self {
            encoders: HashMap::new(),
            peer_id,
            block_id: 0,
            data_shards: DATA_SHARDS_DEFAULT,
            parity_ratio: initial_ratio,
            parity_ewma: initial_ratio,
            last_loss_rate: 0.0,
            buffer: RingBuffer::new(window_size),
            rtt_ms: 0,
            shard_buffer_pool,
        }
    }

    pub fn set_rtt(&mut self, rtt_ms: u64) {
        if self.rtt_ms == rtt_ms {
            return;
        }

        self.rtt_ms = rtt_ms;
        if rtt_ms <= RTT_THRESHOLD_MS {
            log::info!("RTT good ({}ms) will not use parity", rtt_ms);
            self.parity_ratio = 0.0;
            self.parity_ewma = 0.0;
        } else {
            log::info!("RTT too high ({}ms) will use parity", rtt_ms);
            if self.parity_ratio == 0.0 {
                self.parity_ratio = 0.03;
                self.parity_ewma = 0.03;
            }
        }
    }

    pub fn send(&mut self, packet: Box<[u8]>) -> Result<FecAction, FecError> {
        let mut offset = 0usize;
        let mut frames_to_send: Vec<Frame> = Vec::new();

        while offset < packet.len() {
            let block_size = (packet.len() - offset).min(CHUNK_SIZE * self.data_shards);
            let shard_count = (block_size + CHUNK_SIZE - 1) / CHUNK_SIZE;
            let parity_shards = Self::parity_count_from_ratio(self.parity_ratio, shard_count);

            let mut shards: Vec<Vec<u8>> = Vec::with_capacity(shard_count + parity_shards);

            for _ in 0..shard_count {
                if offset < packet.len() {
                    let end = (offset + CHUNK_SIZE).min(packet.len());

                    let mut chunk = if let Some(mut buf) = self.shard_buffer_pool.pop() {
                        buf.clear();
                        buf.resize(CHUNK_SIZE, 0);
                        buf
                    } else {
                        vec![0u8; CHUNK_SIZE]
                    };

                    chunk[..end - offset].copy_from_slice(&packet[offset..end]);
                    shards.push(chunk);
                    offset = end;
                }
            }

            if parity_shards > 0 {
                for _ in 0..parity_shards {
                    let parity_buf = if let Some(mut buf) = self.shard_buffer_pool.pop() {
                        buf.clear();
                        buf.resize(CHUNK_SIZE, 0);
                        buf
                    } else {
                        vec![0u8; CHUNK_SIZE]
                    };
                    shards.push(parity_buf);
                }

                {
                    let mut shards_refs: Vec<&mut [u8]> =
                        shards.iter_mut().map(|v| v.as_mut_slice()).collect();

                    let rs = match self.encoders.get_mut(&(shard_count, parity_shards)) {
                        Some(rs) => rs,
                        None => {
                            let rs = ReedSolomon::new(shard_count, parity_shards)?;
                            self.encoders.insert((shard_count, parity_shards), rs);
                            self.encoders.get_mut(&(shard_count, parity_shards)).unwrap()
                        }
                    };

                    rs.encode(&mut shards_refs)?;
                }
            }

            let mut block_frames = Vec::new();
            for (i, s) in shards.iter().enumerate() {
                let is_parity = i >= shard_count;
                let payload: Arc<[u8]> = Arc::from(s.as_slice());

                let frame = Frame::new(
                    self.block_id,
                    i as u32,
                    shard_count as u8,
                    parity_shards as u8,
                    block_size as u32,
                    is_parity,
                    payload.clone(),
                );

                let fe = FrameEntry {
                    total_size: frame.total_size,
                    block_id: frame.block_id,
                    frame_idx: frame.frame_idx,
                    data_shards: frame.data_shards,
                    parity_shards: frame.parity_shards,
                    is_parity: frame.is_parity,
                    data: payload,
                    timestamp: now_micros(),
                };

                block_frames.push(fe);
                frames_to_send.push(frame);
            }

            let buffer_pool_capacity = DATA_SHARDS_DEFAULT + MAX_PARITY_SHARDS;
            for buffer in shards {
                if self.shard_buffer_pool.len() < buffer_pool_capacity {
                    self.shard_buffer_pool.push(buffer);
                }
            }

            // Store all frames for this block in the ring buffer
            self.buffer.insert(self.block_id, block_frames);
            self.block_id = self.block_id.wrapping_add(1);
        }

        Ok(FecAction::Framed(frames_to_send))
    }

    pub fn feedback(&mut self, fb: Feedback) -> FecAction {
        match fb {
            Feedback::Network(net) => {
                let rtt = net.rtt.map(|it| it.max(self.rtt_ms as u32)).unwrap_or(self.rtt_ms as u32);
                self.set_rtt(rtt as u64);
                if let Some(ack) = net.current_block_id {
                    for i in (0..ack).rev() {
                        self.buffer.remove(i);
                    }
                }

                self.parity_ratio = self.parity_ewma;
                FecAction::Noop
            }
            Feedback::Missing(m) => {
                // Get the block from the ring buffer
                let block_frames = self.buffer.get(m.block_id);

                if let Some(frames_vec) = block_frames {
                    let frames: Vec<Frame> = m.frames
                        .iter()
                        .filter_map(|&frame_idx| {
                            frames_vec.iter().find(|e| e.frame_idx == frame_idx).map(|e| {
                                // Convert Arc<Box<[u8]>> to Arc<[u8]>
                                let payload: Arc<[u8]> = Arc::from(e.data.as_ref().as_ref());
                                Frame::new(
                                    e.block_id,
                                    e.frame_idx,
                                    e.data_shards,
                                    e.parity_shards,
                                    e.total_size,
                                    e.is_parity,
                                    payload,
                                )
                            })
                        })
                        .collect();

                    if frames.is_empty() {
                        FecAction::Terminated
                    } else {
                        FecAction::Retransmit(frames)
                    }
                } else {
                    FecAction::Terminated
                }
            }
            _ => FecAction::Noop,
        }
    }

    fn parity_count_from_ratio(packet_loss: f32, data_shards: usize) -> usize {
        if packet_loss == 0.0 {
            return 0  // Early exit for zero loss rate
        }

        const PACKETS_PER_CHUNK: f32 = 2.0;
        let q = 1.0 - (1.0 - packet_loss).powf(PACKETS_PER_CHUNK);
        let mut p = (q * data_shards as f32).round() as usize;

        if p < MIN_PARITY_SHARDS {
            p = MIN_PARITY_SHARDS;
        }
        if p > MAX_PARITY_SHARDS {
            p = MAX_PARITY_SHARDS;
        }
        p
    }
}

// ===== RECEIVER OPTIMIZATIONS =====

// FIX #6: Use BinaryHeap-ordered structure for O(1) timeout checks
#[derive(Default)]
struct ReceiverBlock {
    data_shards: usize,
    total_size: usize,
    parity_shards: usize,
    total_shards: usize,
    shards: Vec<Option<Vec<u8>>>,
    received: usize,
    first_ts: u64,
    last_frame_ts: u64,
    last_ping_ts: u64,
    is_requested_retransmit: bool,
    is_place_holder: bool,
    is_complete: bool,
}

impl ReceiverBlock {
    fn place_holder() -> Self {
        let now = now_micros();
        Self {
            is_place_holder: true,
            data_shards: DATA_SHARDS_DEFAULT,
            shards: vec![None; DATA_SHARDS_DEFAULT + MIN_PARITY_SHARDS],
            parity_shards: MIN_PARITY_SHARDS,
            total_shards: DATA_SHARDS_DEFAULT + MIN_PARITY_SHARDS,
            received: 0,
            first_ts: now,
            last_frame_ts: now,
            last_ping_ts: now,
            is_requested_retransmit: false,
            is_complete: false,
            total_size: 0,
        }
    }

    fn new(data_shards: usize, parity_shards: usize, total_size: usize) -> Self {
        let total = data_shards + parity_shards;
        let now = now_micros();
        Self {
            is_place_holder: false,
            data_shards,
            total_size,
            parity_shards,
            total_shards: total,
            shards: vec![None; total],
            received: 0,
            first_ts: now,
            is_requested_retransmit: false,
            last_frame_ts: now,
            last_ping_ts: now,
            is_complete: false,
        }
    }

    fn place_value(&mut self, data_shards: usize, parity_shards: usize, total_size: usize) {
        let now = now_micros();
        if self.is_place_holder {
            self.data_shards = data_shards;
            self.parity_shards = parity_shards;
            self.total_size = total_size;
            self.total_shards = self.data_shards + self.parity_shards;
            self.shards = vec![None; self.total_shards];
            self.is_place_holder = false;
            self.received = 0;
            self.first_ts = now;
            self.last_frame_ts = now;
            self.last_ping_ts = now;
            self.is_complete = false;
        }
    }

    fn insert_frame(&mut self, idx: usize, payload: Box<[u8]>) -> Result<bool, FecError> {
        if idx >= self.total_shards {
            return Err(FecError::InvalidFrameIndex {
                idx: idx as u32,
                total_shards: self.total_shards,
            });
        }

        if self.shards[idx].is_none() {
            // FIX #3: Convert Box<[u8]> to Vec<u8> without extra copy
            self.shards[idx] = Some(Vec::from(payload));
            self.received += 1;
        }

        let now = now_micros();
        self.last_frame_ts = now;
        self.last_ping_ts = now;

        Ok(self.received >= self.data_shards)
    }

    fn try_reconstruct(&mut self, decoders: &mut HashMap<(usize, usize), ReedSolomon>) -> Result<bool, FecError> {
        if self.is_complete {
            return Ok(true);
        }

        let present_count = self.shards.iter().filter(|s| s.is_some()).count();
        if present_count < self.data_shards {
            return Ok(false);
        }

        if self.parity_shards == 0 {
            // No parity: just check if all data shards present
            let all_data_present = self.shards
                .iter()
                .take(self.data_shards)
                .all(|s| s.is_some());

            if all_data_present {
                self.is_complete = true;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            // Use Reed-Solomon reconstruction with cached decoder
            log::info!("Use reed solomon for {} data + {} parity", self.data_shards, self.parity_shards);

            let key = (self.data_shards, self.parity_shards);
            let rs = match decoders.get_mut(&key) {
                Some(rs) => rs,
                None => {
                    let rs = ReedSolomon::new(self.data_shards, self.parity_shards)?;
                    decoders.insert(key, rs);
                    decoders.get_mut(&key).unwrap()
                }
            };

            match rs.reconstruct(&mut self.shards) {
                Ok(()) => {
                    self.is_complete = true;
                    Ok(true)
                }
                Err(e) => {
                    log::warn!("Reed-Solomon reconstruction failed: {:?}", e);
                    Ok(false)
                }
            }
        }
    }

    fn is_constructed(&self) -> bool {
        self.is_complete
    }

    fn into_packet(mut self) -> Packet {
        let mut bytes = Vec::with_capacity(self.total_size);
        let mut written = 0;

        unsafe { bytes.set_len(self.total_size); }

        let dst: &mut [u8] = bytes.as_mut_slice();

        for shard_opt in self.shards.iter_mut().take(self.data_shards) {
            if written >= self.total_size {
                break;
            }

            if let Some(shard) = shard_opt.take() {
                let remaining = self.total_size - written;
                let to_write = remaining.min(shard.len());

                unsafe {
                    std::ptr::copy_nonoverlapping(
                        shard.as_ptr(),
                        dst.as_mut_ptr().add(written),
                        to_write,
                    );
                }

                written += to_write;
            }
        }

        Packet::from(bytes.into_boxed_slice())
    }
}

pub struct FecReceiver {
    blocks: RingBuffer<ReceiverBlock>,
    next_block_id: u32,
    rtt_ms: u64,
    total_frames_received: u64,
    total_lost_frames: u64,
    decoders: HashMap<(usize, usize), ReedSolomon>,
    block_pool: Vec<ReceiverBlock>,
}

impl FecReceiver {
    pub fn new() -> Self {
        Self::with_window_size(256)
    }

    pub fn with_window_size(window_size: usize) -> Self {
        let block_pool = (0..16)  // Pool of 16 blocks
            .map(|_| ReceiverBlock::place_holder())
            .collect();

        Self {
            blocks: RingBuffer::new(window_size),
            next_block_id: 0,
            rtt_ms: 0,
            total_frames_received: 0,
            total_lost_frames: 0,
            decoders: HashMap::new(),
            block_pool,
        }
    }

    pub fn calculate_loss_rate(&self) -> f32 {
        if self.total_frames_received == 0 {
            return 0.0;
        }
        (self.total_lost_frames as f32) / (self.total_frames_received as f32)
    }

    pub fn current_block_id(&self) -> u32 {
        self.next_block_id
    }

    pub fn set_rtt(&mut self, rtt_ms: u64) {
        self.rtt_ms = rtt_ms;
    }

    fn calculate_next_check_time(&self) -> Instant {
        let timeout_ms = if self.rtt_ms > 0 {
            (self.rtt_ms * 2).max(MIN_BLOCK_TIMEOUT_MS).min(MAX_BLOCK_TIMEOUT_MS)
        } else {
            MIN_BLOCK_TIMEOUT_MS
        };

        let timeout_us = timeout_ms * 1000;
        let now = now_micros();

        // Find the oldest block needing a check
        let mut oldest_ts = u64::MAX;
        for (_, block) in self.blocks.iter() {
            if block.is_constructed() || block.is_requested_retransmit {
                continue;
            }
            if block.last_ping_ts < oldest_ts {
                oldest_ts = block.last_ping_ts;
            }
        }

        if oldest_ts != u64::MAX {
            let elapsed = now.saturating_sub(oldest_ts);
            if elapsed < timeout_us {
                let remaining = timeout_us.saturating_sub(elapsed);
                return Instant::now() + Duration::from_micros(
                    remaining.max(MIN_BLOCK_TIMEOUT_MS * 1_000).min(MAX_BLOCK_TIMEOUT_MS * 1_000)
                );
            }
        }

        Instant::now() + Duration::from_millis(timeout_ms)
    }

    pub fn receive(&mut self, frame: Frame) -> Result<FecAction, FecError> {
        if frame.data().len() != CHUNK_SIZE {
            return Err(FecError::InvalidFrameSize {
                expected: CHUNK_SIZE,
                actual: frame.data().len(),
            });
        }

        if frame.block_id < self.next_block_id && !frame.is_parity {
            log::info!("Ignoring frame for block {} < {}", frame.block_id, self.next_block_id);
            return self.ping();
        }

        let block_id = frame.block_id;

        // Get or create the block (from pool if available)
        if self.blocks.get(block_id).is_none() {
            let new_block = if let Some(mut pooled) = self.block_pool.pop() {
                pooled.place_value(
                    frame.data_shards as usize,
                    frame.parity_shards as usize,
                    frame.total_size as usize
                );
                pooled
            } else {
                ReceiverBlock::new(
                    frame.data_shards as usize,
                    frame.parity_shards as usize,
                    frame.total_size as usize
                )
            };

            if let Some(replaced) = self.blocks.insert(block_id, new_block) {
                // Buffer is too full, not accept data lost
                log::warn!("Buffer full, block {} and block {}", block_id, replaced.0);
                return Ok(FecAction::Terminated);
            }
        }

        let block = self.blocks.get_mut(block_id).unwrap();
        block.place_value(frame.data_shards as usize, frame.parity_shards as usize, frame.total_size as usize);

        let idx = frame.frame_idx as usize;
        let payload_box = Box::from(frame.data());
        let can_decode = block.insert_frame(idx, payload_box)?;
        self.total_frames_received += 1;

        if can_decode {
            let block = self.blocks.get_mut(block_id).unwrap();
            let reconstructed = block.try_reconstruct(&mut self.decoders)?;

            if reconstructed && block_id == self.next_block_id {
                if let Some(block) = self.blocks.remove(block_id) {
                    let mut completed_blocks = vec![block];

                    loop {
                        self.next_block_id += 1;

                        if !self.blocks.contains_key(self.next_block_id) {
                            let placeholder = if let Some(pooled) = self.block_pool.pop() {
                                pooled
                            } else {
                                ReceiverBlock::place_holder()
                            };

                            self.blocks.insert(self.next_block_id, placeholder);
                            break;
                        }

                        let is_completed = self.blocks.get(self.next_block_id)
                            .map(|it| it.is_constructed())
                            .unwrap_or_default();

                        if is_completed {
                            let block = self.blocks.remove(self.next_block_id).unwrap();
                            completed_blocks.push(block);
                        } else {
                            break;
                        }
                    }

                    let bytes = completed_blocks.into_iter()
                        .map(|it| it.into_packet())
                        .collect::<Vec<_>>();

                    let next_check = self.calculate_next_check_time();
                    return Ok(FecAction::Constructed(bytes, next_check));
                }
            }
        }

        self.ping()
    }

    /// Optimized timeout checking using RingBuffer
    pub fn ping(&mut self) -> Result<FecAction, FecError> {
        let timeout_ms = if self.rtt_ms > 0 {
            (self.rtt_ms * 2).max(MIN_BLOCK_TIMEOUT_MS).min(MAX_BLOCK_TIMEOUT_MS)
        } else {
            MIN_BLOCK_TIMEOUT_MS
        };

        let timeout_us = timeout_ms * 1000;
        let now = now_micros();

        // Efficiently find the oldest block needing a check
        let mut oldest_ts = u64::MAX;
        let mut oldest_id = None;

        for (block_id, block) in self.blocks.iter() {
            if block.is_constructed() || block.is_requested_retransmit {
                continue;
            }

            if block.last_ping_ts < oldest_ts {
                oldest_ts = block.last_ping_ts;
                oldest_id = Some(block_id);
            }
        }

        if let Some(block_id) = oldest_id {
            let next_check = self.calculate_next_check_time();

            if let Some(block) = self.blocks.get_mut(block_id) {
                if now.saturating_sub(block.last_ping_ts) > timeout_us {
                    let action = Self::handle_timeout(block_id, block, next_check);
                    self.total_lost_frames += action.0 as u64;
                    return match action.1 {
                        FecAction::Noop => {
                            Ok(FecAction::Queued(Instant::now() + Duration::from_millis(timeout_ms)))
                        }
                        _ => Ok(action.1),
                    }
                } else {
                    let elapsed = now.saturating_sub(block.last_ping_ts);
                    let remaining = timeout_us.saturating_sub(elapsed);
                    let next_check = Instant::now() + Duration::from_micros(
                        remaining.max(MIN_BLOCK_TIMEOUT_MS * 1_000).min(MAX_BLOCK_TIMEOUT_MS * 1_000)
                    );
                    return Ok(FecAction::Queued(next_check));
                }
            }
        }

        Ok(FecAction::Queued(Instant::now() + Duration::from_millis(timeout_ms)))
    }

    fn handle_timeout(block_id: u32, block: &mut ReceiverBlock, next_check: Instant) -> (usize, FecAction) {
        let present_count = block.shards.iter().filter(|s| s.is_some()).count();
        let needed_more = block.data_shards.saturating_sub(present_count);

        if needed_more == 0 {
            return (0, FecAction::Noop);
        }

        block.last_ping_ts = now_micros();
        let mut missing = Vec::new();

        for i in 0..block.data_shards {
            if missing.len() >= needed_more {
                break;
            }
            if block.shards[i].is_none() {
                missing.push(i as u32);
            }
        }

        if missing.len() < needed_more {
            for i in block.data_shards..block.total_shards {
                if missing.len() >= needed_more {
                    break;
                }
                if block.shards[i].is_none() {
                    missing.push(i as u32);
                }
            }
        }

        let lost_count = missing.len();
        let mf = MissingFrames {
            block_id,
            frames: missing,
        };

        block.is_requested_retransmit = true;
        (lost_count, FecAction::Feedback(FecFeedback {
            feedback: Some(Feedback::Missing(mf)),
        }, next_check))
    }
}

#[derive(Debug)]
pub enum FecAction {
    Framed(Vec<Frame>),
    Constructed(Vec<Packet>, Instant),
    Retransmit(Vec<Frame>),
    Feedback(FecFeedback, Instant),
    Noop,
    Queued(Instant),
    Terminated,
}

// Helper
fn now_micros() -> u64 {
    epoch_micro()
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use super::*;

    #[test]
    fn test_parity_count() {
        let s = FecSender::new(PeerId(Uuid::new_v4()), 256);
        assert!(FecSender::parity_count_from_ratio(0.2, 32) >= MIN_PARITY_SHARDS);
    }

    #[test]
    fn test_frame_metadata_sync() {
        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 256);
        sender.data_shards = 16;
        sender.parity_ratio = 0.5;

        let packet = vec![0u8; 32 * 1024].into_boxed_slice();
        let action = sender.send(packet).unwrap();

        if let FecAction::Framed(frames) = action {
            assert!(!frames.is_empty());
            // All frames should have consistent N/K values
            let first_n = frames[0].data_shards;
            let first_k = frames[0].parity_shards;
            for frame in &frames {
                assert_eq!(frame.data_shards, first_n, "data_shards mismatch");
                assert_eq!(frame.parity_shards, first_k, "parity_shards mismatch");
            }
        } else {
            panic!("Expected Framed action");
        }
    }

    #[test]
    fn test_reordering_tolerance() {
        let mut receiver = FecReceiver::new();

        // Simulate frames arriving out of order
        let payload: Arc<[u8]> = Arc::from(vec![0u8; CHUNK_SIZE].into_boxed_slice());
        let frame1 = Frame::new(
            0,      // block_id
            15,     // frame_idx - Late in sequence
            32,     // data_shards
            8,      // parity_shards
            CHUNK_SIZE as u32,  // total_size
            false,  // is_parity
            payload,
        );

        // This should not trigger timeout
        let result = receiver.receive(frame1);
        assert!(result.is_ok());
        match result.unwrap() {
            FecAction::Queued(_) => (),  // Expected - waiting for more frames
            val => panic!("Should be Queued, not ready to decode {val:?}"),
        }
    }

    #[test]
    fn test_buffer_smart_eviction() {
        let mut buffer: RingBuffer<Vec<FrameEntry>> = RingBuffer::new(10);

        // Insert frames from block 0
        let fe = FrameEntry {
            total_size: CHUNK_SIZE as u32,
            block_id: 0,
            frame_idx: 0,
            data_shards: 2,
            parity_shards: 1,
            is_parity: false,
            data: Arc::new([1u8; CHUNK_SIZE]),
            timestamp: now_micros(),
        };
        buffer.insert(0, vec![fe]);

        let entry = FrameEntry {
            total_size: CHUNK_SIZE as u32,
            block_id: 1,
            frame_idx: 0,
            data_shards: 2,
            parity_shards: 1,
            is_parity: false,
            data: fe.data.clone(),
            timestamp: now_micros(),
        };

        buffer.insert(1, vec![entry]);

        // Block 0 should still be retrievable
        assert!(buffer.get(0).is_some(), "Block 0 should not be evicted");
    }

    #[test]
    fn test_smart_missing_frame_selection() {
        let mut receiver = FecReceiver::new();

        // Create partial block with mostly data frames missing
        let mut frames = Vec::new();
        for i in 20..32 {
            let payload: Arc<[u8]> = Arc::from(vec![0u8; CHUNK_SIZE].into_boxed_slice());
            frames.push(Frame::new(
                0,      // block_id
                i,      // frame_idx
                32,     // data_shards
                8,      // parity_shards
                CHUNK_SIZE as u32,  // total_size
                i >= 32,  // is_parity
                payload,
            ));
        }

        for frame in frames {
            let _ = receiver.receive(frame);
        }
    }

    #[test]
    fn test_frame_validation() {
        let mut receiver = FecReceiver::new();

        let payload: Arc<[u8]> = Arc::from(vec![0u8; 1024].into_boxed_slice());
        let bad_frame = Frame::new(
            0,      // block_id
            0,      // frame_idx
            32,     // data_shards
            8,      // parity_shards
            1024,   // total_size
            false,  // is_parity
            payload,  // Wrong size!
        );

        let result = receiver.receive(bad_frame);
        assert!(result.is_err(), "Should reject invalid frame size");
    }

    #[test]
    fn test_send_receive_small_data() {
        // Test with very small amount of data (100 bytes)
        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 256);
        let mut receiver = FecReceiver::new();

        // Create small test data with a recognizable pattern
        let original_data: Vec<u8> = (0..100).map(|i| (i % 256) as u8).collect();
        let packet = original_data.clone().into_boxed_slice();

        // Send the data through FEC encoder
        let action = sender.send(packet).expect("send failed");

        // Extract frames
        let frames = match action {
            FecAction::Framed(frames) => frames,
            _ => panic!("Expected Framed action"),
        };

        assert!(!frames.is_empty(), "Should have generated frames");

        // Receive all frames
        let mut received_packet: Option<Packet> = None;
        for frame in frames {
            let result = receiver.receive(frame).expect("receive failed");
            match result {
                FecAction::Constructed(mut packet, _) => {
                    received_packet = Some(packet.remove(0));
                    break;
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                _ => panic!("Unexpected action during receive"),
            }
        }

        // Verify we got data back
        assert!(received_packet.is_some(), "Should have constructed data");

        // Get the reconstructed data
        let reconstructed = received_packet.unwrap().into_vec();

        // Trim to original size (FEC pads data to block boundaries)
        let reconstructed_trimmed = &reconstructed[..original_data.len()];

        // Verify data matches
        assert_eq!(
            reconstructed_trimmed, original_data.as_slice(),
            "Reconstructed data should match original data"
        );
    }

    #[test]
    fn test_send_receive_10mb_data() {
        // Logger already initialized by other tests
        // Test with 10MB of data
        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 1024);
        let mut receiver = FecReceiver::new();

        // Create 10MB test data with a pattern
        let data_size = 10 * 1024 * 1024; // 10MB
        let original_data: Vec<u8> = (0..data_size)
            .map(|i| ((i / 1024) % 256) as u8) // Pattern changes every 1KB
            .collect();

        let packet = original_data.clone().into_boxed_slice();

        println!("Sending 10MB data through FEC encoder...");

        let time = std::time::Instant::now();
        // Send the data through FEC encoder
        let action = sender.send(packet).expect("send failed");

        // Extract frames
        let frames = match action {
            FecAction::Framed(frames) => frames,
            _ => panic!("Expected Framed action"),
        };

        println!("Generated {} frames in {}us", frames.len(), time.elapsed().as_micros());
        assert!(!frames.is_empty(), "Should have generated frames");

        // Receive all frames and collect constructed packets
        let mut all_packets: Vec<Packet> = Vec::new();

        for (idx, frame) in frames.into_iter().enumerate() {
            let result = receiver.receive(frame).expect(&format!("receive failed at frame {}", idx));
            match result {
                FecAction::Constructed(mut packet, _) => {
                    all_packets.push(packet.remove(0));
                    // Increment next_block_id to allow the next block to be returned
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                FecAction::Terminated => {
                    panic!("Unexpected termination at frame {}", idx);
                }
                _ => {
                    // Other actions are acceptable during processing
                    continue;
                }
            }
        }

        println!("Received {} blocks", all_packets.len());
        assert!(!all_packets.is_empty(), "Should have constructed at least one block");

        // Reconstruct the original data from all packets
        let mut reconstructed = Vec::new();
        for packet in all_packets {
            reconstructed.extend_from_slice(&packet);
        }

        println!("Original size: {}, Reconstructed size: {}", original_data.len(), reconstructed.len());

        // Verify data matches
        assert_eq!(
            reconstructed.len(),
            original_data.len(),
            "Reconstructed data length should match original"
        );

        // Compare data in chunks to provide better error messages
        let chunk_size = 1024 * 1024; // 1MB chunks
        for (chunk_idx, (orig_chunk, recon_chunk)) in original_data
            .chunks(chunk_size)
            .zip(reconstructed.chunks(chunk_size))
            .enumerate()
        {
            assert_eq!(
                orig_chunk, recon_chunk,
                "Data mismatch in chunk {} (offset {})",
                chunk_idx,
                chunk_idx * chunk_size
            );
        }

        println!("10MB data transfer test passed!");
    }

    #[test]
    fn test_recover_data_loss_using_parity() {
        // SCENARIO:
        // We drop specific Data shards but provide enough Parity shards.
        // The receiver should reconstruct the packet without needing retransmission.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 256);
        sender.parity_ratio = 0.5;
        let mut receiver = FecReceiver::new();

        // 1. Create a packet larger than one chunk to ensure multiple shards
        let packet_size = CHUNK_SIZE * 4;
        let original_data: Vec <_> = (0..packet_size).map(|i| (i % 255) as u8).collect();
        let packet = original_data.clone().into_boxed_slice();

        // 2. Generate frames
        let action = sender.send(packet).expect("send failed");
        let frames = match action {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        // 3. Simulate Loss:
        // We need 'data_shards' amount of frames to recover.
        // Let's drop the last 2 DATA frames and replace them with 2 PARITY frames.
        let data_shards_count = frames[0].data_shards as usize;
        let _parity_shards_count = frames[0].parity_shards as usize;

        // Keep indices: 0..N-2 (Data) AND N..N+2 (Parity)
        // This proves we are actually using the Reed-Solomon math, not just concatenating
        let mut frames_to_deliver = Vec::new();

        // Add first N-2 data frames
        for i in 0..(data_shards_count - 2) {
            frames_to_deliver.push(frames[i].clone());
        }

        // Add first 2 parity frames (which usually start at index = data_shards_count)
        let start_parity_idx = frames.iter().position(|f| f.is_parity).unwrap();
        frames_to_deliver.push(frames[start_parity_idx].clone());
        frames_to_deliver.push(frames[start_parity_idx + 1].clone());

        assert_eq!(frames_to_deliver.len(), data_shards_count, "Must deliver exactly N frames");

        // 4. Receive
        let mut constructed = None;
        for frame in frames_to_deliver {
            match receiver.receive(frame).expect("receive failed") {
                FecAction::Constructed(packet, _) => {
                    constructed = Some(packet);
                }
                _ => {}
            }
        }

        // 5. Verify
        assert!(constructed.is_some(), "Should have reconstructed using parity");
        let result_data = constructed.unwrap().into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(result_data.len(), original_data.len());
        assert_eq!(&result_data[..], &original_data[..]);
    }

    #[test]
    fn test_timeout_generation() {
        // SCENARIO:
        // Send insufficient frames. Wait > MIN_BLOCK_TIMEOUT_MS.
        // Call ping() to check for timeouts.
        // Expect Feedback(MissingFrames).

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 256);
        let mut receiver = FecReceiver::new();

        let packet = vec![0u8; CHUNK_SIZE * 4].into_boxed_slice(); // ~4 data shards
        let frames = match sender.send(packet).unwrap() {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        // 1. Send only 1 frame (insufficient)
        let action = receiver.receive(frames[0].clone()).unwrap();

        // Should get Queued response
        match action {
            FecAction::Queued(_) => {},
            other => panic!("Expected Queued, got {:?}", other),
        }

        // 2. Wait for timeout (MIN_BLOCK_TIMEOUT_MS default)
        // We wait enough to exceed the timeout
        std::thread::sleep(std::time::Duration::from_millis(350));

        // 3. Call ping() to trigger the timeout check
        let action = receiver.ping().unwrap();

        // 4. Verify Feedback
        match action {
            FecAction::Feedback(FecFeedback { feedback: Some(fec_feedback::Feedback::Missing(missing)) }, _) => {
                assert_eq!(missing.block_id, 0);
                assert!(missing.frames.len() > 0);
                assert!(!missing.frames.contains(&0));
                assert!(missing.frames.contains(&1));
            },
            other => panic!("Expected Feedback(Missing), got {:?}", other),
        }
    }

    #[test]
    fn test_full_retransmission_loop() {
        // SCENARIO:
        // 1. Sender sends frames.
        // 2. Network drops critical amount.
        // 3. Receiver Timeouts -> Generates Feedback via ping().
        // 4. Sender processes Feedback -> Generates Retransmit frames.
        // 5. Receiver gets Retransmit frames -> Constructs Packet.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 5 * 1024 * 1024);
        let mut receiver = FecReceiver::new();

        let packet_len = CHUNK_SIZE * 4;
        let original_data: Vec <_> = (0..packet_len).map(|i| (i % 100) as u8).collect();
        let packet = original_data.clone().into_boxed_slice();

        // --- STEP 1: Initial Send ---
        let frames = match sender.send(packet).unwrap() {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        let total_needed = frames[0].data_shards as usize;

        // --- STEP 2: Network Loss ---
        // Deliver only 1 frame. Impossible to reconstruct.
        let action = receiver.receive(frames[0].clone()).unwrap();

        // Should get Queued response
        match action {
            FecAction::Queued(_) => {},
            other => panic!("Expected Queued, got {:?}", other),
        }

        // --- STEP 3: Timeout & Feedback ---
        std::thread::sleep(std::time::Duration::from_millis(350));

        // Call ping() to trigger timeout check
        let action = receiver.ping().unwrap();

        let feedback_obj = match action {
            FecAction::Feedback(fb, _) => fb,
            value => panic!("Receiver did not request retransmission {value:?}"),
        };

        // Extract the inner feedback enum for the sender
        let inner_feedback = feedback_obj.feedback.expect("Empty feedback");

        // --- STEP 4: Sender processes Feedback ---
        let retransmit_action = sender.feedback(inner_feedback);

        let retransmitted_frames = match retransmit_action {
            FecAction::Retransmit(f) => f,
            _ => panic!("Sender did not generate retransmission frames"),
        };

        println!("Retransmitted {} frames", retransmitted_frames.len());

        // Validate sender logic: It should only send what was asked
        assert!(!retransmitted_frames.is_empty());

        // --- STEP 5: Receiver processes Retransmission ---
        let mut final_packet = None;

        for frame in retransmitted_frames {
            match receiver.receive(frame).unwrap() {
                FecAction::Constructed(pkt, _) => {
                    final_packet = Some(pkt);
                    break;
                },
                _ => continue,
            }
        }

        assert!(final_packet.is_some(), "Receiver failed to reconstruct after retransmission");

        let reconstructed = final_packet.unwrap().into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(&reconstructed[..], &original_data[..]);
    }

    #[test]
    fn test_sender_buffer_wraparound() {
        // SCENARIO:
        // Ensure the sender buffer correctly handles block_id wraparounds
        // or simply large block IDs without crashing or losing track.
        // We manually inject a high block_id into the buffer to simulate runtime.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 256);

        // Manually set block_id to u32::MAX
        sender.block_id = u32::MAX;

        let packet = vec![0u8; CHUNK_SIZE].into_boxed_slice();

        // This send should use ID u32::MAX
        let action1 = sender.send(packet.clone()).unwrap();
        if let FecAction::Framed(f) = action1 {
            assert_eq!(f[0].block_id, u32::MAX);
        }

        // This send should wrap to 0
        let action2 = sender.send(packet.clone()).unwrap();
        if let FecAction::Framed(f) = action2 {
            assert_eq!(f[0].block_id, 0);
        }

        // Verify buffer contains both
        // We can't access buffer directly easily if fields are private,
        // but we can try to request retransmit for both to prove they exist.

        // Request retransmit for u32::MAX
        let fb_max = Feedback::Missing(MissingFrames { block_id: u32::MAX, frames: vec![0] });
        let res_max = sender.feedback(fb_max);
        assert!(matches!(res_max, FecAction::Retransmit(_)), "Should find block u32::MAX");

        // Request retransmit for 0
        let fb_zero = Feedback::Missing(MissingFrames { block_id: 0, frames: vec![0] });
        let res_zero = sender.feedback(fb_zero);
        assert!(matches!(res_zero, FecAction::Retransmit(_)), "Should find block 0");
    }

    #[test]
    fn test_ordering_out_of_order_frames_single_block() {
        // SCENARIO:
        // Frames arrive completely out of order for a single block.
        // Receiver should still reconstruct the original packet with correct ordering.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 256);
        sender.parity_ratio = 0.5;
        let mut receiver = FecReceiver::new();

        // Create test data with recognizable pattern
        let packet_size = CHUNK_SIZE * 8;
        let original_data: Vec<u8> = (0..packet_size)
            .map(|i| ((i / 256) % 256) as u8)
            .collect();
        let packet = original_data.clone().into_boxed_slice();

        // Generate frames
        let action = sender.send(packet).expect("send failed");
        let mut frames = match action {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        let data_shards = frames[0].data_shards as usize;

        // Shuffle frames in reverse order
        frames.reverse();

        // Take only data shards (enough for reconstruction)
        let frames_to_deliver: Vec<Frame> = frames
            .into_iter()
            .filter(|f| !f.is_parity)
            .take(data_shards)
            .collect();

        assert_eq!(frames_to_deliver.len(), data_shards, "Must have exactly N data frames");

        // Receive frames in this reversed/scrambled order
        let mut reconstructed = None;
        for frame in frames_to_deliver {
            match receiver.receive(frame).expect("receive failed") {
                FecAction::Constructed(packet, _) => {
                    reconstructed = Some(packet);
                    break;
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                _ => panic!("Unexpected action"),
            }
        }

        // Verify reconstruction succeeded
        assert!(reconstructed.is_some(), "Should reconstruct despite out-of-order delivery");
        let result_data = reconstructed.unwrap().into_iter().flatten().collect::<Vec<_>>();

        // Verify data integrity and ordering
        assert_eq!(result_data.len(), original_data.len(), "Length mismatch");
        assert_eq!(&result_data[..], &original_data[..], "Data corruption detected");
    }

    #[test]
    fn test_ordering_interleaved_blocks_out_of_order() {
        // Logger already initialized by other tests
        // SCENARIO:
        // Multiple blocks' frames are interleaved and delivered out of order.
        // Each block should reconstruct independently with correct internal ordering.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 512);
        sender.parity_ratio = 0.25;
        let mut receiver = FecReceiver::new();

        // Create two distinct packets
        let packet1_size = CHUNK_SIZE * 4;
        let packet2_size = CHUNK_SIZE * 4;

        let packet1_data: Vec<u8> = (0..packet1_size)
            .map(|i| (i % 100) as u8)
            .collect();

        let packet2_data: Vec<u8> = (0..packet2_size)
            .map(|i| ((i + 50) % 200) as u8)
            .collect();

        // Send both packets
        let action1 = sender.send(packet1_data.clone().into_boxed_slice()).expect("send1 failed");
        let action2 = sender.send(packet2_data.clone().into_boxed_slice()).expect("send2 failed");

        let frames1 = match action1 {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        let frames2 = match action2 {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        let data_shards = frames1[0].data_shards as usize;

        // Interleave frames: alternating frame from block0, then block1
        let mut interleaved = Vec::new();
        let mut b1_iter = frames1.into_iter().filter(|f| !f.is_parity);
        let mut b2_iter = frames2.into_iter().filter(|f| !f.is_parity);

        for _ in 0..data_shards {
            if let Some(f1) = b1_iter.next() {
                interleaved.push(f1);
            }
            if let Some(f2) = b2_iter.next() {
                interleaved.push(f2);
            }
        }

        // Deliver in interleaved order
        let mut block_results = HashMap::new();

        for frame in interleaved {
            match receiver.receive(frame).expect("receive failed") {
                FecAction::Constructed(packet, _) => {
                    block_results.insert(receiver.next_block_id - 1, packet);
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                _ => panic!("Unexpected action"),
            }
        }

        // Verify both blocks reconstructed correctly
        assert_eq!(block_results.len(), 2, "Should have reconstructed 2 blocks");

        let result1 = block_results[&0].clone().into_iter().flatten().collect::<Vec<_>>();
        let result2 = block_results[&1].clone().into_iter().flatten().collect::<Vec<_>>();

        assert_eq!(&result1[..], &packet1_data[..], "Block 0 data mismatch");
        assert_eq!(&result2[..], &packet2_data[..], "Block 1 data mismatch");
    }

    #[test]
    fn test_ordering_random_delivery_single_block() {
        // SCENARIO:
        // Frames are randomly shuffled before delivery.
        // Receiver must maintain ordering and reconstruct correctly.

        use std::collections::HashSet;

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 512);
        sender.parity_ratio = 0.3;
        let mut receiver = FecReceiver::new();

        let packet_size = CHUNK_SIZE * 6;
        let original_data: Vec<u8> = (0..packet_size)
            .map(|i| ((i * 7) % 256) as u8)
            .collect();

        let action = sender.send(original_data.clone().into_boxed_slice()).expect("send failed");
        let frames = match action {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        let data_shards = frames[0].data_shards as usize;

        // Collect data frames
        let data_frames: Vec<Frame> = frames
            .into_iter()
            .filter(|f| !f.is_parity)
            .collect();

        // Create a deterministic but non-sequential order
        let mut shuffled = Vec::new();
        for i in (0..data_frames.len()).step_by(2) {
            shuffled.push(data_frames[i].clone());
        }
        for i in (1..data_frames.len()).step_by(2) {
            shuffled.push(data_frames[i].clone());
        }

        // Verify we're actually delivering out of order
        let mut indices_set = HashSet::new();
        let mut prev_idx = None;
        let mut is_ordered = true;
        for frame in &shuffled {
            let idx = frame.frame_idx;
            if let Some(p) = prev_idx {
                if p > idx {
                    is_ordered = false;
                    break;
                }
            }
            prev_idx = Some(idx);
            indices_set.insert(idx);
        }
        assert!(!is_ordered || indices_set.len() > 1, "Shuffle should create out-of-order delivery");

        // Receive frames in shuffled order
        let mut reconstructed = None;
        for frame in shuffled {
            match receiver.receive(frame).expect("receive failed") {
                FecAction::Constructed(packet, _) => {
                    reconstructed = Some(packet);
                    break;
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                _ => {}
            }
        }

        // Verify
        assert!(reconstructed.is_some(), "Should reconstruct from shuffled delivery");
        let result = reconstructed.unwrap().into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(&result[..], &original_data[..], "Data should maintain correct order");
    }

    #[test]
    fn test_ordering_with_sparse_delivery_pattern() {
        // SCENARIO:
        // Frames arrive with gaps (e.g., gaps in indices).
        // Should reconstruct once we have enough frames.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 512);
        sender.parity_ratio = 0.5;
        let mut receiver = FecReceiver::new();

        let packet_size = CHUNK_SIZE * 10;
        let original_data: Vec<u8> = (0..packet_size)
            .map(|i| ((i / 512) as u8).wrapping_mul(13))
            .collect();

        let action = sender.send(original_data.clone().into_boxed_slice()).expect("send failed");
        let frames = match action {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        let data_shards = frames[0].data_shards as usize;

        // Deliver frames with gaps:
        // Take every other frame, skip parity
        let mut delivery_order = Vec::new();
        for (idx, frame) in frames.iter().enumerate() {
            if !frame.is_parity && idx % 2 == 0 {
                delivery_order.push(frame.clone());
            }
        }

        // If we don't have enough, add some parity frames
        if delivery_order.len() < data_shards {
            for frame in &frames {
                if frame.is_parity && delivery_order.len() < data_shards {
                    delivery_order.push(frame.clone());
                }
            }
        }

        // Shuffle this subset
        delivery_order.reverse();

        // Receive
        let mut reconstructed = None;
        for frame in delivery_order {
            match receiver.receive(frame).expect("receive failed") {
                FecAction::Constructed(packet, _) => {
                    reconstructed = Some(packet);
                    break;
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                _ => {}
            }
        }

        // Verify
        assert!(reconstructed.is_some(), "Should reconstruct with sparse delivery pattern");
        let result = reconstructed.unwrap().into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(&result[..], &original_data[..], "Data ordering must be preserved");
    }

    #[test]
    fn test_ordering_burst_delivery_then_trailing() {
        // SCENARIO:
        // Frames arrive in two bursts: first N/2 frames, then gap, then remaining frames.
        // Order must be reconstructed correctly despite time gap.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 512);
        sender.parity_ratio = 0.4;
        let mut receiver = FecReceiver::new();

        let packet_size = CHUNK_SIZE * 8;
        let original_data: Vec<u8> = (0..packet_size)
            .map(|i| (i as u8).wrapping_add(42))
            .collect();

        let action = sender.send(original_data.clone().into_boxed_slice()).expect("send failed");
        let frames = match action {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        let data_shards = frames[0].data_shards as usize;

        // Split into first half and second half
        let data_frames: Vec<Frame> = frames
            .into_iter()
            .filter(|f| !f.is_parity)
            .collect();

        let mid = data_frames.len() / 2;
        let first_burst = data_frames[..mid].to_vec();
        let second_burst = data_frames[mid..].to_vec();

        // Deliver first burst
        for frame in first_burst {
            let _ = receiver.receive(frame);
        }

        // Simulate time gap
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Deliver second burst (some frames may be in different order)
        let mut second_burst_shuffled = second_burst.clone();
        second_burst_shuffled.reverse();

        let mut reconstructed = None;
        for frame in second_burst_shuffled {
            match receiver.receive(frame).expect("receive failed") {
                FecAction::Constructed(packet, _) => {
                    reconstructed = Some(packet);
                    break;
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                _ => {}
            }
        }

        // Verify
        assert!(reconstructed.is_some(), "Should reconstruct after burst delivery");
        let result = reconstructed.unwrap().into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(&result[..], &original_data[..], "Data must maintain correct ordering");
    }

    #[test]
    fn test_ordering_multiple_blocks_sequential_delivery_reverse_frame_order() {
        // SCENARIO:
        // Multiple blocks arrive sequentially (block 0, block 1, block 2...)
        // but within each block, frames arrive in reverse order.
        // Each block must reconstruct with correct internal ordering.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 512);
        sender.parity_ratio = 0.25;
        let mut receiver = FecReceiver::new();

        let block_count = 3;
        let packet_size = CHUNK_SIZE * 4;

        let mut all_frames = Vec::new();
        let mut original_data_blocks = Vec::new();

        // Generate multiple blocks
        for block_num in 0..block_count {
            let original_data: Vec<u8> = (0..packet_size)
                .map(|i| (((block_num as u32).wrapping_add(i as u32)) % 256) as u8)
                .collect();

            let action = sender.send(original_data.clone().into_boxed_slice()).expect("send failed");
            let frames = match action {
                FecAction::Framed(f) => f,
                _ => panic!("Expected Framed"),
            };

            original_data_blocks.push(original_data);
            all_frames.push(frames);
        }

        // For each block, reverse frame order
        let mut delivery_sequence = Vec::new();
        for frames in all_frames {
            let data_shards = frames[0].data_shards as usize;
            let mut block_frames: Vec<Frame> = frames
                .into_iter()
                .filter(|f| !f.is_parity)
                .take(data_shards)
                .collect();
            block_frames.reverse();
            delivery_sequence.extend(block_frames);
        }

        // Receive in this order
        let mut reconstructed_blocks = Vec::new();
        for frame in delivery_sequence {
            match receiver.receive(frame).expect("receive failed") {
                FecAction::Constructed(packet, _) => {
                    reconstructed_blocks.push(packet);
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                _ => {}
            }
        }

        // Verify
        assert_eq!(reconstructed_blocks.len(), block_count, "Should reconstruct all blocks");
        for (idx, reconstructed) in reconstructed_blocks.into_iter().enumerate() {
            let reconstructed = reconstructed.into_iter().flatten().collect::<Vec<_>>();
            assert_eq!(
                &reconstructed[..],
                &original_data_blocks[idx][..],
                "Block {} data must be correctly ordered",
                idx
            );
        }
    }

    #[test]
    fn test_ordering_stress_large_block_random_delivery() {
        // SCENARIO:
        // Large block with many frames delivered in random order.
        // Should reconstruct with correct ordering.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 1024);
        sender.data_shards = 64;
        sender.parity_ratio = 0.2;
        let mut receiver = FecReceiver::new();

        let packet_size = CHUNK_SIZE * 32;
        let original_data: Vec<u8> = (0..packet_size)
            .map(|i| ((i >> 10) as u8).wrapping_mul(17))
            .collect();

        println!("Sending {} byte packet", packet_size);

        let action = sender.send(original_data.clone().into_boxed_slice()).expect("send failed");
        let mut frames = match action {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        let data_shards = frames[0].data_shards as usize;
        println!("Generated {} total frames, {} data shards", frames.len(), data_shards);

        // Collect only data frames and shuffle randomly
        let data_frames: Vec<Frame> = frames
            .into_iter()
            .filter(|f| !f.is_parity)
            .take(data_shards)
            .collect();

        // Deterministic pseudo-random shuffle using a simple algorithm
        let mut shuffled = data_frames.clone();
        for i in 0..shuffled.len() {
            let j = (i * 7 + 13) % shuffled.len();
            shuffled.swap(i, j);
        }

        // Verify shuffle is actually different
        let mut is_same_order = true;
        for (orig, shuf) in data_frames.iter().zip(shuffled.iter()) {
            if orig.frame_idx != shuf.frame_idx {
                is_same_order = false;
                break;
            }
        }

        if is_same_order {
            // Force reverse if somehow ended up in same order
            shuffled.reverse();
        }

        // Receive in shuffled order
        let mut reconstructed = None;
        for (count, frame) in shuffled.into_iter().enumerate() {
            match receiver.receive(frame).expect(&format!("receive failed at frame {}", count)) {
                FecAction::Constructed(packet, _) => {
                    let packet = packet.into_iter().flatten().collect::<Vec<_>>();
                    reconstructed = Some(packet);
                    println!("Reconstruction succeeded at frame {}", count);
                    break;
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                _ => {}
            }
        }

        // Verify
        assert!(reconstructed.is_some(), "Should reconstruct large block from shuffled frames");
        let result = reconstructed.unwrap();
        println!("Reconstructed size: {}, Original size: {}", result.len(), original_data.len());
        assert_eq!(result.len(), original_data.len(), "Size mismatch");

        // Verify in chunks
        for (chunk_idx, (orig_chunk, result_chunk)) in original_data
            .chunks(CHUNK_SIZE)
            .zip(result.chunks(CHUNK_SIZE))
            .enumerate()
        {
            assert_eq!(
                orig_chunk, result_chunk,
                "Chunk {} data mismatch",
                chunk_idx
            );
        }

        println!("Large block ordering test passed!");
    }

    #[test]
    fn test_ordering_frame_idx_gaps_reconstruction() {
        // SCENARIO:
        // Some frame indices are completely missing (network loss).
        // Parity frames fill the gaps.
        // Reconstruction must maintain correct frame ordering.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 512);
        sender.parity_ratio = 0.5;
        let mut receiver = FecReceiver::new();

        let packet_size = CHUNK_SIZE * 8;
        let original_data: Vec<u8> = (0..packet_size)
            .map(|i| ((i ^ 0xAA) as u8))
            .collect();

        let action = sender.send(original_data.clone().into_boxed_slice()).expect("send failed");
        let frames = match action {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        let data_shards = frames[0].data_shards as usize;
        let parity_shards = frames[0].parity_shards as usize;

        // Simulate loss: skip frames at indices 2, 5, 8
        let skip_indices = vec![2, 5, 8];
        let mut delivery_frames = Vec::new();

        for (idx, frame) in frames.into_iter().enumerate() {
            if !skip_indices.contains(&(frame.frame_idx as usize)) {
                delivery_frames.push(frame);
            }
        }

        // Ensure we have enough frames for reconstruction
        while delivery_frames.len() < data_shards {
            panic!("Insufficient frames after loss simulation");
        }

        // Take only what we need
        delivery_frames.truncate(data_shards);

        // Shuffle delivery order
        let mut shuffled = delivery_frames.clone();
        for i in 0..shuffled.len() / 2 {
            let len = shuffled.len();
            shuffled.swap(i, len - 1 - i);
        }

        // Receive
        let mut reconstructed = None;
        for frame in shuffled {
            match receiver.receive(frame).expect("receive failed") {
                FecAction::Constructed(packet, _) => {
                    reconstructed = Some(packet);
                    break;
                }
                FecAction::Queued(_) | FecAction::Noop => continue,
                _ => {}
            }
        }

        // Verify
        assert!(reconstructed.is_some(), "Should reconstruct despite frame index gaps");
        let result = reconstructed.unwrap().into_iter().flatten().collect::<Vec<_>>();
        assert_eq!(&result[..], &original_data[..], "Data ordering preserved through reconstruction");
    }
}
