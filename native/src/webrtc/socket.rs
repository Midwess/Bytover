use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SyncUdpSocket {
    inner: Arc<Mutex<UdpSocket>>,
}

impl SyncUdpSocket {
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            inner: Arc::new(Mutex::new(socket)),
        }
    }

    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> std::io::Result<usize> {
        let socket = self.inner.lock().await;
        socket.send_to(buf, target).await
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        let socket = self.inner.lock().await;
        socket.recv_from(buf).await
    }

    pub fn local_addr_sync(&self) -> std::io::Result<SocketAddr> {
        // Use try_lock to avoid deadlock in sync context
        // This may fail if another async operation holds the lock
        match self.inner.try_lock() {
            Ok(socket) => socket.local_addr(),
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "socket locked",
            )),
        }
    }

    pub fn local_addr(&self) -> SocketAddr {
        // Call this from async context after getting the lock
        // Kept for API compatibility
        self.inner.blocking_lock().local_addr().unwrap()
    }
}

unsafe impl Send for SyncUdpSocket {}
unsafe impl Sync for SyncUdpSocket {}
