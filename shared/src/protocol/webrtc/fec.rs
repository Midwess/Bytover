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
use schema::devlog::bitbridge::{fec_feedback, FecFeedback, MissingFrames, MissingBlocks};
use schema::devlog::bitbridge::fec_feedback::Feedback;

// Too big chunk size will cause higher chance of packet loss
pub const CHUNK_SIZE: usize = 2 * 1100;
pub const DATA_SHARDS_DEFAULT: usize = 48;
pub const MIN_PARITY_SHARDS: usize = 2;
pub const MAX_PARITY_SHARDS: usize = 10;
const MAX_BLOCK_TIMEOUT_MS: u64 = 500;
const RTT_THRESHOLD_MS: u64 = 250;

const PACKET_THRESHOLD: u32 = 3;
const K_TIME_THRESHOLD: f64 = 9.0 / 8.0;
const MIN_LOSS_DELAY_US: u64 = 20 * 1_000;

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

    pub rtt_estimator: RttEstimator,
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
            rtt_estimator: RttEstimator::new(),
            encoders: HashMap::new(),
            peer_id,
            block_id: 0,
            data_shards: DATA_SHARDS_DEFAULT,
            parity_ratio: initial_ratio,
            parity_ewma: initial_ratio,
            last_loss_rate: 0.0,
            buffer: RingBuffer::new(window_size),
            rtt_ms: 50,
            shard_buffer_pool,
        }
    }

    pub fn rtt(&self) -> u64 {
        self.rtt_ms
    }

    pub fn set_rtt(&mut self, rtt_ms: u64) {
        if self.rtt_ms == rtt_ms {
            return;
        }

        self.rtt_estimator.update(self.rtt_ms);
        self.rtt_ms = rtt_ms;
        if rtt_ms <= RTT_THRESHOLD_MS {
            self.parity_ratio = 0.0;
            self.parity_ewma = 0.0;
        } else {
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
            Feedback::Missing(missing_blocks) => {
                // Collect frames from ALL missing blocks
                let mut all_frames = Vec::new();

                for missing_block in missing_blocks.blocks {
                    // Get the block from the ring buffer
                    if let Some(frames_vec) = self.buffer.get(missing_block.block_id) {
                        let frames: Vec<Frame> = missing_block.frames
                            .iter()
                            .filter_map(|&frame_idx| {
                                frames_vec.iter().find(|e| e.frame_idx == frame_idx).map(|e| {
                                    // Convert Arc<box<[u8]>> to Arc<[u8]>
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

                        all_frames.extend(frames);
                    }
                }

                if all_frames.is_empty() {
                    FecAction::Terminated
                } else {
                    FecAction::Retransmit(all_frames)
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

// ===== RECEIVER OPTIMIZATIONS WITH SEQUENCE TRACKING =====

/// SequenceTracker for detecting lost frames
/// Tracks continuous sequence and sent requests to avoid duplicates
#[derive(Debug, Clone)]
struct SequenceTracker {
    /// Largest frame index with continuous sequence from frame 0
    largest_continuous_seq: u32,
    /// Tracks which frames have been requested for retransmit
    sent_seq: Vec<bool>,
    /// Timestamp when largest_continuous_seq was last updated
    seq_update_ts: u64,
}

impl SequenceTracker {
    fn new(total_shards: usize) -> Self {
        Self {
            largest_continuous_seq: 0,
            sent_seq: vec![false; total_shards],
            seq_update_ts: now_micros(),
        }
    }

    /// Update continuous sequence when frame is received
    /// Returns true if sequence advanced
    fn update_continuous_seq(&mut self, received_frames: &[Option<Vec<u8>>]) -> bool {
        let old_seq = self.largest_continuous_seq;

        // Find longest continuous sequence from 0
        let mut seq = self.largest_continuous_seq;
        while (seq as usize) < received_frames.len() && received_frames[seq as usize].is_some() {
            seq += 1;
        }

        self.largest_continuous_seq = seq;

        if seq > old_seq {
            self.seq_update_ts = now_micros();
            true
        } else {
            false
        }
    }

    /// Mark frame as requested (sent_seq = true)
    fn mark_sent(&mut self, frame_idx: u32) {
        if (frame_idx as usize) < self.sent_seq.len() {
            self.sent_seq[frame_idx as usize] = true;
        }
    }

    /// Check if frame was already requested
    fn was_sent(&self, frame_idx: u32) -> bool {
        (frame_idx as usize) < self.sent_seq.len() && self.sent_seq[frame_idx as usize]
    }

    fn detect_lost_frames(
        &self,
        received_frames: &[Option<Vec<u8>>],
    ) -> Vec<u32> {
        let mut lost_frames = Vec::new();

        let contig = self.largest_continuous_seq;

        // Find the largest received frame index
        let mut largest_received = 0u32;
        for (idx, frame) in received_frames.iter().enumerate() {
            if frame.is_some() {
                largest_received = idx as u32;
            }
        }

        // Check for lost frames up to the largest received frame
        let check_up_to = largest_received.max(contig);

        for idx in 0..check_up_to {
            let idx_usize = idx as usize;

            if idx_usize >= received_frames.len() {
                continue;
            }

            if received_frames[idx_usize].is_some() {
                continue;
            }

            if self.was_sent(idx) {
                continue;
            }

            // Detect loss if:
            // 1. Frame is before continuous sequence with threshold, OR
            // 2. Frame is in a gap between continuous sequence and a later received frame
            if idx + PACKET_THRESHOLD < contig ||
               (idx >= contig && idx < largest_received) {
                lost_frames.push(idx);
            }
        }

        lost_frames
    }

    fn reset_sent_seq_range(&mut self, start: u32, end: u32) {
        for idx in start..=end {
            if (idx as usize) < self.sent_seq.len() {
                self.sent_seq[idx as usize] = false;
            }
        }
    }
}

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
    largest_received_idx: u32,
    is_requested_retransmit: bool,
    last_requested_retransmit_frame_idx: i32,
    is_place_holder: bool,
    is_complete: bool,
    // NEW: Sequence tracking for frame loss detection
    seq_tracker: Option<SequenceTracker>,
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
            largest_received_idx: 0,
            is_requested_retransmit: false,
            is_complete: false,
            total_size: 0,
            last_requested_retransmit_frame_idx: -1,
            seq_tracker: None,
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
            largest_received_idx: 0,
            last_requested_retransmit_frame_idx: -1,
            is_complete: false,
            seq_tracker: Some(SequenceTracker::new(total)),
        }
    }

    fn place_value(&mut self, data_shards: usize, parity_shards: usize, total_size: usize) {
        let now = now_micros();
        if self.is_place_holder {
            let total = data_shards + parity_shards;
            self.data_shards = data_shards;
            self.parity_shards = parity_shards;
            self.total_size = total_size;
            self.total_shards = total;
            self.shards = vec![None; self.total_shards];
            self.is_place_holder = false;
            self.received = 0;
            self.first_ts = now;
            self.last_frame_ts = now;
            self.last_ping_ts = now;
            self.largest_received_idx = 0;
            self.is_complete = false;
            self.seq_tracker = Some(SequenceTracker::new(total));
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
            self.shards[idx] = Some(Vec::from(payload));
            self.received += 1;
        }

        let now = now_micros();
        self.last_frame_ts = now;
        self.last_ping_ts = now;
        self.is_complete = self.received >= self.data_shards;

        // Track largest received frame index for gap-based loss detection
        self.largest_received_idx = self.largest_received_idx.max(idx as u32);

        // Update continuous sequence tracker
        if let Some(ref mut tracker) = self.seq_tracker {
            tracker.update_continuous_seq(&self.shards);
        }

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
    rtt_estimator: RttEstimator
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
            rtt_estimator: RttEstimator::new(),
            next_block_id: 0,
            rtt_ms: 50,
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
        self.rtt_estimator.update(rtt_ms * 1000);
    }

    fn calculate_next_check_time(&self, mul: Option<f32>) -> Instant {
        let timeout_ms = if self.rtt_ms > 0 {
            (self.rtt_ms * 2).max(MIN_LOSS_DELAY_US / 1000).min(MAX_BLOCK_TIMEOUT_MS)
        } else {
            MIN_LOSS_DELAY_US / 1000
        };

        let timeout_us = timeout_ms * 1000;
        let now = now_micros();

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
                    remaining.max(MIN_LOSS_DELAY_US)
                );
            }
        }

        Instant::now() + Duration::from_millis((timeout_ms as f32 * mul.unwrap_or(1f32)) as u64)
    }

    /// In case we know when network getting hiccup
    /// we can use this function to make timeout longer
    /// to prevent retransmit-storm
    pub fn hiccup(&mut self) -> Instant {
        self.blocks.entries.iter_mut().filter_map(|it| it.as_mut()).for_each(|it| {
            it.1.last_ping_ts = now_micros();
            it.1.last_frame_ts = now_micros();
        });

        self.calculate_next_check_time(Some(2.0f32))
    }

    pub fn receive(&mut self, frames: Vec<Frame>) -> Result<FecAction, FecError> {
        if frames.is_empty() {
            return self.ping();
        }

        let mut blocks_to_reconstruct = Vec::new();

        for frame in frames {
            if frame.data().len() != CHUNK_SIZE {
                return Err(FecError::InvalidFrameSize {
                    expected: CHUNK_SIZE,
                    actual: frame.data().len(),
                });
            }

            if frame.block_id < self.next_block_id {
                log::info!("Ignoring frame for block {} < {}", frame.block_id, self.next_block_id);
                continue;
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

            if can_decode && !blocks_to_reconstruct.contains(&block_id) {
                blocks_to_reconstruct.push(block_id);
            }
        }

        // Try to reconstruct all blocks that are ready
        for block_id in blocks_to_reconstruct {
            if let Some(block) = self.blocks.get_mut(block_id) {
                let _ = block.try_reconstruct(&mut self.decoders)?;
            }
        }

        // Check if we can emit the next sequential block(s)
        if self.blocks.get(self.next_block_id).map(|b| b.is_constructed()).unwrap_or(false) {
            if let Some(block) = self.blocks.remove(self.next_block_id) {
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

                let next_check = self.calculate_next_check_time(None);
                return Ok(FecAction::Constructed(bytes, next_check));
            }
        }

        self.ping()
    }

    pub fn ping(&mut self) -> Result<FecAction, FecError> {
        let now = now_micros();
        let timeout_us = loss_delay_us(self.rtt_estimator.srtt_us, self.rtt_estimator.rttvar_us);

        let mut all_missing_blocks = Vec::new();
        let mut total_lost = 0usize;

        for entry in self.blocks.entries.iter_mut() {
            if let Some((block_id, block)) = entry.as_mut() {
                if block.is_constructed() || block.is_requested_retransmit {
                    continue;
                }

                let mut missing_frames = Vec::new();
                let present_count = block.shards.iter().filter(|s| s.is_some()).count();
                let needed_more = block.data_shards.saturating_sub(present_count);

                if needed_more == 0 {
                    continue;
                }

                let frame_to = now.saturating_sub(block.last_frame_ts);
                let is_timeout = self.next_block_id >= *block_id && *block_id <= self.next_block_id + 2 && frame_to > (timeout_us * (4f64 * K_TIME_THRESHOLD) as u64);
                let is_ordered = frame_to > timeout_us;
                if is_timeout {
                    block.is_requested_retransmit = true;
                    for i in 0..block.data_shards {
                        if block.shards[i].is_none() {
                            if let Some(ref mut tracker) = &mut block.seq_tracker {
                                tracker.mark_sent(i as u32);
                            }

                            missing_frames.push(i);
                            if missing_frames.len() >= needed_more {
                                break;
                            }
                        }
                    }
                }
                else if is_ordered {
                    if let Some(ref mut tracker) = &mut block.seq_tracker {
                        let gap_lost = tracker.detect_lost_frames(&block.shards);
                        for frame_idx in gap_lost {
                            if !tracker.was_sent(frame_idx) {
                                missing_frames.push(frame_idx as usize);
                                tracker.mark_sent(frame_idx);
                                log::info!(
                                    "Detected lost frame {} in block {} (gap-based, seq={})",
                                    frame_idx,
                                    block_id,
                                    tracker.largest_continuous_seq
                                );
                            }
                        }
                    }
                }

                if !missing_frames.is_empty() {
                    block.last_ping_ts = now;
                    total_lost += missing_frames.len();

                    all_missing_blocks.push(MissingFrames {
                        block_id: *block_id,
                        frames: missing_frames.into_iter().map(|it| it as u32).collect(),
                    });
                }

                self.total_lost_frames += total_lost as u64;
            }
        }

        let next_check = self.calculate_next_check_time(None);

        if !all_missing_blocks.is_empty() {
            return Ok(FecAction::Feedback(FecFeedback {
                feedback: Some(Feedback::Missing(MissingBlocks {
                    blocks: all_missing_blocks,
                })),
            }, next_check));
        }

        let remaining_us = loss_delay_us(self.rtt_estimator.srtt_us, self.rtt_estimator.rttvar_us);
        let next_check = Instant::now() + Duration::from_micros(remaining_us);
        Ok(FecAction::Queued(next_check))
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

// Helper functions
fn now_micros() -> u64 {
    epoch_micro()
}

const ALPHA_NUM: u64 = 1;
const ALPHA_DEN: u64 = 8;
const BETA_NUM: u64 = 1;
const BETA_DEN: u64 = 4;

#[derive(Debug, Clone)]
pub struct RttEstimator {
    pub srtt_us: u64,
    pub rttvar_us: u64,
    initialized: bool,
}

impl RttEstimator {
    pub fn new() -> Self {
        Self {
            srtt_us: 0,
            rttvar_us: 0,
            initialized: false,
        }
    }

    pub fn update(&mut self, latest_rtt_us: u64) {
        if !self.initialized {
            self.srtt_us = latest_rtt_us;
            self.rttvar_us = latest_rtt_us / 2;
            self.initialized = true;
            return;
        }

        let srtt = self.srtt_us as i64;
        let latest = latest_rtt_us as i64;

        let abs_diff = (srtt - latest).abs() as u64;

        self.rttvar_us =
            ((self.rttvar_us * (BETA_DEN - BETA_NUM)) +
                (abs_diff * BETA_NUM)) / BETA_DEN;

        self.srtt_us =
            ((self.srtt_us * (ALPHA_DEN - ALPHA_NUM)) +
                (latest_rtt_us * ALPHA_NUM)) / ALPHA_DEN;
    }
}

/// Calculate QUIC-style loss delay in microseconds
/// Based on QUIC's time threshold mechanism
pub fn loss_delay_us(srtt_us: u64, rttvar_us: u64) -> u64 {
    let base = srtt_us
        + 4 * rttvar_us; // jitter protection

    let delay = (base as f64 * K_TIME_THRESHOLD) as u64;

    delay
        .max(MIN_LOSS_DELAY_US)
        .min(MAX_BLOCK_TIMEOUT_MS * 1000)
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
    fn test_sequence_tracker_continuous() {
        let mut tracker = SequenceTracker::new(32);
        let mut shards: Vec<Option<Vec<u8>>> = vec![None; 32];

        // Receive frames 0, 1, 2
        for i in 0..3 {
            shards[i] = Some(vec![0u8; CHUNK_SIZE]);
            tracker.update_continuous_seq(&shards);
        }

        assert_eq!(tracker.largest_continuous_seq, 3);

        // Now skip frame 4 and receive frame 5
        shards[5] = Some(vec![0u8; CHUNK_SIZE]);
        tracker.update_continuous_seq(&shards);

        // Continuous sequence should still be 3 (gap at 4)
        assert_eq!(tracker.largest_continuous_seq, 3);

        // Detect lost frames in gap
        let lost = tracker.detect_lost_frames(&shards);
        assert!(lost.contains(&4), "Frame 4 should be detected as lost");
    }

    #[test]
    fn test_sequence_tracker_loss_detection() {
        let mut tracker = SequenceTracker::new(32);
        let mut shards: Vec<Option<Vec<u8>>> = vec![None; 32];

        // Receive frames with large gap
        for i in [0, 1, 2, 10, 11, 12] {
            shards[i] = Some(vec![0u8; CHUNK_SIZE]);
        }

        tracker.update_continuous_seq(&shards);

        let lost = tracker.detect_lost_frames(&shards);

        // Should detect gap: continuous_seq=3, largest_received=12, gap=9 > PACKET_THRESHOLD(3)
        assert!(!lost.is_empty(), "Should detect lost frames in gap");

        // Verify lost frames include the gap region
        assert!(lost.iter().any(|&f| f > 2 && f < 10), "Should include frames in gap region");
    }

    #[test]
    fn test_sent_seq_tracking() {
        let mut tracker = SequenceTracker::new(32);

        // Mark some frames as sent
        tracker.mark_sent(5);
        tracker.mark_sent(6);

        assert!(tracker.was_sent(5), "Frame 5 should be marked as sent");
        assert!(tracker.was_sent(6), "Frame 6 should be marked as sent");
        assert!(!tracker.was_sent(7), "Frame 7 should not be marked as sent");

        // Reset sent range
        tracker.reset_sent_seq_range(5, 6);
        assert!(!tracker.was_sent(5), "Frame 5 should be unmarked after reset");
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

        let payload: Arc<[u8]> = Arc::from(vec![0u8; CHUNK_SIZE].into_boxed_slice());
        let frame1 = Frame::new(
            0,      // block_id
            15,     // frame_idx - Late in sequence
            32,     // data_shards
            8,      // parity_shards
            CHUNK_SIZE as u32,
            false,
            payload,
        );

        let result = receiver.receive(vec![frame1]);
        assert!(result.is_ok());
        match result.unwrap() {
            FecAction::Queued(_) => (),
            val => panic!("Should be Queued, not ready to decode {val:?}"),
        }
    }

    #[test]
    fn test_buffer_smart_eviction() {
        let mut buffer: RingBuffer<Vec<FrameEntry>> = RingBuffer::new(10);

        let data = Arc::new([1u8; CHUNK_SIZE]);
        let fe = FrameEntry {
            total_size: CHUNK_SIZE as u32,
            block_id: 0,
            frame_idx: 0,
            data_shards: 2,
            parity_shards: 1,
            is_parity: false,
            data: data.clone(),
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
            data: data.clone(),
            timestamp: now_micros(),
        };

        buffer.insert(1, vec![entry]);

        assert!(buffer.get(0).is_some(), "Block 0 should not be evicted");
    }

    #[test]
    fn test_frame_validation() {
        let mut receiver = FecReceiver::new();

        let payload: Arc<[u8]> = Arc::from(vec![0u8; 1024].into_boxed_slice());
        let bad_frame = Frame::new(
            0,
            0,
            32,
            8,
            1024,
            false,
            payload,
        );

        let result = receiver.receive(vec![bad_frame]);
        assert!(result.is_err(), "Should reject invalid frame size");
    }

    #[test]
    fn test_send_receive_small_data() {
        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 256);
        let mut receiver = FecReceiver::new();

        let original_data: Vec<u8> = (0..100).map(|i| (i % 256) as u8).collect();
        let packet = original_data.clone().into_boxed_slice();

        let action = sender.send(packet).expect("send failed");

        let frames = match action {
            FecAction::Framed(frames) => frames,
            _ => panic!("Expected Framed action"),
        };

        assert!(!frames.is_empty(), "Should have generated frames");

        let result = receiver.receive(frames).expect("receive failed");
        let received_packet = match result {
            FecAction::Constructed(mut packets, _) => {
                assert!(!packets.is_empty(), "Should have constructed packets");
                Some(packets.remove(0))
            }
            _ => None,
        };

        assert!(received_packet.is_some(), "Should have constructed data");
    }
}
