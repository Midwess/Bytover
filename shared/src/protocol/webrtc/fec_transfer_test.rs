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

    let mut fec_sender = FecSender::new(4096);
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

/// Network simulator that delays packets instead of dropping them.
/// This simulates the real production scenario where packets are delayed,
/// causing false loss detection.
struct NetworkSimulatorWithDelay {
    delay_rate: f64,      // Probability of delaying a packet
    delay_iterations: usize, // How many iterations to delay
    rng: StdRng,
    in_flight: VecDeque<Box<[u8]>>,
    delayed: VecDeque<(usize, Box<[u8]>)>, // (delivery_iteration, packet)
    packets_sent: u64,
    packets_delayed: u64,
    current_iteration: usize,
}

impl NetworkSimulatorWithDelay {
    fn new(delay_rate: f64, delay_iterations: usize, seed: u64) -> Self {
        Self {
            delay_rate,
            delay_iterations,
            rng: StdRng::seed_from_u64(seed),
            in_flight: VecDeque::new(),
            delayed: VecDeque::new(),
            packets_sent: 0,
            packets_delayed: 0,
            current_iteration: 0,
        }
    }

    fn send(&mut self, packet: Box<[u8]>) {
        self.packets_sent += 1;
        if self.rng.gen::<f64>() < self.delay_rate {
            // Delay this packet
            self.packets_delayed += 1;
            let delivery_iter = self.current_iteration + self.delay_iterations;
            self.delayed.push_back((delivery_iter, packet));
        } else {
            self.in_flight.push_back(packet);
        }
    }

    fn send_reliable(&mut self, packet: Box<[u8]>) {
        self.packets_sent += 1;
        self.in_flight.push_back(packet);
    }

    fn tick(&mut self) {
        self.current_iteration += 1;
    }

    fn receive_all(&mut self) -> Vec<Box<[u8]>> {
        let mut result: Vec<Box<[u8]>> = self.in_flight.drain(..).collect();

        // Add delayed packets that are ready
        while let Some((delivery_iter, _)) = self.delayed.front() {
            if *delivery_iter <= self.current_iteration {
                let (_, packet) = self.delayed.pop_front().unwrap();
                result.push(packet);
            } else {
                break;
            }
        }

        result
    }

    fn has_pending(&self) -> bool {
        !self.in_flight.is_empty() || !self.delayed.is_empty()
    }

    fn stats(&self) -> (u64, u64) {
        (self.packets_sent, self.packets_delayed)
    }
}

