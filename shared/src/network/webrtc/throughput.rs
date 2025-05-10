use std::sync::{Arc, Weak};
use std::time::Duration;

use bytes::{Buf, Bytes};
use futures_util::StreamExt;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex, OnceCell};
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
            // Max size of message is 64KB, we expect the speed must at least 10KB/s
            send_timeout: Duration::from_millis(6400),
            received_broadcast: received_tx,
            max_concurrent_sends,
            sent_queue: OnceCell::new()
        }
    }

    pub fn start(&self) {
        let (sent_tx, sent_rx) = mpsc::channel(1024);
        let _ = self.sent_queue.set(sent_tx);

        let sent_rx = Arc::new(Mutex::new(sent_rx));
        tokio_scoped::scope(|scope| {
            for _ in 0..self.max_concurrent_sends {
                let sent_rx = sent_rx.clone();
                scope.spawn(async move {
                    while let Some(request) = sent_rx.lock().await.recv().await {
                        let result = self.send_by_channel(request.channel, &request.bytes).await;
                        if let Err(err) = request.tx.send(result) {
                            log::warn!(target: "throughput-controller", "Failed to send result to the channel: {:?}", err);
                        }
                    }
                });
            }
        });
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

    async fn send_by_channel(&self, channel: Weak<RTCDataChannel>, bytes: &Bytes) -> Result<usize, DataChannelError> {
        if let Some(channel) = channel.upgrade() {
            const CHUNK_SIZE: usize = 63 * 1024;
            let mut total_sent = 0;
            
            if bytes.len() <= CHUNK_SIZE {
                let sent_bytes = timeout(self.send_timeout, channel.send(bytes))
                    .await
                    .map_err(|_| DataChannelError::Timeout(self.send_timeout))??;
                self.wait_buffer(Arc::downgrade(&channel), sent_bytes).await;
                total_sent = sent_bytes;
            } else {
                let chunks = bytes.chunks(CHUNK_SIZE);
                let mut remaining_chunks = chunks.collect::<Vec<_>>();
                while let Some(chunk) = remaining_chunks.pop() {
                    let sent_bytes = timeout(self.send_timeout, channel.send(&Bytes::from(chunk.to_vec())))
                        .await
                        .map_err(|_| DataChannelError::Timeout(self.send_timeout))??;
                    self.wait_buffer(Arc::downgrade(&channel), sent_bytes).await;
                    total_sent += sent_bytes;
                }
            }
            
            Ok(total_sent)
        } else {
            Err(DataChannelError::DataChannelClosed("Channel already deallocated".to_string()))
        }
    }

    pub async fn send(&self, channel: Weak<RTCDataChannel>, bytes: &Bytes) -> Result<usize, DataChannelError> {
        let Some(sent_tx) = self.sent_queue.get() else {
            return Err(DataChannelError::DataChannelClosed(
                "The throughput controller is not started".to_string()
            ));
        };

        let (request, rx) = SendRequest::new(bytes.clone(), channel);

        if let Err(err) = sent_tx.send(request).await {
            return Err(DataChannelError::ThroughputController(err.to_string()));
        }

        let sent_bytes = rx
            .await
            .map_err(|_| DataChannelError::ThroughputController("The channel is closed".to_string()))??;

        Ok(sent_bytes)
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
