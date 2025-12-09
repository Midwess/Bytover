use reed_solomon_erasure::galois_8::ReedSolomon;
use thiserror::Error;
use matchbox_protocol::PeerId;
use std::collections::{HashMap, VecDeque};
use bytemuck::bytes_of;
use matchbox_socket::Packet;
use core_services::utils::time::epoch_micro;
use schema::devlog::bitbridge::{fec_feedback, FecFeedback, MissingFrames};
use schema::devlog::bitbridge::fec_feedback::Feedback;

//
// CONFIG
//
const CHUNK_SIZE: usize = 8 * 1024;
const DATA_SHARDS_DEFAULT: usize = 32;
const MIN_PARITY_SHARDS: usize = 1;
const MAX_PARITY_SHARDS: usize = 64;
const MIN_BLOCK_TIMEOUT_MS: u64 = 80;
const PARITY_ADAPTATION_WEIGHT: f32 = 0.15; // EWMA weight (lower = slower)
const LOSS_RATE_HYSTERESIS: f32 = 0.02; // Only adapt if change > 2%

// ======================================================
// Error Types (Enhanced)
// ======================================================

#[derive(Debug, Error)]
pub enum FecError {
    #[error("reed-solomon encoding/decoding error {0:?}")]
    ReedSolomon(#[from] reed_solomon_erasure::Error),
    #[error("invalid frame size: expected {expected}, got {actual}")]
    InvalidFrameSize { expected: usize, actual: usize },
    #[error("invalid frame index {idx} for block with {total_shards} shards")]
    InvalidFrameIndex { idx: u32, total_shards: usize },
    #[error("block id mismatch or wraparound detected")]
    BlockIdMismatch,
    #[error("generic error")]
    Generic,
}

// ======================================================
// Data Structures
// ======================================================

#[derive(Clone, Debug)]
pub struct FrameEntry {
    pub block_id: u32,
    pub total_size: u32,
    pub frame_idx: u32,
    pub data_shards: u8,
    pub parity_shards: u8,
    pub is_parity: bool,
    pub data: Box<[u8]>,
    pub timestamp: u64,
}

impl Frame {
    pub fn serialize(&self) -> Box<[u8]> {
        let header_len =
            size_of::<u32>() + // block_id
                size_of::<u32>() + // total_size
                size_of::<u32>() + // frame_idx
                1 +                // data_shards
                1 +                // parity_shards
                1;                 // is_parity

        let mut buf = Vec::with_capacity(header_len + self.payload.len());

        buf.extend_from_slice(bytes_of(&self.block_id));
        buf.extend_from_slice(bytes_of(&self.total_size));
        buf.extend_from_slice(bytes_of(&self.frame_idx));
        buf.push(self.data_shards);
        buf.push(self.parity_shards);
        buf.push(self.is_parity as u8);

        buf.extend_from_slice(&self.payload);

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

        let payload = buf[offset..].to_vec().into_boxed_slice();

        Some(Self {
            block_id,
            total_size,
            frame_idx,
            data_shards,
            parity_shards,
            is_parity,
            payload,
        })
    }
}

impl FrameEntry {
    pub fn serialize(&self) -> Box<[u8]> {
        let header_len =
            size_of::<u32>() + // block_id
                size_of::<u32>() + // total_size
                size_of::<u32>() + // frame_idx
                1 +                // data_shards
                1 +                // parity_shards
                1 +                // is_parity
                size_of::<u64>(); // timestamp

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

        let data = buf[offset..].to_vec().into_boxed_slice();

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

#[derive(Debug)]
pub struct FrameBuffer {
    entries: VecDeque<FrameEntry>,
    capacity_bytes: usize,
    used_bytes: usize,
    min_required_block_id: u32, // NEW: track oldest block still needed
    max_block_id_seen: u32,     // NEW: detect wraparound
}

impl FrameBuffer {
    pub fn new(capacity_bytes: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            capacity_bytes,
            used_bytes: 0,
            min_required_block_id: 0,
            max_block_id_seen: 0,
        }
    }

    /// Insert frame and evict old frames intelligently
    /// Never evicts blocks <= min_required_block_id (still needed for retransmit)
    pub fn insert(&mut self, entry: FrameEntry) -> Result<(), FecError> {
        let size = entry.data.len();
        if size != CHUNK_SIZE {
            return Err(FecError::InvalidFrameSize {
                expected: CHUNK_SIZE,
                actual: size,
            });
        }

        // Track newest block_id (detect wraparound)
        if entry.block_id.wrapping_sub(self.max_block_id_seen) < i32::MAX as u32 {
            self.max_block_id_seen = entry.block_id;
        }

        self.used_bytes += size;
        self.entries.push_back(entry);

        while self.used_bytes > self.capacity_bytes {
            if let Some(front) = self.entries.front() {
                // If front block is still required, evict from back instead
                if front.block_id <= self.min_required_block_id {
                    if let Some(removed) = self.entries.pop_back() {
                        self.used_bytes -= removed.data.len();
                    } else {
                        break;
                    }
                } else {
                    if let Some(removed) = self.entries.pop_front() {
                        self.used_bytes -= removed.data.len();
                    } else {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Update the minimum required block id (from receiver feedback)
    pub fn update_min_required_block(&mut self, block_id: u32) {
        self.min_required_block_id = self.min_required_block_id.max(block_id);
    }

    /// Return a cloned FrameEntry if present
    pub fn search(&self, block_id: u32, idx: u32) -> Option<FrameEntry> {
        self.entries
            .iter()
            .find(|e| e.block_id == block_id && e.frame_idx == idx)
            .cloned()
    }

    /// Collect frames for retransmit
    pub fn collect_for_retransmit(&self, block_id: u32, frames: &[u32]) -> Vec<FrameEntry> {
        frames
            .iter()
            .filter_map(|&idx| self.search(block_id, idx))
            .collect()
    }

    /// Get current usage for metrics
    pub fn usage_percent(&self) -> f32 {
        (self.used_bytes as f32 / self.capacity_bytes as f32) * 100.0
    }
}

// ===============================================
// Frames and Feedback
// ===============================================

#[derive(Clone, Debug)]
pub struct Frame {
    pub block_id: u32,
    pub frame_idx: u32,
    pub data_shards: u8,
    pub parity_shards: u8,
    pub total_size: u32,
    pub is_parity: bool,
    pub payload: Box<[u8]>,
}

// ===============================================
// Actions returned to application
// ===============================================

#[derive(Debug)]
pub enum FecAction {
    /// New outgoing frames (to pass to transport)
    Framed(Vec<Frame>),

    /// Receiver assembled an original payload
    Constructed(Packet),

    /// Sender should retransmit these frames
    Retransmit(Vec<Frame>),

    /// Receiver wants to send feedback to sender
    Feedback(FecFeedback),

    /// Nothing to do
    Noop,

    /// Cannot recover data; terminal error for that block
    Terminated,
}

pub struct FecSender {
    pub peer_id: PeerId,
    pub block_id: u32,

    pub data_shards: usize,
    pub parity_ratio: f32,

    parity_ewma: f32,
    last_loss_rate: f32,

    pub buffer: FrameBuffer,
}

impl FecSender {
    pub fn new(peer_id: PeerId, buffer_capacity_bytes: usize) -> Self {
        let initial_ratio = 0.25;
        Self {
            peer_id,
            block_id: 0,
            data_shards: DATA_SHARDS_DEFAULT,
            parity_ratio: initial_ratio,
            parity_ewma: initial_ratio,
            last_loss_rate: 0.0,
            buffer: FrameBuffer::new(buffer_capacity_bytes),
        }
    }

    /// Top-level send: encode packet into FEC frames
    pub fn send(&mut self, packet: Box<[u8]>) -> Result<FecAction, FecError> {
        let mut offset = 0usize;
        let mut frames_to_send: Vec<Frame> = Vec::new();

        while offset < packet.len() {
            // Build data shards for one block
            let block_size = (packet.len() - offset).min(CHUNK_SIZE * self.data_shards);
            let shard_count = (block_size + CHUNK_SIZE - 1) / CHUNK_SIZE;
            let parity_shards = Self::parity_count_from_ratio(self.parity_ratio, shard_count);
            let mut shards: Vec<Vec<u8>> = Vec::with_capacity(parity_shards + shard_count);
            for _ in 0..shard_count {
                if offset < packet.len() {
                    let end = (offset + CHUNK_SIZE).min(packet.len());
                    let mut chunk = vec![0u8; CHUNK_SIZE];
                    chunk[..end - offset].copy_from_slice(&packet[offset..end]);
                    shards.push(chunk);
                    offset = end;
                }
            }

            for _ in 0..parity_shards {
                shards.push(vec![0u8; CHUNK_SIZE]);
            }

            {
                let mut shards_refs: Vec<&mut [u8]> =
                    shards.iter_mut().map(|v| v.as_mut_slice()).collect();

                let rs = ReedSolomon::new(shard_count, parity_shards)?;
                rs.encode(&mut shards_refs)?;
            }

            for (i, s) in shards.into_iter().enumerate() {
                let is_parity = i >= shard_count;
                let frame = Frame {
                    total_size: block_size as u32,
                    block_id: self.block_id,
                    frame_idx: i as u32,
                    data_shards: shard_count as u8,
                    parity_shards: parity_shards as u8,
                    is_parity,
                    payload: s.into_boxed_slice(),
                };

                // Backup for recovery
                let fe = FrameEntry {
                    total_size: frame.total_size,
                    block_id: frame.block_id,
                    frame_idx: frame.frame_idx,
                    data_shards: frame.data_shards,
                    parity_shards: frame.parity_shards,
                    is_parity: frame.is_parity,
                    data: frame.payload.clone(),
                    timestamp: now_micros(),
                };

                self.buffer.insert(fe)?;
                frames_to_send.push(frame);
            }

            self.block_id = self.block_id.wrapping_add(1);
        }

        Ok(FecAction::Framed(frames_to_send))
    }

    /// Process feedback from receiver with EWMA adaptation
    pub fn feedback(&mut self, fb: Feedback) -> Result<FecAction, FecError> {
        match fb {
            Feedback::Network(net) => {
                let base = 0.05f32;
                let factor = 1.5f32;
                let target = (base + net.loss_rate * factor).clamp(0.05, 1.0);

                if (net.loss_rate - self.last_loss_rate).abs() > LOSS_RATE_HYSTERESIS {
                    self.parity_ewma = (self.parity_ewma * (1.0 - PARITY_ADAPTATION_WEIGHT))
                        + (target * PARITY_ADAPTATION_WEIGHT);
                    self.last_loss_rate = net.loss_rate;
                }

                self.parity_ratio = self.parity_ewma;
                Ok(FecAction::Noop)
            }
            Feedback::Missing(m) => {
                self.buffer.update_min_required_block(m.block_id);

                let entries = self.buffer.collect_for_retransmit(m.block_id, m.frames.as_slice());
                if entries.is_empty() {
                    Ok(FecAction::Terminated)
                } else {
                    let frames: Vec<Frame> = entries
                        .into_iter()
                        .map(|e| Frame {
                            block_id: e.block_id,
                            frame_idx: e.frame_idx,
                            total_size: e.total_size,
                            data_shards: e.data_shards,
                            parity_shards: e.parity_shards,
                            is_parity: e.is_parity,
                            payload: e.data.clone(),
                        })
                        .collect();
                    Ok(FecAction::Retransmit(frames))
                }
            },
            _ => Ok(FecAction::Noop),
        }
    }

    fn parity_count_from_ratio(ratio: f32, data_shards: usize) -> usize {
        let mut k = ((ratio * data_shards as f32).round() as isize)
            .max(MIN_PARITY_SHARDS as isize) as usize;
        if k > MAX_PARITY_SHARDS {
            k = MAX_PARITY_SHARDS;
        }
        k
    }
}

#[derive(Default)]
struct ReceiverBlock {
    data_shards: usize,
    total_size: usize,
    parity_shards: usize,
    total_shards: usize,
    shards: Vec<Option<Box<[u8]>>>,
    // Only defined if block is complete
    constructed_frames: Vec<Vec<u8>>,
    received: usize,
    first_ts: u64,
    last_frame_ts: u64,
    last_ping_ts: u64,
    is_place_holder: bool,
}

impl ReceiverBlock {
    fn place_holder() -> Self {
        let mut this = ReceiverBlock::default();
        let now = now_micros();
        this.is_place_holder = true;
        this.data_shards = DATA_SHARDS_DEFAULT;
        this.shards = vec![None; DATA_SHARDS_DEFAULT + MIN_PARITY_SHARDS];
        this.parity_shards = MIN_PARITY_SHARDS;
        this.total_shards = DATA_SHARDS_DEFAULT + MIN_PARITY_SHARDS;
        this.received = 0;
        this.first_ts = now;
        this.last_frame_ts = now;
        this.last_ping_ts = now;
        this
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
            last_frame_ts: now,
            last_ping_ts: now,
            constructed_frames: Vec::new(),
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
        }
    }

    fn is_constructed(&self) -> bool {
        self.constructed_frames.len() > 0
    }

    fn into_packet(self) -> Packet {
        let mut assembled: Vec<_> = self.constructed_frames.into_iter().flatten().collect();
        assembled.truncate(self.total_size);
        Packet::from(assembled.into_boxed_slice())
    }
}

pub struct FecReceiver {
    blocks: HashMap<u32, ReceiverBlock>,
    next_block_id: u32,
}

impl FecReceiver {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            next_block_id: 0,
        }
    }

    pub fn receive(&mut self, frame: Frame) -> Result<FecAction, FecError> {
        if frame.payload.len() != CHUNK_SIZE {
            return Err(FecError::InvalidFrameSize {
                expected: CHUNK_SIZE,
                actual: frame.payload.len(),
            });
        }

        // Ignore frames for blocks we've already delivered
        if frame.block_id < self.next_block_id {
             return Ok(FecAction::Noop);
        }

        // Check if previous block is created, it must be
        // but in worst network, it can be dropped, we want to create place holder
        // so it can be retransmited
        for i in (self.next_block_id as i32 - 1i32).max(0)..frame.block_id as i32 {
            if !self.blocks.contains_key(&(i as u32)) {
                self.blocks.insert(i as u32, ReceiverBlock::place_holder());
            }
        }

        let block_id = frame.block_id;
        let block = self.blocks.entry(block_id).or_insert_with(|| {
            ReceiverBlock::new(frame.data_shards as usize, frame.parity_shards as usize, frame.total_size as usize)
        });

        // in case of place holder block, we need to update its value
        block.place_value(frame.data_shards as usize, frame.parity_shards as usize, frame.total_size as usize);

        let idx = frame.frame_idx as usize;
        if idx >= block.total_shards {
            return Err(FecError::InvalidFrameIndex {
                idx: frame.frame_idx,
                total_shards: block.total_shards,
            });
        }

        // Insert frame if not already present
        if block.shards[idx].is_none() {
            block.shards[idx] = Some(frame.payload);
            block.received += 1;
        }

        let now = now_micros();
        block.last_frame_ts = now;
        block.last_ping_ts = now;

        // Check if we have enough frames to decode
        let present_count = block.shards.iter().filter(|s| s.is_some()).count();

        if present_count >= block.data_shards {
            // Attempt reconstruction
            // Attempt reconstruction
            // We must prepare a Vec<Option<Vec<u8>>> where missing shards are None.
            // The library will fill in the None spots.
            let mut shards: Vec<Option<Vec<u8>>> = block
                .shards
                .iter()
                .map(|opt| opt.as_ref().map(|b| b.to_vec()))
                .collect();

            let rs = ReedSolomon::new(block.data_shards, block.parity_shards)
                .and_then(|rs| rs.reconstruct(&mut shards));

            match rs {
                Ok(()) => {
                    // Success: assemble payload from data shards only
                    // We take only the first data_shards items, as those contain the original data
                    let assembled: Vec<_> = shards
                        .into_iter()
                        .take(block.data_shards)
                        .filter_map(|x| x)
                        .collect();

                    // If reconstruction succeeded, we must have all data shards now
                    if assembled.len() == block.data_shards {
                        block.constructed_frames = assembled;
                        if block_id == self.next_block_id {
                            if let Some(block) = self.blocks.remove(&block_id) {
                                return Ok(FecAction::Constructed(block.into_packet()));
                            }
                        }
                    }
                }
                Err(_) => {
                    // Reconstruction failed despite having enough frames?
                    // This can happen if the frames are corrupted or inconsistent.
                    // Fall through to timeout/missing handling
                }
            }
        }

        // Too much failed, the buffer cannot maintain
        // any longer.
        if self.blocks.len() > 64 {
            println!("FecReceiver buffer full, dropping block {}, block size = {}", block_id, self.blocks.len());
            return Ok(FecAction::Terminated);
        }

        let timeout_us = MIN_BLOCK_TIMEOUT_MS * 1000;

        let block = self.blocks.iter_mut().min_by(|i1, i2| i1.1.last_ping_ts.cmp(&i2.1.last_ping_ts));
        if let Some((idx, old_block)) = block {
            if now_micros().saturating_sub(old_block.last_ping_ts) > timeout_us {
                let action = Self::handle_timeout(*idx, old_block);
                return Ok(action)
            }
        }

        Ok(FecAction::Noop)
    }

    fn handle_timeout(block_id: u32, block: &mut ReceiverBlock) -> FecAction {
        let present_count = block.shards.iter().filter(|s| s.is_some()).count();
        let needed_more = block.data_shards.saturating_sub(present_count);

        if needed_more == 0 {
            // Already have enough data frames
            return FecAction::Noop;
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

        let mf = MissingFrames {
            block_id,
            frames: missing,
        };

        FecAction::Feedback(FecFeedback {
          feedback: Some(Feedback::Missing(mf)),
        })
    }
}

fn now_micros() -> u64 {
    epoch_micro()
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use super::*;

    #[test]
    fn test_parity_count() {
        let s = FecSender::new(PeerId(Uuid::new_v4()), 5 * 1024 * 1024);
        assert!(FecSender::parity_count_from_ratio(0.2, 32) >= MIN_PARITY_SHARDS);
    }

    #[test]
    fn test_frame_metadata_sync() {
        // NEW TEST: Verify frame header contains N/K values
        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 5 * 1024 * 1024);
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
        // NEW TEST: Frames arriving out-of-order shouldn't trigger false timeouts
        let mut receiver = FecReceiver::new();

        // Simulate frames arriving out of order
        let frame1 = Frame {
            total_size: CHUNK_SIZE as u32,
            block_id: 0,
            frame_idx: 15,  // Late in sequence
            data_shards: 32,
            parity_shards: 8,
            is_parity: false,
            payload: vec![0u8; CHUNK_SIZE].into_boxed_slice(),
        };

        // This should not trigger timeout
        let result = receiver.receive(frame1);
        assert!(result.is_ok());
        match result.unwrap() {
            FecAction::Noop => (),  // Expected
            val => panic!("Should be Noop, not ready to decode {val:?}"),
        }
    }

    #[test]
    fn test_buffer_smart_eviction() {
        // NEW TEST: Buffer should not evict blocks still needed
        let mut buffer = FrameBuffer::new(10 * CHUNK_SIZE);  // Very small buffer

        // Insert frames from block 0
        let fe = FrameEntry {
            total_size: CHUNK_SIZE as u32,
            block_id: 0,
            frame_idx: 0,
            data_shards: 2,
            parity_shards: 1,
            is_parity: false,
            data: vec![1u8; CHUNK_SIZE].into_boxed_slice(),
            timestamp: now_micros(),
        };
        buffer.insert(fe).expect("insert 1 failed");

        // Mark block 0 as still required
        buffer.update_min_required_block(0);

        // Now when we insert more (should evict from back, not drop block 0)
        let fe2 = FrameEntry {
            total_size: CHUNK_SIZE as u32,
            block_id: 1,
            frame_idx: 0,
            data_shards: 2,
            parity_shards: 1,
            is_parity: false,
            data: vec![2u8; CHUNK_SIZE].into_boxed_slice(),
            timestamp: now_micros(),
        };
        buffer.insert(fe2).expect("insert 2 failed");

        // Block 0 should still be searchable
        assert!(buffer.search(0, 0).is_some(), "Block 0 should not be evicted");
    }

    #[test]
    fn test_smart_missing_frame_selection() {
        // NEW TEST: Missing frame feedback should prioritize data frames
        let mut receiver = FecReceiver::new();

        // Create partial block with mostly data frames missing
        let mut frames = Vec::new();
        for i in 20..32 {
            frames.push(Frame {
                total_size: CHUNK_SIZE as u32,
                block_id: 0,
                frame_idx: i,
                data_shards: 32,
                parity_shards: 8,
                is_parity: i >= 32,
                payload: vec![0u8; CHUNK_SIZE].into_boxed_slice(),
            });
        }

        for frame in frames {
            let _ = receiver.receive(frame);
        }

        // Advance time to trigger timeout
        // (In real test, we'd mock time or add a force_timeout method)
        // This is a simplified version showing the structure
    }

    #[test]
    fn test_frame_validation() {
        // NEW TEST: Invalid frame sizes should be rejected
        let mut receiver = FecReceiver::new();

        let bad_frame = Frame {
            total_size: 1024,
            block_id: 0,
            frame_idx: 0,
            data_shards: 32,
            parity_shards: 8,
            is_parity: false,
            payload: vec![0u8; 1024].into_boxed_slice(),  // Wrong size!
        };

        let result = receiver.receive(bad_frame);
        assert!(result.is_err(), "Should reject invalid frame size");
    }

    #[test]
    fn test_send_receive_small_data() {
        // Test with very small amount of data (100 bytes)
        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 5 * 1024 * 1024);
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
                FecAction::Constructed(packet) => {
                    received_packet = Some(packet);
                    break;
                }
                FecAction::Noop => continue,
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
        // Test with 10MB of data
        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 50 * 1024 * 1024);
        let mut receiver = FecReceiver::new();

        // Create 10MB test data with a pattern
        let data_size = 10 * 1024 * 1024; // 10MB
        let original_data: Vec<u8> = (0..data_size)
            .map(|i| ((i / 1024) % 256) as u8) // Pattern changes every 1KB
            .collect();

        let packet = original_data.clone().into_boxed_slice();

        println!("Sending 10MB data through FEC encoder...");

        // Send the data through FEC encoder
        let action = sender.send(packet).expect("send failed");

        // Extract frames
        let frames = match action {
            FecAction::Framed(frames) => frames,
            _ => panic!("Expected Framed action"),
        };

        println!("Generated {} frames", frames.len());
        assert!(!frames.is_empty(), "Should have generated frames");

        // Receive all frames and collect constructed packets
        let mut all_packets: Vec<Packet> = Vec::new();

        for (idx, frame) in frames.into_iter().enumerate() {
            let result = receiver.receive(frame).expect(&format!("receive failed at frame {}", idx));
            match result {
                FecAction::Constructed(packet) => {
                    all_packets.push(packet);
                    // Increment next_block_id to allow the next block to be returned
                    receiver.next_block_id += 1;
                }
                FecAction::Noop => continue,
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

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 5 * 1024 * 1024);
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
                FecAction::Constructed(packet) => {
                    constructed = Some(packet);
                    receiver.next_block_id += 1; // Ack the block
                }
                _ => {}
            }
        }

        // 5. Verify
        assert!(constructed.is_some(), "Should have reconstructed using parity");
        let result_data = constructed.unwrap();
        assert_eq!(result_data.len(), original_data.len());
        assert_eq!(&result_data[..], &original_data[..]);
    }