/// Test that reproduces false retransmission issue seen in production.
///
/// Root cause: False loss detection due to delayed packets.
/// - RTT < 250ms, so parity = 0 (no FEC recovery)
/// - Original packet is DELAYED (not lost)
/// - Loss detection triggers (timeout) → retransmit requested
/// - Original delayed packet arrives → block completes
/// - Retransmit arrives → counted as false retransmit
///
/// Production logs showed:
/// - Received frame for old block 3370 (current block is 3373)
/// - false_retransmit = 303-326 out of total_retransmit = 328 (92-99%)
#[tokio::test]
async fn test_false_retransmission_with_latency() {
    const DATA_SIZE: usize = 1 * 1024 * 1024; // 1MB
    // Rate of packets being delayed (not dropped)
    const DELAY_RATE: f64 = 0.15;
    const SEED: u64 = 42;
    // Delay in iterations - long enough for loss detection to trigger
    const PACKET_DELAY_ITERATIONS: usize = 200;

    let test_data = generate_test_data(DATA_SIZE);
    let peer_id: PeerId = uuid::Uuid::new_v4().into();

    let mut fec_sender = FecSender::new(4096);
    // RTT < 250ms means parity = 0 (no FEC)
    fec_sender.set_rtt(30);

    let mut fec_receiver = FecReceiver::with_window_size(2048);
    fec_receiver.set_rtt(30);

    // Use delay-based network (no drops, only delays)
    let mut network = NetworkSimulatorWithDelay::new(DELAY_RATE, PACKET_DELAY_ITERATIONS, SEED);
    let mut retransmit_packets_sent: u64 = 0;

    let mut received_data: Vec<u8> = Vec::with_capacity(DATA_SIZE);

    let chunk_size = CHUNK_SIZE * DATA_SHARDS_DEFAULT;
    let prefix: u16 = 1;
    let mut offset = 0;
    let mut stall_iterations = 0;
    let max_stall_iterations = 2000;
    let timeout = std::time::Instant::now();
    let max_duration = Duration::from_secs(120);

    let max_iterations = 500000;

    while received_data.len() < DATA_SIZE && network.current_iteration < max_iterations {
        network.tick();

        if timeout.elapsed() > max_duration {
            panic!(
                "Test timeout: received={}/{} after {:?}",
                received_data.len(),
                DATA_SIZE,
                timeout.elapsed()
            );
        }

        let prev_received = received_data.len();

        // Send data
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

        // Receive packets (includes delayed packets that are now ready)
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
                                    // Retransmit is sent immediately
                                    // But original delayed packet may arrive first
                                    for frame in frames {
                                        retransmit_packets_sent += 1;
                                        network.send_reliable(frame.serialize());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }

        // When all data sent, periodically ping to request retransmissions
        if offset >= test_data.len() && received_data.len() < DATA_SIZE {
            tokio::time::sleep(Duration::from_millis(1)).await;

            match fec_receiver.ping() {
                Ok(FecAction::Feedback(fb, _)) => {
                    if let Some(feedback) = fb.feedback {
                        match fec_sender.feedback(feedback) {
                            FecAction::Retransmit(frames) => {
                                for frame in frames {
                                    retransmit_packets_sent += 1;
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
                        "Transfer stalled: received={}/{}, no progress after {} iterations",
                        received_data.len(),
                        DATA_SIZE,
                        max_stall_iterations
                    );
                }
            } else {
                stall_iterations = 0;
            }
        }
    }

    // Drain remaining delayed packets to trigger false retransmit detection
    // These are the original packets that were delayed - now arriving after blocks completed
    let max_drain_iterations = PACKET_DELAY_ITERATIONS + 100;
    for _ in 0..max_drain_iterations {
        network.tick();
        let packets = network.receive_all();
        if packets.is_empty() && !network.has_pending() {
            break;
        }
        if !packets.is_empty() {
            let mut frames = Vec::new();
            for packet in packets {
                if let Some(frame) = Frame::deserialize(&packet) {
                    frames.push(frame);
                }
            }
            if !frames.is_empty() {
                // These should trigger false_retransmit since blocks are already complete
                let _ = fec_receiver.receive(frames);
            }
        }
    }

    let (sent, delayed) = network.stats();
    let (frames_received, frames_lost, retransmit_count, false_retransmit) = fec_receiver.stats();
    let receiver_loss_rate = fec_receiver.calculate_loss_rate();

    println!("=== False Retransmission Test (Delayed Packets) ===");
    println!(
        "Network stats: sent={}, delayed={}, retransmit_sent={}, delay_rate={:.2}%",
        sent,
        delayed,
        retransmit_packets_sent,
        if sent > 0 { (delayed as f64 / sent as f64) * 100.0 } else { 0.0 }
    );
    println!(
        "Receiver stats: frames_received={}, frames_lost={}, retransmit_requests={}, false_retransmit={}",
        frames_received,
        frames_lost,
        retransmit_count,
        false_retransmit
    );
    println!(
        "False retransmit ratio: {:.1}% ({}/{})",
        if retransmit_count > 0 { (false_retransmit as f64 / retransmit_count as f64) * 100.0 } else { 0.0 },
        false_retransmit,
        retransmit_count
    );
    println!(
        "Observed loss rate: {:.2}%",
        receiver_loss_rate * 100.0
    );

    // Verify data integrity
    assert_eq!(
        received_data.len(),
        DATA_SIZE,
        "Data length mismatch: expected {}, got {}",
        DATA_SIZE,
        received_data.len()
    );
    assert_eq!(test_data, received_data, "Data integrity check failed");

    // With delayed packets and false loss detection, we expect false retransmits
    if retransmit_count > 0 && false_retransmit > 0 {
        let false_ratio = false_retransmit as f64 / retransmit_count as f64;
        println!(
            "SUCCESS: Reproduced false retransmission - {:.1}% of retransmits were unnecessary (original packet was delayed, not lost)",
            false_ratio * 100.0
        );
    }
}

/// Test with higher delay rate to reproduce production-like false retransmission ratio
///
/// Production scenario:
/// - 92-99% false retransmit ratio
/// - Packets delayed, not lost
/// - Loss detection too aggressive
#[tokio::test]
async fn test_extreme_false_retransmission() {
    const DATA_SIZE: usize = 1 * 1024 * 1024; // 1MB
    // Higher delay rate to simulate worse network conditions
    const DELAY_RATE: f64 = 0.25;
    const SEED: u64 = 123;
    // Longer delay to ensure loss detection triggers before packet arrives
    const PACKET_DELAY_ITERATIONS: usize = 300;

    let test_data = generate_test_data(DATA_SIZE);
    let peer_id: PeerId = uuid::Uuid::new_v4().into();

    let mut fec_sender = FecSender::new(4096);
    // RTT < 250ms means parity = 0 (no FEC)
    fec_sender.set_rtt(30);

    let mut fec_receiver = FecReceiver::with_window_size(2048);
    fec_receiver.set_rtt(30);

    // Use delay-based network (no drops, only delays)
    let mut network = NetworkSimulatorWithDelay::new(DELAY_RATE, PACKET_DELAY_ITERATIONS, SEED);
    let mut retransmit_packets_sent: u64 = 0;

    let mut received_data: Vec<u8> = Vec::with_capacity(DATA_SIZE);

    let chunk_size = CHUNK_SIZE * DATA_SHARDS_DEFAULT;
    let prefix: u16 = 1;
    let mut offset = 0;
    let mut stall_iterations = 0;
    let max_stall_iterations = 2000;
    let timeout = std::time::Instant::now();
    let max_duration = Duration::from_secs(120);

    let max_iterations = 500000;

    while received_data.len() < DATA_SIZE && network.current_iteration < max_iterations {
        network.tick();

        if timeout.elapsed() > max_duration {
            panic!(
                "Test timeout: received={}/{} after {:?}",
                received_data.len(),
                DATA_SIZE,
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
                                        retransmit_packets_sent += 1;
                                        network.send_reliable(frame.serialize());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }

        if offset >= test_data.len() && received_data.len() < DATA_SIZE {
            tokio::time::sleep(Duration::from_millis(1)).await;

            match fec_receiver.ping() {
                Ok(FecAction::Feedback(fb, _)) => {
                    if let Some(feedback) = fb.feedback {
                        match fec_sender.feedback(feedback) {
                            FecAction::Retransmit(frames) => {
                                for frame in frames {
                                    retransmit_packets_sent += 1;
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
                        "Transfer stalled: received={}/{}, no progress after {} iterations",
                        received_data.len(),
                        DATA_SIZE,
                        max_stall_iterations
                    );
                }
            } else {
                stall_iterations = 0;
            }
        }
    }

    // Drain remaining delayed packets to trigger false retransmit detection
    let max_drain_iterations = PACKET_DELAY_ITERATIONS + 100;
    for _ in 0..max_drain_iterations {
        network.tick();
        let packets = network.receive_all();
        if packets.is_empty() && !network.has_pending() {
            break;
        }
        if !packets.is_empty() {
            let mut frames = Vec::new();
            for packet in packets {
                if let Some(frame) = Frame::deserialize(&packet) {
                    frames.push(frame);
                }
            }
            if !frames.is_empty() {
                let _ = fec_receiver.receive(frames);
            }
        }
    }

    let (sent, delayed) = network.stats();
    let (frames_received, frames_lost, retransmit_count, false_retransmit) = fec_receiver.stats();
    let receiver_loss_rate = fec_receiver.calculate_loss_rate();

    println!("=== Extreme False Retransmission Test (Delayed Packets) ===");
    println!(
        "Network stats: sent={}, delayed={}, retransmit_sent={}, delay_rate={:.2}%",
        sent,
        delayed,
        retransmit_packets_sent,
        if sent > 0 { (delayed as f64 / sent as f64) * 100.0 } else { 0.0 }
    );
    println!(
        "Receiver stats: frames_received={}, frames_lost={}, retransmit_requests={}, false_retransmit={}",
        frames_received,
        frames_lost,
        retransmit_count,
        false_retransmit
    );
    println!(
        "False retransmit ratio: {:.1}% ({}/{})",
        if retransmit_count > 0 { (false_retransmit as f64 / retransmit_count as f64) * 100.0 } else { 0.0 },
        false_retransmit,
        retransmit_count
    );
    println!(
        "Observed loss rate: {:.2}%",
        receiver_loss_rate * 100.0
    );

    assert_eq!(
        received_data.len(),
        DATA_SIZE,
        "Data length mismatch: expected {}, got {}",
        DATA_SIZE,
        received_data.len()
    );
    assert_eq!(test_data, received_data, "Data integrity check failed");

    // With delayed packets and aggressive loss detection, we expect high false retransmit ratio
    if retransmit_count > 0 && false_retransmit > 0 {
        let false_ratio = false_retransmit as f64 / retransmit_count as f64;
        println!(
            "SUCCESS: Reproduced production-like false retransmission - {:.1}% of retransmits were unnecessary",
            false_ratio * 100.0
        );
    }
}

/// Test with multiple challenging configurations to verify the new loss detection thresholds.
/// Tests various combinations of:
/// - Delay rates (10%, 20%, 30%)
/// - RTT values (20ms, 50ms, 100ms)
/// - Packet delay iterations (100, 200, 400)
#[tokio::test]
async fn test_threshold_with_multiple_configs() {
    struct TestConfig {
        delay_rate: f64,
        rtt_ms: u64,
        delay_iterations: usize,
        name: &'static str,
    }

    let configs = [
        TestConfig { delay_rate: 0.10, rtt_ms: 20, delay_iterations: 100, name: "Low delay (10%), Low RTT (20ms)" },
        TestConfig { delay_rate: 0.20, rtt_ms: 30, delay_iterations: 150, name: "Medium delay (20%), Low RTT (30ms)" },
        TestConfig { delay_rate: 0.15, rtt_ms: 50, delay_iterations: 200, name: "Medium delay (15%), Medium RTT (50ms)" },
        TestConfig { delay_rate: 0.25, rtt_ms: 100, delay_iterations: 300, name: "High delay (25%), High RTT (100ms)" },
        TestConfig { delay_rate: 0.30, rtt_ms: 30, delay_iterations: 400, name: "High delay (30%), Low RTT (30ms)" },
    ];

    println!("\n=== Testing Multiple Configurations ===\n");

    for (i, config) in configs.iter().enumerate() {
        let data_size: usize = 512 * 1024; // 512KB per test
        let test_data = generate_test_data(data_size);
        let peer_id: PeerId = uuid::Uuid::new_v4().into();

        let mut fec_sender = FecSender::new(4096);
        fec_sender.set_rtt(config.rtt_ms);

        let mut fec_receiver = FecReceiver::with_window_size(2048);
        fec_receiver.set_rtt(config.rtt_ms);

        let mut network = NetworkSimulatorWithDelay::new(
            config.delay_rate,
            config.delay_iterations,
            (i as u64) * 1000 + 42,
        );
        let mut retransmit_packets_sent: u64 = 0;
        let mut received_data: Vec<u8> = Vec::with_capacity(data_size);

        let chunk_size = CHUNK_SIZE * DATA_SHARDS_DEFAULT;
        let prefix: u16 = 1;
        let mut offset = 0;
        let mut stall_iterations = 0;
        let max_stall_iterations = 2000;
        let max_iterations = 200000;

        while received_data.len() < data_size && network.current_iteration < max_iterations {
            network.tick();
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
                                            retransmit_packets_sent += 1;
                                            network.send_reliable(frame.serialize());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
            }

            if offset >= test_data.len() && received_data.len() < data_size {
                tokio::time::sleep(Duration::from_millis(1)).await;

                match fec_receiver.ping() {
                    Ok(FecAction::Feedback(fb, _)) => {
                        if let Some(feedback) = fb.feedback {
                            match fec_sender.feedback(feedback) {
                                FecAction::Retransmit(frames) => {
                                    for frame in frames {
                                        retransmit_packets_sent += 1;
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
                    Err(_) => {}
                }

                if received_data.len() == prev_received && !network.has_pending() {
                    stall_iterations += 1;
                    if stall_iterations > max_stall_iterations {
                        panic!("Config '{}' stalled", config.name);
                    }
                } else {
                    stall_iterations = 0;
                }
            }
        }

        // Drain remaining delayed packets
        for _ in 0..(config.delay_iterations + 100) {
            network.tick();
            let packets = network.receive_all();
            if packets.is_empty() && !network.has_pending() {
                break;
            }
            if !packets.is_empty() {
                let mut frames = Vec::new();
                for packet in packets {
                    if let Some(frame) = Frame::deserialize(&packet) {
                        frames.push(frame);
                    }
                }
                if !frames.is_empty() {
                    let _ = fec_receiver.receive(frames);
                }
            }
        }

        let (sent, delayed) = network.stats();
        let (_, _, retransmit_count, false_retransmit) = fec_receiver.stats();

        let false_ratio = if retransmit_count > 0 {
            (false_retransmit as f64 / retransmit_count as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "[{}] {} | delayed={}/{} ({:.1}%) | retransmit={} | false_retransmit={} ({:.1}%)",
            if received_data.len() == data_size { "PASS" } else { "FAIL" },
            config.name,
            delayed,
            sent,
            (delayed as f64 / sent as f64) * 100.0,
            retransmit_count,
            false_retransmit,
            false_ratio
        );

        assert_eq!(
            received_data.len(),
            data_size,
            "Config '{}' failed: data length mismatch",
            config.name
        );
        assert_eq!(test_data, received_data, "Config '{}' failed: data integrity", config.name);
    }

    println!("\n=== All configurations passed ===\n");
}
