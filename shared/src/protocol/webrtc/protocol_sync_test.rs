use crate::protocol::webrtc::fec::{
    FecAction, FecReceiver, FecSender, Frame, CHUNK_SIZE, DATA_SHARDS_DEFAULT,
};
use matchbox_protocol::PeerId;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;
use std::time::Duration;
use schema::devlog::bitbridge::FecFeedback;
use schema::devlog::bitbridge::fec_feedback::Feedback;
use schema::devlog::bitbridge::NetworkStats;

// 100MB as requested
const TEST_DATA_SIZE: usize = 100 * 1024 * 1024;
// 10% loss as requested
const TEST_LOSS_RATE: f64 = 0.10;
const TEST_RTT_MS: u64 = 50;
const TEST_SEED: u64 = 42;

struct NetworkSimulator {
    loss_rate: f64,
    rng: StdRng,
    in_flight: VecDeque<(Box<[u8]>, std::time::Instant)>,
    packets_sent: u64,
    packets_dropped: u64,
    latency: Duration,
}

impl NetworkSimulator {
    fn new(loss_rate: f64, latency: Duration, seed: u64) -> Self {
        Self {
            loss_rate,
            rng: StdRng::seed_from_u64(seed),
            in_flight: VecDeque::new(),
            packets_sent: 0,
            packets_dropped: 0,
            latency,
        }
    }

    fn send(&mut self, packet: Box<[u8]>, reliable: bool) {
        self.packets_sent += 1;
        // Only drop unreliable packets
        if !reliable && self.rng.gen::<f64>() < self.loss_rate {
            self.packets_dropped += 1;
        } else {
            self.in_flight.push_back((packet, std::time::Instant::now() + self.latency));
        }
    }

    fn receive_ready(&mut self) -> Vec<Box<[u8]>> {
        let now = std::time::Instant::now();
        let mut ready = Vec::new();
        while let Some((_, time)) = self.in_flight.front() {
            if *time <= now {
                ready.push(self.in_flight.pop_front().unwrap().0);
            } else {
                break;
            }
        }
        ready
    }

    fn has_pending(&self) -> bool {
        !self.in_flight.is_empty()
    }
}

fn generate_test_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    for i in 0..size {
        data.push((i % 256) as u8);
    }
    data
}

#[tokio::test]
async fn test_protocol_sync_100mb_random_loss() {
    let test_data = generate_test_data(TEST_DATA_SIZE);
    let peer_id: PeerId = uuid::Uuid::new_v4().into();

    let mut fec_sender = FecSender::new(4096);
    fec_sender.set_rtt(TEST_RTT_MS);

    let mut fec_receiver = FecReceiver::with_window_size(4096);
    fec_receiver.set_rtt(TEST_RTT_MS);

    let mut network_to_receiver = NetworkSimulator::new(TEST_LOSS_RATE, Duration::from_millis(TEST_RTT_MS / 2), TEST_SEED);
    let mut network_to_sender = NetworkSimulator::new(0.0, Duration::from_millis(TEST_RTT_MS / 2), TEST_SEED + 1);
    
    let mut received_data: Vec<u8> = Vec::with_capacity(TEST_DATA_SIZE);

    let chunk_size = CHUNK_SIZE * DATA_SHARDS_DEFAULT;
    let mut offset = 0;
    let start_time = std::time::Instant::now();
    let max_duration = Duration::from_secs(300); 

    // Sliding window tracking
    let mut last_peer_block_id = 0;
    const WINDOW_SIZE: u32 = 128;

    println!("Starting 100MB transfer test with 10% loss and {}ms RTT...", TEST_RTT_MS);

    while received_data.len() < TEST_DATA_SIZE || network_to_receiver.has_pending() || network_to_sender.has_pending() {
        if start_time.elapsed() > max_duration {
            let sent = network_to_receiver.packets_sent;
            let dropped = network_to_receiver.packets_dropped;
            panic!(
                "Test timeout! Received {}/{} bytes. Sent: {}, Dropped: {}, Peer Block: {}, Sender Block: {}",
                received_data.len(),
                TEST_DATA_SIZE,
                sent,
                dropped,
                last_peer_block_id,
                fec_sender.block_id
            );
        }

        // 1. Sender: Send data if window allows
        while offset < TEST_DATA_SIZE && (fec_sender.block_id.wrapping_sub(last_peer_block_id) < WINDOW_SIZE) {
            let end = (offset + chunk_size).min(TEST_DATA_SIZE);
            let packet = test_data[offset..end].to_vec().into_boxed_slice();
            
            if let Ok(FecAction::Framed(frames)) = fec_sender.send(0, packet) {
                for frame in frames {
                    network_to_receiver.send(frame.serialize(), false);
                }
            }
            offset = end;
        }

        // 2. Receiver: Process packets
        let packets = network_to_receiver.receive_ready();
        let mut frames = Vec::new();
        for p in packets {
            if let Some(frame) = Frame::deserialize(&p) {
                frames.push(frame);
            }
        }

        if !frames.is_empty() {
            match fec_receiver.receive(frames) {
                Ok(FecAction::Constructed(packets, _)) => {
                    for (_, p) in packets {
                        received_data.extend_from_slice(&p);
                    }
                    // Simulate Event-Based ACK on construction
                    let feedback = FecFeedback {
                        feedback: Some(Feedback::Network(NetworkStats {
                            current_block_id: Some(fec_receiver.current_block_id()),
                            rtt: Some(TEST_RTT_MS as u32),
                            loss_rate: 0.0,
                            hold_counter: None,
                        }))
                    };
                    network_to_sender.send(bincode::serialize(&feedback).unwrap().into_boxed_slice(), true);
                }
                Ok(FecAction::Feedback(fb, _)) => {
                    network_to_sender.send(bincode::serialize(&fb).unwrap().into_boxed_slice(), true);
                }
                _ => {}
            }
        } else {
            // Periodic ping (checks for loss)
            if let Ok(FecAction::Feedback(fb, _)) = fec_receiver.ping() {
                network_to_sender.send(bincode::serialize(&fb).unwrap().into_boxed_slice(), true);
            }
        }

        // 3. Sender: Process Feedback
        let feedback_packets = network_to_sender.receive_ready();
        for p in feedback_packets {
            let fb: FecFeedback = bincode::deserialize(&p).unwrap();
            if let Some(feedback) = fb.feedback {
                match &feedback {
                    Feedback::Network(stats) => {
                        if let Some(bid) = stats.current_block_id {
                            last_peer_block_id = last_peer_block_id.max(bid);
                        }
                    }
                    Feedback::Missing(missing) => {
                        if let Some(first) = missing.blocks.first() {
                            last_peer_block_id = last_peer_block_id.max(first.block_id);
                        }
                    }
                }
                
                let action = fec_sender.feedback(feedback);
                if let FecAction::Retransmit(frames) = action {
                    for frame in frames {
                        network_to_receiver.send(frame.serialize(), true);
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(1)).await;
        
        if received_data.len() % (10 * 1024 * 1024) == 0 && received_data.len() > 0 {
            println!("Progress: {}/{} MB", received_data.len() / (1024 * 1024), TEST_DATA_SIZE / (1024 * 1024));
        }
    }

    println!("Transfer completed in {:?}", start_time.elapsed());
    assert_eq!(received_data.len(), TEST_DATA_SIZE, "Data size mismatch");
    assert_eq!(received_data, test_data, "Data corruption detected");
}
