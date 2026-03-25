use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex, oneshot};

#[derive(Debug, Error)]
pub enum SyncUdpSocketError {
    #[error("queue closed")]
    QueueClosed,
    #[error("response sender dropped")]
    SenderDropped,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

const QUEUE_SIZE: usize = 2048;
const NUM_QUEUES: usize = 16;

/// Queue entry for outgoing packets
struct QueueEntry {
    buf: Vec<u8>,
    target: SocketAddr,
    resp: oneshot::Sender<std::io::Result<usize>>,
}

/// Queue entry for incoming packet requests (waiting on a specific address)
struct RecvEntry {
    addr: SocketAddr,
    resp: oneshot::Sender<std::io::Result<(Vec<u8>, SocketAddr)>>,
}

fn hash_addr(addr: &SocketAddr) -> usize {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    addr.hash(&mut h);
    (h.finish() % NUM_QUEUES as u64) as usize
}

#[derive(Clone)]
pub struct SyncUdpSocket {
    inner: Arc<Mutex<UdpSocket>>,
    queues: Arc<Vec<Mutex<mpsc::Sender<QueueEntry>>>>,
    recv_queues: Arc<Vec<Mutex<mpsc::Sender<RecvEntry>>>>,
}

impl SyncUdpSocket {
    pub fn new(socket: UdpSocket) -> Self {
        let (txs, rxs): (Vec<_>, Vec<_>) = (0..NUM_QUEUES)
            .map(|_| mpsc::channel(QUEUE_SIZE))
            .unzip();
        let (recv_txs, recv_rxs): (Vec<_>, Vec<_>) = (0..NUM_QUEUES)
            .map(|_| mpsc::channel(QUEUE_SIZE))
            .unzip();

        let inner = Arc::new(Mutex::new(socket));
        let socket_clone = inner.clone();

        // Spawn background task to poll all queues forever
        tokio::spawn(async move {
            Self::poll_loop(socket_clone, rxs, recv_rxs).await;
        });

        Self {
            inner,
            queues: Arc::new(txs.into_iter().map(Mutex::new).collect()),
            recv_queues: Arc::new(recv_txs.into_iter().map(Mutex::new).collect()),
        }
    }

    /// Background task that polls all queues forever
    async fn poll_loop(
        socket: Arc<Mutex<UdpSocket>>,
        mut queues: Vec<mpsc::Receiver<QueueEntry>>,
        mut recv_queues: Vec<mpsc::Receiver<RecvEntry>>,
    ) {
        loop {
            // Poll each send queue
            for queue in queues.iter_mut() {
                while let Ok(entry) = queue.try_recv() {
                    let QueueEntry { buf, target, resp } = entry;
                    let sock = socket.lock().await;
                    let result = sock.send_to(&buf, target).await;
                    let _ = resp.send(result);
                }
            }

            // Poll each recv queue
            for queue in recv_queues.iter_mut() {
                while let Ok(entry) = queue.try_recv() {
                    let RecvEntry { addr, resp } = entry;
                    let mut buf = vec![0u8; 65535];
                    let sock = socket.lock().await;
                    match sock.recv_from(&mut buf).await {
                        Ok((size, src)) if src == addr => {
                            buf.truncate(size);
                            let _ = resp.send(Ok((buf, src)));
                        }
                        Ok((size, src)) => {
                            // Data from wrong address - route to correct queue
                            buf.truncate(size);
                            let idx = hash_addr(&src);
                            if let Some(q) = recv_queues.get(idx) {
                                let _ = q.try_send(RecvEntry { addr: src, resp });
                            }
                        }
                        Err(e) => {
                            let _ = resp.send(Err(e));
                        }
                    }
                }
            }

            // Yield to avoid busy-spinning
            tokio::task::yield_now().await;
        }
    }

    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> Result<usize, SyncUdpSocketError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let entry = QueueEntry { buf: buf.to_vec(), target, resp: resp_tx };
        let idx = hash_addr(&target);
        let queue = self.queues[idx].lock().await;
        queue.send(entry).await.map_err(|_| SyncUdpSocketError::QueueClosed)?;
        Ok(resp_rx.await.map_err(|_| SyncUdpSocketError::SenderDropped)??)
    }

    /// Wait for a packet from the specified address
    pub async fn recv_from(&self, addr: SocketAddr) -> Result<(Vec<u8>, SocketAddr), SyncUdpSocketError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let entry = RecvEntry { addr, resp: resp_tx };
        let idx = hash_addr(&addr);
        let queue = self.recv_queues[idx].lock().await;
        queue.send(entry).await.map_err(|_| SyncUdpSocketError::QueueClosed)?;
        Ok(resp_rx.await.map_err(|_| SyncUdpSocketError::SenderDropped)??)
    }

    /// Receive any incoming packet into `buf`, returning `(bytes_read, source_addr)`.
    /// Unlike `recv_from`, this does not filter by address — suitable for ICE negotiation.
    pub async fn recv_any(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), SyncUdpSocketError> {
        let sock = self.inner.lock().await;
        Ok(sock.recv_from(buf).await?)
    }

    pub fn local_addr_sync(&self) -> Result<SocketAddr, SyncUdpSocketError> {
        match self.inner.try_lock() {
            Ok(socket) => Ok(socket.local_addr()?),
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "socket locked").into()),
        }
    }

    pub fn local_addr(&self) -> Result<SocketAddr, SyncUdpSocketError> {
        Ok(self.inner.blocking_lock().local_addr()?)
    }
}

unsafe impl Send for SyncUdpSocket {}
unsafe impl Sync for SyncUdpSocket {}
