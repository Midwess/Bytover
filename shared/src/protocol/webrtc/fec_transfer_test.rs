use crate::protocol::webrtc::fec::{
    FecAction, FecReceiver, FecSender, Frame, CHUNK_SIZE, DATA_SHARDS_DEFAULT,
};
use matchbox_protocol::PeerId;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;
use std::time::Duration;

const TEST_DATA_SIZE: usize = 10 * 1024 * 1024;
const TEST_LOSS_RATE: f64 = 0.05;
const TEST_RTT_MS: u64 = 30;
const TEST_SEED: u64 = 22;

struct NetworkSimulator {
    loss_rate: f64,
    rng: StdRng,
    in_flight: VecDeque<Box<[u8]>>,
    packets_sent: u64,
    packets_dropped: u64,
}

impl NetworkSimulator {
    fn new(loss_rate: f64, seed: u64) -> Self {
        Self {
            loss_rate,
            rng: StdRng::seed_from_u64(seed),
            in_flight: VecDeque::new(),
            packets_sent: 0,
            packets_dropped: 0,
        }
    }

    fn send(&mut self, packet: Box<[u8]>) {
        self.packets_sent += 1;
        if self.rng.gen::<f64>() < self.loss_rate {
            self.packets_dropped += 1;
        } else {
            self.in_flight.push_back(packet);
        }
    }

    /// Send packet reliably (no loss) - used for retransmit channel
    fn send_reliable(&mut self, packet: Box<[u8]>) {
        self.packets_sent += 1;
        self.in_flight.push_back(packet);
    }

    fn receive_all(&mut self) -> Vec<Box<[u8]>> {
        self.in_flight.drain(..).collect()
    }

    fn has_pending(&self) -> bool {
        !self.in_flight.is_empty()
    }

    fn stats(&self) -> (u64, u64) {
        (self.packets_sent, self.packets_dropped)
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
async fn test_fec_transfer_100mb_with_loss() {
    let test_data = generate_test_data(TEST_DATA_SIZE);
    let peer_id: PeerId = uuid::Uuid::new_v4().into();

    let mut fec_sender = FecSender::new(peer_id, 4096);
    fec_sender.set_rtt(TEST_RTT_MS);

    let mut fec_receiver = FecReceiver::with_window_size(2048);
    fec_receiver.set_rtt(TEST_RTT_MS);

    let mut network = NetworkSimulator::new(TEST_LOSS_RATE, TEST_SEED);
    let mut received_data: Vec<u8> = Vec::with_capacity(TEST_DATA_SIZE);

    let chunk_size = CHUNK_SIZE * DATA_SHARDS_DEFAULT;
    let prefix: u16 = 1;
    let mut offset = 0;
    let mut stall_iterations = 0;
    let max_stall_iterations = 100;
    let timeout = std::time::Instant::now();
    let max_duration = Duration::from_secs(600);

    while received_data.len() < TEST_DATA_SIZE {
        if timeout.elapsed() > max_duration {
            panic!(
                "Test timeout: received={}/{} after {:?}",
                received_data.len(),
                TEST_DATA_SIZE,
                timeout.elapsed()
            );
        }

        let prev_received = received_data.len();

        if offset < test_data.len() {
            let end = (offset + chunk_size).min(test_data.len());
            let chunk = test_data[offset..end].to_vec().into_boxed_slice();

            match fec_sender.send(prefix, chunk) {
                Ok(FecAction::Framed(frames)) => {
                    for frame in frames {
                        network.send(frame.serialize());
                    }
                }
                Ok(_) => {}
                Err(e) => panic!("FEC send error: {:?}", e),
            }
            offset = end;
        }

        let packets = network.receive_all();
        if !packets.is_empty() {
            let mut frames = Vec::new();
            for packet in packets {
                if let Some(frame) = Frame::deserialize(&packet) {
                    frames.push(frame);
                }
            }

            if !frames.is_empty() {
                match fec_receiver.receive(frames) {
                    Ok(FecAction::Constructed(packets_with_prefix, _)) => {
                        for (_, packet) in packets_with_prefix {
                            received_data.extend_from_slice(&packet);
                        }
                    }
                    Ok(FecAction::Feedback(fb, _)) => {
                        if let Some(feedback) = fb.feedback {
                            match fec_sender.feedback(feedback) {
                                FecAction::Retransmit(frames) => {
                                    for frame in frames {
                                        network.send_reliable(frame.serialize());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(FecAction::Terminated) => {
                        panic!(
                            "FEC receiver terminated: received={}/{}, offset={}/{}",
                            received_data.len(),
                            TEST_DATA_SIZE,
                            offset,
                            test_data.len()
                        );
                    }
                    Ok(_) => {}
                    Err(e) => panic!("FEC receive error: {:?}", e),
                }
            }
        }

        if offset >= test_data.len() && received_data.len() < TEST_DATA_SIZE {
            tokio::time::sleep(Duration::from_millis(100)).await;

            match fec_receiver.ping() {
                Ok(FecAction::Feedback(fb, _)) => {
                    if let Some(feedback) = fb.feedback {
                        match fec_sender.feedback(feedback) {
                            FecAction::Retransmit(frames) => {
                                for frame in frames {
                                    network.send_reliable(frame.serialize());
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(FecAction::Constructed(packets_with_prefix, _)) => {
                    for (_, packet) in packets_with_prefix {
                        received_data.extend_from_slice(&packet);
                    }
                }
                Ok(_) => {}
                Err(e) => panic!("FEC ping error: {:?}", e),
            }

            if received_data.len() == prev_received && !network.has_pending() {
                stall_iterations += 1;
                if stall_iterations > max_stall_iterations {
                    panic!(
                        "Transfer stalled: received={}/{}, no progress after {} sleep iterations",
                        received_data.len(),
                        TEST_DATA_SIZE,
                        max_stall_iterations
                    );
                }
            } else {
                stall_iterations = 0;
            }
        }
    }

    let (sent, dropped) = network.stats();
    let (frames_received, frames_lost, retransmit_count, false_retransmit) = fec_receiver.stats();
    let receiver_loss_rate = fec_receiver.calculate_loss_rate();

    println!(
        "Network stats: sent={}, dropped={}, loss_rate={:.2}%",
        sent,
        dropped,
        if sent > 0 { (dropped as f64 / sent as f64) * 100.0 } else { 0.0 }
    );
    println!(
        "Receiver stats: frames_received={}, frames_lost={}, retransmit_requests={}, false_retransmit={}, observed_loss_rate={:.2}%",
        frames_received,
        frames_lost,
        retransmit_count,
        false_retransmit,
        receiver_loss_rate * 100.0
    );
    println!(
        "Transfer complete: expected={}, received={}",
        TEST_DATA_SIZE,
        received_data.len()
    );

    assert_eq!(
        received_data.len(),
        TEST_DATA_SIZE,
        "Data length mismatch: expected {}, got {}",
        TEST_DATA_SIZE,
        received_data.len()
    );

    assert_eq!(test_data, received_data, "Data integrity check failed");
}
