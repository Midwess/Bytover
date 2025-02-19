use ping::{ping, Error};
use std::time::{Duration, Instant};

use crate::errors::NetworkError;

#[async_trait::async_trait]
pub trait NetworkModule {
    // Check if the module is connected to the upstream
    async fn is_connected(&self) -> bool;
    // The module could try to reconnect it self, we need to wait until it is connected
    async fn wait_until_connected(&self, timeout: Duration) {
        let elapsed = Instant::now();
        while elapsed.elapsed() < timeout {
            if self.is_connected().await {
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    // Call this method will cause module to reconnect to the upstream
    // Even if it is already connected
    async fn connect(&self, timeout: Duration) -> Result<(), NetworkError>;
}

pub struct InternetConnection {}

impl InternetConnection {
    pub async fn is_connected(&self) -> bool {
        let timeout = Duration::from_millis(100);
        let packet_size: u32 = 56;
        let addr = "1.1.1.1";

        match ping(addr.parse().unwrap(), Some(timeout), Some(packet_size), None, None, None) {
            Ok(_) => {
                log::debug!("Ping successful to 1.1.1.1");
                true
            }
            Err(err) => {
                match err {
                    Error::InvalidProtocol => log::debug!("Invalid protocol"),
                    Error::InternalError => log::debug!("Internal error"),
                    Error::DecodeV4Error => log::debug!("IPv4 decode error"),
                    Error::DecodeEchoReplyError => log::debug!("Echo reply decode error"),
                    Error::IoError { error } => log::debug!("IO error: {}", error)
                }
                false
            }
        }
    }
}
