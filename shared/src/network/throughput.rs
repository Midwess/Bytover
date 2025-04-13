use std::time::Duration;

pub struct ThroughputController {
    pub max_bytes_sent: usize,
    pub receive_timeout: Duration
}

impl ThroughputController {
    pub fn new(max_bytes_sent: usize, receive_timeout: Duration) -> Self {
        Self { max_bytes_sent, receive_timeout }
    }
    
    pub fn on_sents(&self, amount: usize) {}

    pub fn on_received(&self, amount: usize) {}

    pub async fn timeout_til_next_bytes(&self) {}
}
