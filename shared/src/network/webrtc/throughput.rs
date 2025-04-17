use std::sync::{Arc, Weak};
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use tokio::sync::broadcast;
use tokio::task::yield_now;
use tokio::time::{sleep, timeout};
use webrtc::data_channel::RTCDataChannel;

use super::data_channel::{DataChannelError, RTCStreamChannel};

#[derive(Debug, thiserror::Error)]
pub enum ThroughputError {
    #[error("Timeout")]
    Timeout(Duration),
    #[error("Channel closed")]
    ChannelClosed
}

pub struct ThroughputController {
    pub max_bytes_buffer: usize,
    pub received_timeout: Duration,
    pub received_broadcast: broadcast::Sender<()>
}

impl ThroughputController {
    pub fn new(max_bytes_buffer: usize, received_timeout: Duration) -> Self {
        let (received_tx, _) = broadcast::channel(2048);
        Self {
            max_bytes_buffer,
            received_timeout,
            received_broadcast: received_tx
        }
    }

    pub async fn wait_buffer(&self, channel: Weak<RTCDataChannel>, sent_bytes: usize) {
        while let Some(channel) = channel.upgrade() {
            let current_buffer = channel.buffered_amount().await;
            if sent_bytes + current_buffer < self.max_bytes_buffer {
                return;
            }

            yield_now().await;
        }
    }

    fn on_received(&self) {
        let _ = self.received_broadcast.send(());
    }

    pub async fn send(&self, channel: Weak<RTCDataChannel>, bytes: &Bytes) -> Result<usize, DataChannelError> {
        let send_timeout = Duration::from_secs(10);
        if let Some(channel) = channel.upgrade() {
            let sent_bytes = timeout(send_timeout, channel.send(bytes))
                .await
                .map_err(|_| DataChannelError::Timeout(send_timeout))??;
            self.wait_buffer(Arc::downgrade(&channel), sent_bytes).await;
            Ok(sent_bytes)
        } else {
            Err(DataChannelError::DataChannelClosed("Channel already deallocated".to_string()))
        }
    }

    pub async fn next_bytes(&self, stream: &mut RTCStreamChannel) -> Result<Option<Vec<u8>>, DataChannelError> {
        let mut rx = self.received_broadcast.subscribe();

        loop {
            let sleep_task = sleep(self.received_timeout);
            tokio::pin!(sleep_task);

            tokio::select! {
                result = stream.next() => {
                    if let Some(Ok(result)) = result {
                        self.on_received();
                        return Ok(Some(result));
                    }

                    if let Some(Err(error)) = result {
                        return Err(error);
                    }

                    return Ok(None);
                },
                _ = &mut sleep_task => {
                    return Err(DataChannelError::Timeout(self.received_timeout));
                },
                _ = rx.recv() => {
                    continue;
                }
            }
        }
    }
}
