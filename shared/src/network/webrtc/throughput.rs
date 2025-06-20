use std::sync::{Arc, Weak};
use std::time::Duration;

use bytes::Bytes;
use futures_util::future::join_all;
use futures_util::StreamExt;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex, OnceCell};
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

struct SendRequest {
    bytes: Bytes,
    channel: Weak<RTCDataChannel>,
    tx: oneshot::Sender<Result<usize, DataChannelError>>
}

impl SendRequest {
    pub fn new(bytes: Bytes, channel: Weak<RTCDataChannel>) -> (Self, oneshot::Receiver<Result<usize, DataChannelError>>) {
        let (tx, rx) = oneshot::channel();
        (Self { bytes, channel, tx }, rx)
    }
}

pub struct ThroughputController {
    pub max_bytes_buffer: usize,
    pub received_timeout: Duration,
    pub received_broadcast: broadcast::Sender<()>,
    pub send_timeout: Duration,
    max_concurrent_sends: usize,
    sent_queue: OnceCell<mpsc::Sender<SendRequest>>
}

impl ThroughputController {
    pub fn new(max_bytes_buffer: usize, received_timeout: Duration, max_concurrent_sends: usize) -> Self {
        let (received_tx, _) = broadcast::channel(16);
        Self {
            max_bytes_buffer,
            received_timeout,
            send_timeout: Duration::from_millis(6400),
            received_broadcast: received_tx,
            max_concurrent_sends,
            sent_queue: OnceCell::new()
        }
    }

    pub async fn start(&self) {
        let (sent_tx, sent_rx) = mpsc::channel(5);
        let _ = self.sent_queue.set(sent_tx);

        let sent_rx = Arc::new(Mutex::new(sent_rx));
        let mut futures = vec![];
        for _ in 0..self.max_concurrent_sends {
            let sent_rx = sent_rx.clone();
            futures.push(async move {
                while let Some(request) = sent_rx.lock().await.recv().await {
                    let result = self.send_by_channel(request.channel, request.bytes).await;
                    let _ = request.tx.send(result);
                }
            });
        }

        let _ = join_all(futures).await;
    }

    pub async fn wait_buffer(&self, channel: Weak<RTCDataChannel>, sent_bytes: usize) {
        while let Some(channel) = channel.upgrade() {
            let current_buffer = channel.buffered_amount().await;
            if sent_bytes + current_buffer < self.max_bytes_buffer {
                return;
            }

            let sleep_duration = ((current_buffer as u64) / 10240).clamp(1, 10);
            tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
        }
    }

    fn on_received(&self) {
        let _ = self.received_broadcast.send(());
    }

    async fn send_by_channel(&self, channel: Weak<RTCDataChannel>, bytes: Bytes) -> Result<usize, DataChannelError> {
        if let Some(channel) = channel.upgrade() {
            const CHUNK_SIZE: usize = 63 * 1024;
            let mut total_sent = 0;
            let mut remaining_bytes = bytes;

            while !remaining_bytes.is_empty() {
                let chunk_size = remaining_bytes.len().min(CHUNK_SIZE);
                let chunk = remaining_bytes.split_to(chunk_size);

                let sent_bytes = timeout(self.send_timeout, channel.send(&chunk))
                    .await
                    .map_err(|_| DataChannelError::Timeout(self.send_timeout))??;

                self.wait_buffer(Arc::downgrade(&channel), sent_bytes).await;
                total_sent += sent_bytes;
            }

            Ok(total_sent)
        } else {
            Err(DataChannelError::DataChannelCorrupted(
                "Channel already deallocated".to_string()
            ))
        }
    }

    pub async fn send(
        &self,
        channel: Weak<RTCDataChannel>,
        bytes: &Bytes
    ) -> Result<oneshot::Receiver<Result<usize, DataChannelError>>, DataChannelError> {
        let Some(sent_tx) = self.sent_queue.get() else {
            return Err(DataChannelError::DataChannelCorrupted(
                "The throughput controller is not started".to_string()
            ));
        };

        let (request, rx) = SendRequest::new(bytes.clone(), channel);

        if let Err(err) = sent_tx.send(request).await {
            return Err(DataChannelError::ThroughputController(err.to_string()));
        }

        Ok(rx)
    }

    pub async fn next_bytes(&self, stream: &mut RTCStreamChannel) -> Result<Option<Vec<u8>>, DataChannelError> {
        let mut rx = self.received_broadcast.subscribe();

        loop {
            let sleep_task = sleep(self.received_timeout);
            tokio::pin!(sleep_task);

            tokio::select! {
                result = stream.next() => {
                    if let Some(Ok(Some(result))) = result {
                        self.on_received();
                        return Ok(Some(result));
                    }

                    if let Some(Err(error)) = result {
                        return Err(error);
                    }

                    if let Some(Ok(None)) = result {
                        return Ok(None);
                    }

                    return Err(DataChannelError::DataChannelCorrupted("The data channel is closed".to_string()));
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
