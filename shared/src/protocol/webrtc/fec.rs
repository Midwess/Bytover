use core_services::utils::time::epoch_micro;
use matchbox_protocol::PeerId;
use matchbox_socket::Packet;
use n0_future::time::Instant;
use reed_solomon_erasure::galois_8::ReedSolomon;
use schema::devlog::bitbridge::fec_feedback::Feedback;
use schema::devlog::bitbridge::{FecFeedback, MissingBlocks, MissingFrames};
use std::collections::HashMap;
use std::fmt;
use std::mem::size_of;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

// Too big chunk size will cause higher chance of packet loss
pub const CHUNK_SIZE: usize = 1095;
pub const DATA_SHARDS_DEFAULT: usize = 100;
pub const MIN_PARITY_SHARDS: usize = 2;
pub const MAX_PARITY_SHARDS: usize = 10;
const MAX_BLOCK_TIMEOUT_MS: u64 = 1200;
const RTT_THRESHOLD_MS: u64 = 250;

const K_TIME_THRESHOLD: f64 = 9.0 / 8.0;
const MIN_LOSS_DELAY_US: u64 = 50 * 1_000;
const QUICK_LOSS_THRESHOLD: usize = 5;

#[derive(Debug, Error)]
pub enum FecError {
    #[error("reed-solomon encoding/decoding error {0:?}")]
    ReedSolomon(reed_solomon_erasure::Error),
    #[error("invalid frame size: expected {expected}, got {actual}")]
    InvalidFrameSize { expected: usize, actual: usize },
    #[error("invalid frame index {idx} for block with {total_shards} shards")]
    InvalidFrameIndex { idx: u8, total_shards: usize },
    #[error("block id mismatch or wraparound detected")]
    BlockIdMismatch,
    #[error("generic error")]
    Generic
}

impl From<reed_solomon_erasure::Error> for FecError {
    fn from(e: reed_solomon_erasure::Error) -> Self {
        Self::ReedSolomon(e)
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct FrameEntry {
    pub block_id: u32,
    pub total_size: u32,
    pub frame_idx: u8,
    pub data_shards: u8,
    pub parity_shards: u8,
    pub is_parity: bool,
    pub prefix: u16,
    pub data: Arc<[u8]>,
    pub timestamp: u64
}

#[derive(Clone)]
pub struct Frame {
    pub block_id: u32,
    pub frame_idx: u8,
    pub data_shards: u8,
    pub parity_shards: u8,
    pub total_size: u32,
    pub is_parity: bool,
    pub prefix: u16,

    buffer: Arc<[u8]>,
    payload_offset: usize
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
            .field("prefix", &self.prefix)
            .field("payload_len", &self.data().len())
            .finish()
    }
}

impl Frame {
    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.buffer[self.payload_offset..]
    }

    /// Create a new Frame with just payload (for sender)
    pub fn new(
        block_id: u32,
        frame_idx: u8,
        data_shards: u8,
        parity_shards: u8,
        total_size: u32,
        is_parity: bool,
        prefix: u16,
        payload: Arc<[u8]>
    ) -> Self {
        Self {
            block_id,
            frame_idx,
            data_shards,
            parity_shards,
            total_size,
            is_parity,
            prefix,
            buffer: payload,
            payload_offset: 0
        }
    }

    pub fn serialize(&self) -> Box<[u8]> {
        let payload = self.data();
        let header_len = size_of::<u32>() * 2 + 6;
        let total_len = header_len + payload.len();
        let mut buf = Vec::with_capacity(total_len);

        unsafe {
            let ptr = buf.as_mut_ptr();
            std::ptr::copy_nonoverlapping(&self.block_id as *const u32 as *const u8, ptr, 4);
            std::ptr::copy_nonoverlapping(&self.total_size as *const u32 as *const u8, ptr.add(4), 4);
            ptr.add(8).write(self.frame_idx);
            ptr.add(9).write(self.data_shards);
            ptr.add(10).write(self.parity_shards);
            ptr.add(11).write(self.is_parity as u8);
            std::ptr::copy_nonoverlapping(&self.prefix as *const u16 as *const u8, ptr.add(12), 2);
            std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.add(14), payload.len());
            buf.set_len(total_len);
        }

