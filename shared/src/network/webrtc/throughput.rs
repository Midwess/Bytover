use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use tokio::sync::{broadcast, Mutex};
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
    pub ready_to_send_broadcast: Mutex<HashMap<String, broadcast::Sender<()>>>,
    pub received_broadcast: broadcast::Sender<()>
}

impl ThroughputController {
    pub fn new(max_bytes_buffer: usize, received_timeout: Duration) -> Self {
        let (received_tx, _) = broadcast::channel(16);
        Self {
            max_bytes_buffer,
            received_timeout,
            ready_to_send_broadcast: Mutex::new(HashMap::new()),
            received_broadcast: received_tx
        }
    }

    pub async fn handle(&self, channel: Weak<RTCDataChannel>) {
        if let Some(channel) = channel.upgrade() {
            let (ready_to_send_tx, _) = broadcast::channel(1);
            self.ready_to_send_broadcast
                .lock()
                .await
                .insert(channel.label().to_string(), ready_to_send_tx.clone());

            channel.set_buffered_amount_low_threshold(self.max_bytes_buffer / 2).await;
            channel
                .on_buffered_amount_low(Box::new(move || {
                    let _ = ready_to_send_tx.send(());
                    Box::pin(async move {})
                }))
                .await;
        }
    }

    pub async fn wait_buffer(&self, channel: Weak<RTCDataChannel>, sent_bytes: usize) {
        if let Some(channel) = channel.upgrade() {
            let label = channel.label().to_string();
            let current_buffer = channel.buffered_amount().await;
            if sent_bytes + current_buffer < self.max_bytes_buffer {
                return;
            }

            let mut rx = self.ready_to_send_broadcast.lock().await.get(&label).unwrap().subscribe();
            let _ = rx.recv().await;
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
