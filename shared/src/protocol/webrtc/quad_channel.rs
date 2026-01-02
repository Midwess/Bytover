use std::sync::atomic::{AtomicU8, Ordering};
use crate::protocol::webrtc::errors::WebRtcErrors;
use crate::shell::api::BufferExt;
use futures::channel::mpsc;
use matchbox_protocol::PeerId;
use matchbox_socket::{Packet, PeerBuffered};
use std::time::Duration;

pub struct QuadUnreliableChannel {
    channels: [mpsc::UnboundedSender<(PeerId, Packet)>; 4],
    channel_ids: [usize; 4],
    buffer: PeerBuffered,
    current_channel: AtomicU8
}

impl QuadUnreliableChannel {
    pub fn new(
        channel1: mpsc::UnboundedSender<(PeerId, Packet)>,
        channel2: mpsc::UnboundedSender<(PeerId, Packet)>,
        channel3: mpsc::UnboundedSender<(PeerId, Packet)>,
        channel4: mpsc::UnboundedSender<(PeerId, Packet)>,
        channel1_id: usize,
        channel2_id: usize,
        channel3_id: usize,
        channel4_id: usize,
        buffer: PeerBuffered
    ) -> Self {
        Self {
            channels: [
                channel1, channel2, channel3, channel4
            ],
            channel_ids: [
                channel1_id,
                channel2_id,
                channel3_id,
                channel4_id
            ],
            buffer,
            current_channel: AtomicU8::new(0)
        }
    }

    pub fn send(&self, peer_id: PeerId, packet: Packet) -> Result<(), mpsc::TrySendError<(PeerId, Packet)>> {
        let channel_index = self.current_channel.load(Ordering::Relaxed);
        let result = self.channels[channel_index as usize].unbounded_send((peer_id, packet));
        self.current_channel.store((channel_index + 1) % 4, Ordering::Relaxed);
        result
    }

    pub async fn wait_buffer_low(&self, min_buffer_size: usize, timeout: Duration) {
        for &channel_id in &self.channel_ids {
            self.buffer.wait_buffer_low(channel_id, min_buffer_size, timeout).await;
        }
    }

    pub async fn bytes_sent_received(&self) -> (usize, usize) {
        let mut total_sent = 0;
        let mut total_received = 0;

        for &channel_id in &self.channel_ids {
            let (sent, received) = self.buffer.channel_bytes_sent_received(channel_id).await.unwrap_or((0, 0));
            total_sent += sent;
            total_received += received;
        }

        (total_sent, total_received)
    }

    pub async fn bytes_sent(&self) -> usize {
        let mut total_sent = 0;

        for &channel_id in &self.channel_ids {
            let sent = self.buffer.channel_bytes_sent_received(channel_id).await.map(|it| it.0).unwrap_or(0);
            total_sent += sent;
        }

        total_sent
    }

    pub async fn flush_timeout(&self) -> Result<(), WebRtcErrors> {
        for &channel_id in &self.channel_ids {
            self.buffer.flush_timeout(channel_id).await?;
        }
        Ok(())
    }
}