        buf.into_boxed_slice()
    }

    pub fn deserialize(buf: &[u8]) -> Option<Self> {
        let mut offset = 0;
        macro_rules! read {
            ($ty:ty) => {{
                if offset + size_of::<$ty>() > buf.len() {
                    return None;
                }
                let bytes = &buf[offset..offset + size_of::<$ty>()];
                offset += size_of::<$ty>();
                <$ty>::from_le_bytes(bytes.try_into().ok()?)
            }};
        }

        let block_id: u32 = read!(u32);
        let total_size: u32 = read!(u32);

        if offset + 6 > buf.len() {
            return None;
        } // u8*4 + u16 = 6 bytes
        let frame_idx = buf[offset];
        offset += 1;
        let data_shards = buf[offset];
        offset += 1;
        let parity_shards = buf[offset];
        offset += 1;
        let is_parity = buf[offset] != 0;
        offset += 1;
        let prefix: u16 = read!(u16);

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
            prefix,
            buffer,
            payload_offset
        })
    }
}

#[allow(dead_code)]
impl FrameEntry {
    pub fn serialize(&self) -> Box<[u8]> {
        let header_len = size_of::<u32>() * 2 + size_of::<u64>() + size_of::<u16>() + 4;
        let total_len = header_len + self.data.len();
        let mut buf = Vec::with_capacity(total_len);

        unsafe {
            let ptr = buf.as_mut_ptr();
            std::ptr::copy_nonoverlapping(&self.block_id as *const u32 as *const u8, ptr, 4);
            std::ptr::copy_nonoverlapping(&self.total_size as *const u32 as *const u8, ptr.add(4), 4);
            ptr.add(8).write(self.frame_idx);
            ptr.add(9).write(self.data_shards);
            ptr.add(10).write(self.parity_shards);
            ptr.add(11).write(self.is_parity as u8);
            std::ptr::copy_nonoverlapping(&self.prefix as *const u16 as *const u8, ptr.add(12), 2);
            std::ptr::copy_nonoverlapping(&self.timestamp as *const u64 as *const u8, ptr.add(14), 8);
            std::ptr::copy_nonoverlapping(self.data.as_ptr(), ptr.add(22), self.data.len());
            buf.set_len(total_len);
        }

        buf.into_boxed_slice()
    }

    pub fn deserialize(buf: &[u8]) -> Option<Self> {
        let mut offset = 0;
        macro_rules! read {
            ($ty:ty) => {{
                if offset + size_of::<$ty>() > buf.len() {
                    return None;
                }
                let bytes = &buf[offset..offset + size_of::<$ty>()];
                offset += size_of::<$ty>();
                <$ty>::from_le_bytes(bytes.try_into().ok()?)
            }};
        }

        let block_id: u32 = read!(u32);
        let total_size: u32 = read!(u32);

        if offset + 4 > buf.len() {
            return None;
        }
        let frame_idx = buf[offset];
        offset += 1;
        let data_shards = buf[offset];
        offset += 1;
        let parity_shards = buf[offset];
        offset += 1;
        let is_parity = buf[offset] != 0;
        offset += 1;

        let prefix: u16 = read!(u16);
        let timestamp: u64 = read!(u64);
        let data: Arc<[u8]> = buf[offset..].to_vec().into();

        Some(Self {
            block_id,
            total_size,
            frame_idx,
            data_shards,
            parity_shards,
            is_parity,
            prefix,
            data,
            timestamp
        })
    }
}

/// RingBuffer using Vec and modulo arithmetic for O(1) indexing
/// block_id maps to index (block_id % window_size)
#[allow(dead_code)]
#[derive(Debug)]
pub struct RingBuffer<T> {
    entries: Vec<Option<(u32, T)>>,
    window_size: usize,
    window_mask: usize
}

impl<T> RingBuffer<T> {
    pub fn new(window_size: usize) -> Self {
        assert!(window_size.is_power_of_two(), "window_size must be power of 2");
        Self {
            entries: (0..window_size).map(|_| None).collect(),
            window_size,
            window_mask: window_size - 1
        }
    }

