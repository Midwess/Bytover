use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use thiserror::Error;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Debug, Error)]
pub enum SyncUdpSocketError {
    #[error("queue closed")]
    QueueClosed,
    #[error("response sender dropped")]
    SenderDropped,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error)
}

struct SendEntry {
    buf: Vec<u8>,
    target: SocketAddr,
    resp: oneshot::Sender<std::io::Result<usize>>
}

struct RecvEntry {
    addr: SocketAddr,
    resp: oneshot::Sender<std::io::Result<(Vec<u8>, SocketAddr)>>
}

fn map_to_v6(addr: SocketAddr) -> SocketAddr {
    match addr {
        SocketAddr::V4(v4) => SocketAddr::new(IpAddr::V6(v4.ip().to_ipv6_mapped()), v4.port()),
        v6 => v6
    }
}

#[derive(Clone)]
pub struct SyncUdpSocket {
    inner: Arc<Mutex<UdpSocket>>,
    send_tx: mpsc::Sender<SendEntry>,
    recv_tx: mpsc::Sender<RecvEntry>
}

impl SyncUdpSocket {
    pub fn new(socket: UdpSocket) -> Self {
        let is_v6 = socket.local_addr().map(|a| a.is_ipv6()).unwrap_or(false);
        let (send_tx, send_rx) = mpsc::channel(2048);
        let (recv_tx, recv_rx) = mpsc::channel(2048);
        let inner = Arc::new(Mutex::new(socket));
        let socket_ref = inner.clone();

        tokio::spawn(Self::poll_loop(socket_ref, send_rx, recv_rx, is_v6));

        Self { inner, send_tx, recv_tx }
    }

    async fn poll_loop(
        socket: Arc<Mutex<UdpSocket>>,
        mut send_rx: mpsc::Receiver<SendEntry>,
        mut recv_rx: mpsc::Receiver<RecvEntry>,
        is_v6: bool
    ) {
        loop {
            tokio::select! {
                Some(entry) = send_rx.recv() => {
                    let target = if is_v6 { map_to_v6(entry.target) } else { entry.target };
                    let sock = socket.lock().await;
                    let result = sock.send_to(&entry.buf, target).await;
                    let _ = entry.resp.send(result);
                }
                Some(entry) = recv_rx.recv() => {
                    let expected = if is_v6 { map_to_v6(entry.addr) } else { entry.addr };
                    let mut buf = vec![0u8; 65535];
                    let sock = socket.lock().await;
                    match sock.recv_from(&mut buf).await {
                        Ok((size, src)) if src == expected || src == entry.addr => {
                            buf.truncate(size);
                            let _ = entry.resp.send(Ok((buf, src)));
                        }
                        Ok(_) => {
                            let _ = entry.resp.send(Err(std::io::Error::new(
                                std::io::ErrorKind::UnexpectedEof,
                                "Wrong address",
                            )));
                        }
                        Err(e) => {
                            let _ = entry.resp.send(Err(e));
                        }
                    }
                }
                else => break,
            }
        }
    }

    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> Result<usize, SyncUdpSocketError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let entry = SendEntry {
            buf: buf.to_vec(),
            target,
            resp: resp_tx
        };
        self.send_tx.send(entry).await.map_err(|_| SyncUdpSocketError::QueueClosed)?;
        Ok(resp_rx.await.map_err(|_| SyncUdpSocketError::SenderDropped)??)
    }

    pub async fn recv_from(&self, addr: SocketAddr) -> Result<(Vec<u8>, SocketAddr), SyncUdpSocketError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let entry = RecvEntry { addr, resp: resp_tx };
        self.recv_tx.send(entry).await.map_err(|_| SyncUdpSocketError::QueueClosed)?;
        Ok(resp_rx.await.map_err(|_| SyncUdpSocketError::SenderDropped)??)
    }

    pub async fn recv_any(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), SyncUdpSocketError> {
        let sock = self.inner.lock().await;
        Ok(sock.recv_from(buf).await?)
    }

    pub fn local_addr_sync(&self) -> Result<SocketAddr, SyncUdpSocketError> {
        match self.inner.try_lock() {
            Ok(socket) => Ok(socket.local_addr()?),
            Err(_) => Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "socket locked").into())
        }
    }

    pub async fn local_addr(&self) -> Result<SocketAddr, SyncUdpSocketError> {
        Ok(self.inner.lock().await.local_addr()?)
    }
}

unsafe impl Send for SyncUdpSocket {}
unsafe impl Sync for SyncUdpSocket {}