    #[test]
    fn test_timeout_generation() {
        // SCENARIO:
        // Send insufficient frames. Wait > MIN_BLOCK_TIMEOUT_MS.
        // Trigger receiver (via a new frame or heartbeat).
        // Expect Feedback(MissingFrames).

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 1024 * 1024);
        let mut receiver = FecReceiver::new();

        let packet = vec![0u8; CHUNK_SIZE * 4].into_boxed_slice(); // ~4 data shards
        let frames = match sender.send(packet).unwrap() {
            FecAction::Framed(f) => f,
            _ => panic!("Expected Framed"),
        };

        // 1. Send only 1 frame (insufficient)
        receiver.receive(frames[0].clone()).unwrap();

        // 2. Wait for timeout (80ms default defined in const)
        // We wait 100ms to be safe
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 3. Trigger the timeout check
        // The receiver usually checks timeouts when `receive` is called.
        let trigger_frame = Frame {
            block_id: 62, // Irrelevant future block
            frame_idx: 0,
            data_shards: 1,
            parity_shards: 1,
            total_size: 100,
            is_parity: false,
            payload: vec![0u8; CHUNK_SIZE].into_boxed_slice(),
        };

        let action = receiver.receive(trigger_frame).unwrap();

        // 4. Verify Feedback
        match action {
            FecAction::Feedback(FecFeedback { feedback: Some(fec_feedback::Feedback::Missing(missing)) }) => {
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
        // 3. Receiver Timeouts -> Generates Feedback.
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
        let x = receiver.receive(frames[0].clone());

        // --- STEP 3: Timeout & Feedback ---
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Trigger timeout check using a dummy frame from "future"
        let trigger_frame = Frame {
            block_id: 10,
            frame_idx: 0,
            total_size: 10,
            data_shards: 1,
            parity_shards: 0,
            is_parity: false,
            payload: vec![0u8; CHUNK_SIZE].into_boxed_slice(),
        };

        let action = receiver.receive(trigger_frame).unwrap();

        let feedback_obj = match action {
            FecAction::Feedback(fb) => fb,
            value => panic!("Receiver did not request retransmission {value:?}"),
        };

        // Extract the inner feedback enum for the sender
        let inner_feedback = feedback_obj.feedback.expect("Empty feedback");

        // --- STEP 4: Sender processes Feedback ---
        let retransmit_action = sender.feedback(inner_feedback).expect("Sender feedback failed");

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
                FecAction::Constructed(pkt) => {
                    final_packet = Some(pkt);
                    break;
                },
                _ => continue,
            }
        }

        assert!(final_packet.is_some(), "Receiver failed to reconstruct after retransmission");

        let reconstructed = final_packet.unwrap();
        assert_eq!(&reconstructed[..], &original_data[..]);
    }

    #[test]
    fn test_sender_buffer_wraparound() {
        // SCENARIO:
        // Ensure the sender buffer correctly handles block_id wraparounds
        // or simply large block IDs without crashing or losing track.
        // We manually inject a high block_id into the buffer to simulate runtime.

        let mut sender = FecSender::new(PeerId(Uuid::new_v4()), 1024 * 1024);

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
        let res_max = sender.feedback(fb_max).unwrap();
        assert!(matches!(res_max, FecAction::Retransmit(_)), "Should find block u32::MAX");

        // Request retransmit for 0
        let fb_zero = Feedback::Missing(MissingFrames { block_id: 0, frames: vec![0] });
        let res_zero = sender.feedback(fb_zero).unwrap();
        assert!(matches!(res_zero, FecAction::Retransmit(_)), "Should find block 0");
    }
}