    #[inline]
    pub fn insert(&mut self, block_id: u32, data: T) -> Option<(u32, T)> {
        let idx = (block_id as usize) & self.window_mask;
        self.entries[idx].replace((block_id, data))
    }

    #[inline]
    pub fn get(&self, block_id: u32) -> Option<&T> {
        let idx = (block_id as usize) & self.window_mask;
        match &self.entries[idx] {
            Some((id, data)) if *id == block_id => Some(data),
            _ => None
        }
    }

    #[inline]
    pub fn get_mut(&mut self, block_id: u32) -> Option<&mut T> {
        let idx = (block_id as usize) & self.window_mask;
        match &mut self.entries[idx] {
            Some((id, data)) if *id == block_id => Some(data),
            _ => None
        }
    }

    #[inline]
    pub fn remove(&mut self, block_id: u32) -> Option<T> {
        let idx = (block_id as usize) & self.window_mask;
        match &self.entries[idx] {
            Some((id, _)) if *id == block_id => self.entries[idx].take().map(|(_, data)| data),
            _ => None
        }
    }

    #[inline]
    pub fn contains_key(&self, block_id: u32) -> bool {
        let idx = (block_id as usize) & self.window_mask;
        match &self.entries[idx] {
            Some((id, _)) => *id == block_id,
            None => false
        }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }
}

#[allow(dead_code)]
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
    shard_buffer_pool: Vec<Vec<u8>>
}

impl FecSender {
    pub fn new(peer_id: PeerId, window_size: usize) -> Self {
        let initial_ratio = 0.0;
        let buffer_pool_size = DATA_SHARDS_DEFAULT + MAX_PARITY_SHARDS;
        let shard_buffer_pool = (0..buffer_pool_size).map(|_| Vec::with_capacity(CHUNK_SIZE)).collect();

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
            shard_buffer_pool
        }
    }

    #[inline]
    pub fn rtt(&self) -> u64 {
        self.rtt_ms
    }

    #[inline]
    pub fn set_rtt(&mut self, rtt_ms: u64) {
        if self.rtt_ms == rtt_ms {
            return;
        }

        self.rtt_ms = rtt_ms;
        self.rtt_estimator.update(self.rtt_ms * 1000);
        if rtt_ms <= RTT_THRESHOLD_MS {
            self.parity_ratio = 0.0;
            self.parity_ewma = 0.0;
        } else if self.parity_ratio == 0.0 {
            self.parity_ratio = 0.03;
            self.parity_ewma = 0.03;
        }
    }

    pub fn send(&mut self, prefix: u16, packet: Box<[u8]>) -> Result<FecAction, FecError> {
        let mut offset = 0usize;
        let estimated_frames =
            (packet.len() / (CHUNK_SIZE * self.data_shards)).saturating_add(1) * (self.data_shards + MAX_PARITY_SHARDS);
        let mut frames_to_send: Vec<Frame> = Vec::with_capacity(estimated_frames);

        while offset < packet.len() {
            let block_size = (packet.len() - offset).min(CHUNK_SIZE * self.data_shards);
            let shard_count = block_size.div_ceil(CHUNK_SIZE);
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
                    let mut shards_refs: Vec<&mut [u8]> = shards.iter_mut().map(|v| v.as_mut_slice()).collect();

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
                    i as u8,
                    shard_count as u8,
                    parity_shards as u8,
                    block_size as u32,
                    is_parity,
                    prefix,
                    payload.clone()
                );

                let fe = FrameEntry {
                    total_size: frame.total_size,
                    block_id: frame.block_id,
                    frame_idx: frame.frame_idx,
                    data_shards: frame.data_shards,
                    parity_shards: frame.parity_shards,
                    is_parity: frame.is_parity,
                    prefix: frame.prefix,
                    data: payload,
                    timestamp: now_micros()
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
                let mut all_frames = Vec::new();

                for missing_block in missing_blocks.blocks {
                    // Ignore retransmit block that we did not sent yet
                    if missing_block.block_id > self.block_id {
                        continue;
                    }

                    if let Some(frames_vec) = self.buffer.get(missing_block.block_id) {
                        let frames: Vec<Frame> = missing_block
                            .frames
                            .iter()
                            .filter_map(|&frame_idx| {
                                let frame_idx_u8 = frame_idx as u8;
                                frames_vec.iter().find(|e| e.frame_idx == frame_idx_u8).map(|e| {
                                    let payload: Arc<[u8]> = Arc::from(e.data.as_ref());
                                    Frame::new(
                                        e.block_id,
                                        e.frame_idx,
                                        e.data_shards,
                                        e.parity_shards,
                                        e.total_size,
                                        e.is_parity,
                                        e.prefix,
                                        payload
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
        }
    }

    fn parity_count_from_ratio(packet_loss: f32, data_shards: usize) -> usize {
        if packet_loss == 0.0 {
            return 0;
        }

        let p_success = 1.0 - packet_loss;
        let q = 1.0 - (p_success * p_success);
        let p = (q * data_shards as f32).round() as usize;

        p.clamp(MIN_PARITY_SHARDS, MAX_PARITY_SHARDS)
    }
}

#[derive(Debug, Clone)]
struct LossDetector {
    requested_frames: Vec<bool>
}

impl LossDetector {
    fn new(total_shards: usize) -> Self {
        Self {
            requested_frames: vec![false; total_shards]
        }
    }

    fn mark_requested(&mut self, frame_idx: u8) {
        if (frame_idx as usize) < self.requested_frames.len() {
            self.requested_frames[frame_idx as usize] = true;
        }
    }

    fn is_requested(&self, frame_idx: u8) -> bool {
        (frame_idx as usize) < self.requested_frames.len() && self.requested_frames[frame_idx as usize]
    }

    fn detect_quick_loss(&self, received_frames: &[Option<Vec<u8>>], threshold: usize) -> Vec<u8> {
        let mut lost = Vec::new();
        let mut i = 0;

        while i < received_frames.len() {
            while i < received_frames.len() && received_frames[i].is_some() {
                i += 1;
            }

            if i >= received_frames.len() {
                break;
            }

            let gap_start = i;
            while i < received_frames.len() && received_frames[i].is_none() && !self.is_requested(i as u8) {
                i += 1;
            }

            while i < received_frames.len() && (received_frames[i].is_none() && self.is_requested(i as u8)) {
                i += 1;
            }

            if gap_start == i {
                continue;
            }

            let gap_end = i - 1;
            let gap_size = gap_end - gap_start + 1;

            let continuous_start = i;
            while i < received_frames.len() && received_frames[i].is_some() {
                i += 1;
            }
            let continuous_count = i - continuous_start;

            if continuous_count >= threshold + gap_size {
                for idx in gap_start..=gap_end {
                    if !self.is_requested(idx as u8) {
                        lost.push(idx as u8);
                    }
                }
            }
        }

        lost
    }

    fn detect_lost_frames(
        &self,
        received_frames: &[Option<Vec<u8>>],
        since: u64,
        now: u64,
        time_threshold_us: u64,
        quick_loss_threshold: usize
    ) -> Vec<u8> {
        if since.saturating_add(time_threshold_us) > now {
            let time_lost: Vec<u8> = received_frames
                .iter()
                .enumerate()
                .filter_map(|(idx, frame)| {
                    if self.is_requested(idx as u8) {
                        return None;
                    }

                    if frame.is_none() {
                        return Some(idx as u8);
                    }

                    None
                })
                .collect();

            return time_lost
        }

        if since.saturating_add(time_threshold_us / 2) > now {
            return self.detect_quick_loss(received_frames, quick_loss_threshold);
        }

        Vec::new()
    }
}

#[derive(Default)]
struct ReceiverBlock {
    data_shards: usize,
    total_size: usize,
    parity_shards: usize,
    total_shards: usize,
    shards: Vec<Option<Vec<u8>>>,
    frame_send_times: Vec<u64>,
    received: usize,
    first_ts: u64,
    last_frame_ts: u64,
    last_ping_ts: u64,
    is_complete: bool,
    is_placeholder: bool,
    prefix: u16,

    loss_detector: Option<LossDetector>
}

impl ReceiverBlock {
    fn placeholder() -> Self {
        let now = now_micros();
        Self {
            is_placeholder: true,
            data_shards: DATA_SHARDS_DEFAULT,
            shards: vec![None; DATA_SHARDS_DEFAULT + MIN_PARITY_SHARDS],
            frame_send_times: vec![0; DATA_SHARDS_DEFAULT + MIN_PARITY_SHARDS],
            parity_shards: MIN_PARITY_SHARDS,
            total_shards: DATA_SHARDS_DEFAULT + MIN_PARITY_SHARDS,
            received: 0,
            first_ts: now,
            last_frame_ts: now,
            last_ping_ts: now,
            is_complete: false,
            total_size: 0,
            prefix: 0,
            loss_detector: None
        }
    }

    fn new(data_shards: usize, parity_shards: usize, total_size: usize, prefix: u16) -> Self {
        let total = data_shards + parity_shards;
        let now = now_micros();
        Self {
            is_placeholder: false,
            data_shards,
            total_size,
            parity_shards,
            total_shards: total,
            shards: vec![None; total],
            frame_send_times: vec![0; total],
            received: 0,
            first_ts: now,
            last_frame_ts: now,
            last_ping_ts: now,
            is_complete: false,
            prefix,
            loss_detector: Some(LossDetector::new(total))
        }
    }

    fn place_value(&mut self, data_shards: usize, parity_shards: usize, total_size: usize, prefix: u16) {
        let now = now_micros();
        if self.is_placeholder {
            let total = data_shards + parity_shards;
            self.data_shards = data_shards;
            self.parity_shards = parity_shards;
            self.total_size = total_size;
            self.total_shards = total;
            self.shards = vec![None; total];
            self.frame_send_times = vec![0; total];
            self.is_placeholder = false;
            self.received = 0;
            self.first_ts = now;
            self.last_frame_ts = now;
            self.prefix = prefix;
            self.last_ping_ts = now;
            self.is_complete = false;
            self.loss_detector = Some(LossDetector::new(total));
        }
    }

    fn insert_frame(&mut self, idx: usize, payload: Box<[u8]>, false_retransmit_counter: &mut u64) -> Result<bool, FecError> {
        if idx >= self.total_shards {
            return Err(FecError::InvalidFrameIndex {
                idx: idx as u8,
                total_shards: self.total_shards
            });
        }

        if self.shards[idx].is_none() {
            self.shards[idx] = Some(Vec::from(payload));
            self.frame_send_times[idx] = now_micros(); // Record when frame arrived
            self.received += 1;
        } else {
            *false_retransmit_counter = false_retransmit_counter.saturating_add(1);
        }

        let now = now_micros();
        self.last_frame_ts = now;
        self.last_ping_ts = now;
        self.is_complete = self.received >= self.data_shards;

        Ok(self.is_complete)
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
            let all_data_present = self.shards.iter().take(self.data_shards).all(|s| s.is_some());

            if all_data_present {
                self.is_complete = true;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
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

    #[inline]
    fn is_constructed(&self) -> bool {
        self.is_complete
    }

    fn into_packet(mut self) -> (u16, Packet) {
        let mut bytes = Vec::with_capacity(self.total_size);
        unsafe {
            bytes.set_len(self.total_size);
        }
        let dst: &mut [u8] = bytes.as_mut_slice();
        let mut written = 0;

        for shard_opt in self.shards.iter_mut().take(self.data_shards) {
            if written >= self.total_size {
                break;
            }

            if let Some(shard) = shard_opt.take() {
                let remaining = self.total_size - written;
                let to_write = remaining.min(shard.len());

                unsafe {
                    std::ptr::copy_nonoverlapping(shard.as_ptr(), dst.as_mut_ptr().add(written), to_write);
                }
                written += to_write;
            }
        }

        (self.prefix, Packet::from(bytes.into_boxed_slice()))
    }
}

pub struct FecReceiver {
    blocks: RingBuffer<ReceiverBlock>,
    false_retransmit: u64,
    retransmit_count: u64,
    next_block_id: u32,
    total_frames_received: u64,
    total_lost_frames: u64,
    decoders: HashMap<(usize, usize), ReedSolomon>,
    block_pool: Vec<ReceiverBlock>,
    rtt_estimator: RttEstimator,
    blocks_to_reconstruct_buf: Vec<u32>
}

impl FecReceiver {
    pub fn new() -> Self {
        Self::with_window_size(256)
    }

    pub fn with_window_size(window_size: usize) -> Self {
        let block_pool = (0..16).map(|_| ReceiverBlock::placeholder()).collect();

        Self {
            blocks: RingBuffer::new(window_size),
            false_retransmit: 0,
            retransmit_count: 0,
            rtt_estimator: RttEstimator::new(),
            next_block_id: 0,
            total_frames_received: 0,
            total_lost_frames: 0,
            decoders: HashMap::new(),
            block_pool,
            blocks_to_reconstruct_buf: Vec::new()
        }
    }

    #[inline]
    pub fn calculate_loss_rate(&self) -> f32 {
        if self.total_frames_received == 0 {
            return 0.0;
        }
        (self.total_lost_frames as f32) / (self.total_frames_received as f32)
    }

    #[inline]
    pub fn current_block_id(&self) -> u32 {
        self.next_block_id
    }

    #[inline]
    pub fn set_rtt(&mut self, rtt_ms: u64) {
        let rtt_ms = rtt_ms.max(MIN_LOSS_DELAY_US / 1000);
        self.rtt_estimator.update(rtt_ms * 1000);
    }

    pub fn rtt(&mut self) -> u64 {
        self.rtt_estimator.srtt_us / 1000
    }

    pub fn calculate_next_check_time(&self) -> Instant {
        let timeout_us = loss_delay_us(self.rtt_estimator.srtt_us, self.rtt_estimator.rttvar_us, None).min(100_000) / 2;

        Instant::now() + Duration::from_micros(timeout_us)
    }

    pub fn receive(&mut self, frames: Vec<Frame>) -> Result<FecAction, FecError> {
        if frames.is_empty() {
            return self.ping();
        }

        self.blocks_to_reconstruct_buf.clear();

        for frame in frames {
            if frame.data().len() != CHUNK_SIZE {
                return Err(FecError::InvalidFrameSize {
                    expected: CHUNK_SIZE,
                    actual: frame.data().len()
                });
            }

            if frame.block_id < self.next_block_id {
                log::warn!(
                    "Received frame for old block {} (current block is {}) \
                    false_retransmit = {}, total_retransmit = {}, srtt = {}, rttvar = {}",
                    frame.block_id,
                    self.next_block_id,
                    self.false_retransmit,
                    self.retransmit_count,
                    self.rtt_estimator.srtt_us,
                    self.rtt_estimator.rttvar_us
                );
                self.false_retransmit = self.false_retransmit.saturating_add(1);
                continue;
            }

            let block_id = frame.block_id;

            // Get or create block
            if self.blocks.get(block_id).is_none() {
                let new_block = if let Some(mut pooled) = self.block_pool.pop() {
                    pooled.place_value(
                        frame.data_shards as usize,
                        frame.parity_shards as usize,
                        frame.total_size as usize,
                        frame.prefix
                    );

                    pooled
                } else {
                    ReceiverBlock::new(
                        frame.data_shards as usize,
                        frame.parity_shards as usize,
                        frame.total_size as usize,
                        frame.prefix
                    )
                };

                if let Some(replaced) = self.blocks.insert(block_id, new_block) {
                    let (evicted_id, evicted_block) = replaced;

                    if evicted_block.is_constructed() {
                        log::error!(
                            "CRITICAL: Evicted COMPLETED block {} (was waiting to be emitted). Current block_id={}, next_block_id={}",
                            evicted_id,
                            block_id,
                            self.next_block_id
                        );
                    } else {
                        log::warn!(
                            "Buffer full, evicted IN-PROGRESS block {} by {}. Received={}/{} shards, next_block_id={}",
                            evicted_id,
                            block_id,
                            evicted_block.received,
                            evicted_block.data_shards,
                            self.next_block_id
                        );
                    }

                    return Ok(FecAction::Terminated);
                }
            }

            let block = self.blocks.get_mut(block_id).unwrap();
            block.place_value(
                frame.data_shards as usize,
                frame.parity_shards as usize,
                frame.total_size as usize,
                frame.prefix
            );

            let idx = frame.frame_idx as usize;
            let payload_box = Box::from(frame.data());
            let can_decode = block.insert_frame(idx, payload_box, &mut self.false_retransmit)?;
            self.total_frames_received += 1;

            if can_decode && !self.blocks_to_reconstruct_buf.contains(&block_id) {
                self.blocks_to_reconstruct_buf.push(block_id);
            }
        }

        let blocks_to_reconstruct = &self.blocks_to_reconstruct_buf;
        for &block_id in blocks_to_reconstruct {
            if let Some(block) = self.blocks.get_mut(block_id) {
                let _ = block.try_reconstruct(&mut self.decoders)?;
            }
        }

        // Emit completed blocks
        if self.blocks.get(self.next_block_id).map(|b| b.is_constructed()).unwrap_or(false) {
            if let Some(block) = self.blocks.remove(self.next_block_id) {
                log::debug!(
                    "Block {} constructed with pefix {}, size {} bytes",
                    self.next_block_id,
                    block.prefix,
                    block.total_size
                );
                let mut completed_blocks = vec![block];

                loop {
                    self.next_block_id += 1;

                    if !self.blocks.contains_key(self.next_block_id) {
                        let placeholder = if let Some(pooled) = self.block_pool.pop() {
                            pooled
                        } else {
                            ReceiverBlock::placeholder()
                        };

                        if let Some(evicted) = self.blocks.insert(self.next_block_id, placeholder) {
                            let (evicted_id, evicted_block) = evicted;

                            if evicted_block.is_constructed() {
                                log::error!(
                                    "CRITICAL: Placeholder evicted COMPLETED block {} (next_block_id={}, received={}/{})",
                                    evicted_id,
                                    self.next_block_id,
                                    evicted_block.received,
                                    evicted_block.data_shards
                                );

                                return Ok(FecAction::Terminated);
                            }

                            log::warn!(
                                "Placeholder evicted IN-PROGRESS block {} (next_block_id={}, received={}/{})",
                                evicted_id,
                                self.next_block_id,
                                evicted_block.received,
                                evicted_block.data_shards
                            );
                        }

                        break;
                    }

                    let is_completed = self.blocks.get(self.next_block_id).map(|b| b.is_constructed()).unwrap_or_default();

                    if is_completed {
                        let block = self.blocks.remove(self.next_block_id).unwrap();
                        log::debug!(
                            "Block {} constructed with prefix {}, size {} bytes",
                            self.next_block_id,
                            block.prefix,
                            block.total_size
                        );
                        completed_blocks.push(block);
                    } else {
                        break;
                    }
                }

                let bytes: Vec<(u16, Packet)> = completed_blocks.into_iter().map(|b| b.into_packet()).collect();
                log::debug!("Emitting {} reconstructed packets from consecutive blocks", bytes.len());
                let next_check = self.calculate_next_check_time();
                return Ok(FecAction::Constructed(bytes, next_check));
            }
        }

        self.ping()
    }

    pub fn ping(&mut self) -> Result<FecAction, FecError> {
        let now = now_micros();
        let ratio = if self.retransmit_count > 0 {
            (self.retransmit_count as f64 + (self.false_retransmit as f64 * 3.0)) / self.retransmit_count as f64
        } else {
            1.0
        };

        let time_threshold_us = loss_delay_us(self.rtt_estimator.srtt_us, self.rtt_estimator.rttvar_us, Some(ratio));
        let mut all_missing_blocks = Vec::new();

        for entry in self.blocks.entries.iter_mut() {
            if let Some((block_id, block)) = entry.as_mut() {
                if *block_id > self.next_block_id {
                    continue;
                }

                if block.is_complete {
                    continue;
                }

                let present_count = block.shards.iter().filter(|s| s.is_some()).count();
                if present_count >= block.data_shards {
                    continue; // Can reconstruct
                }

                if let Some(ref mut detector) = &mut block.loss_detector {
                    let lost_frames = detector.detect_lost_frames(
                        &block.shards,
                        block.last_ping_ts,
                        now,
                        time_threshold_us,
                        (ratio * QUICK_LOSS_THRESHOLD as f64) as usize
                    );

                    self.retransmit_count += lost_frames.len() as u64;
                    if !lost_frames.is_empty() {
                        for &frame_idx in &lost_frames {
                            detector.mark_requested(frame_idx);
                            self.total_lost_frames += 1;
                        }

                        block.last_ping_ts = now;

                        all_missing_blocks.push(MissingFrames {
                            block_id: *block_id,
                            frames: lost_frames.iter().map(|&f| f as u32).collect()
                        });

                        log::info!(
                            "Loss detection for block {}: {} frames lost",
                            block_id,
                            all_missing_blocks.last().unwrap().frames.len()
                        );
                    }
                }
            }
        }

        let next_check = self.calculate_next_check_time();

        if !all_missing_blocks.is_empty() {
            return Ok(FecAction::Feedback(
                FecFeedback {
                    feedback: Some(Feedback::Missing(MissingBlocks {
                        blocks: all_missing_blocks
                    }))
                },
                next_check
            ));
        }

        Ok(FecAction::Queued(next_check))
    }
}

#[derive(Debug)]
pub enum FecAction {
    Framed(Vec<Frame>),
    Constructed(Vec<(u16, Packet)>, Instant),
    Retransmit(Vec<Frame>),
    Feedback(FecFeedback, Instant),
    Noop,
    Queued(Instant),
    Terminated
}

#[inline]
fn now_micros() -> u64 {
    epoch_micro()
}

const ALPHA_NUM: u64 = 1;
const ALPHA_DEN: u64 = 8;
const BETA_NUM: u64 = 1;
const BETA_DEN: u64 = 4;
const K: u64 = 4;

const MIN_RTTVAR_US: u64 = 5_000;      // 5ms minimum
const CLOCK_GRANULARITY_US: u64 = 10_000; // 10ms
const MIN_RTO_US: u64 = 200_000;       // 200ms
const MAX_RTO_US: u64 = 60_000_000;    // 60s
const DEFAULT_RTO_US: u64 = 1_000_000; // 1s

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

    #[inline]
    pub fn update(&mut self, latest_rtt_us: u64) {
        if !self.initialized {
            self.srtt_us = latest_rtt_us;
            self.rttvar_us = (latest_rtt_us / 2).max(MIN_RTTVAR_US);
            self.initialized = true;
            return;
        }

        let srtt = self.srtt_us as i64;
        let latest = latest_rtt_us as i64;
        let abs_diff = (srtt - latest).unsigned_abs();

        // Update RTTVAR with minimum floor
        self.rttvar_us = ((self.rttvar_us * (BETA_DEN - BETA_NUM))
                         + (abs_diff * BETA_NUM)) / BETA_DEN;
        self.rttvar_us = self.rttvar_us.max(MIN_RTTVAR_US);

        // Update SRTT
        self.srtt_us = ((self.srtt_us * (ALPHA_DEN - ALPHA_NUM))
                       + (latest_rtt_us * ALPHA_NUM)) / ALPHA_DEN;
    }

    #[inline]
    pub fn rto_us(&self) -> u64 {
        if !self.initialized {
            return DEFAULT_RTO_US;
        }

        // RFC 6298: RTO = SRTT + max(G, K * RTTVAR)
        let variance_term = (K * self.rttvar_us).max(CLOCK_GRANULARITY_US);
        let rto = self.srtt_us.saturating_add(variance_term);

        rto.clamp(MIN_RTO_US, MAX_RTO_US)
    }

    pub fn reset(&mut self) {
        self.srtt_us = 0;
        self.rttvar_us = 0;
        self.initialized = false;
    }
}

pub fn loss_delay_us(srtt_us: u64, rttvar_us: u64, mul: Option<f64>) -> u64 {
    let srtt_clamped = srtt_us.max(1);
    let rttvar_clamped = rttvar_us.max(1);
    let base = srtt_clamped + (rttvar_clamped << 2);

    let delay = ((base as f64) * K_TIME_THRESHOLD) as u64;
    delay.clamp(MIN_LOSS_DELAY_US, MAX_BLOCK_TIMEOUT_MS * 1000) * mul.unwrap_or(1.0).min(5.0) as u64
}
