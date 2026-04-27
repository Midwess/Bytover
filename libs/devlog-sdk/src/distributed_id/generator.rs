use chrono::{DateTime, Utc};
use futures_timer::Delay;
use std::time::Duration;
use thiserror::Error;

pub const EPOCH_SINCE: u64 = 1735405200000; // 2024-12-29T00:00:00Z

pub const MAX_ID_IN_MS: u64 = 1024;

pub const MAX_WORKER_ID: u16 = (1 << 13) - 1;

#[derive(Error, Debug)]
pub enum DistributedIdError {
    #[error("Invalid coordinator configuration: {0}")]
    InvalidConfig(String),
    #[error("Coordinator error: {0}")]
    Coordinator(String),
    #[error("Worker id space exhausted in namespace {0}")]
    WorkerIdsExhausted(String)
}

#[derive(Debug, Clone)]
pub struct Worker {
    pub distributed_id: u64,
    pub name: String,
    pub created_at: DateTime<Utc>
}

#[derive(Debug)]
pub struct DistributedIdGenerator {
    pub worker: Worker,
    pub last_timestamp: u64,
    // Sequence counter for IDs generated in that same ms
    pub sequence: u64
}

impl DistributedIdGenerator {
    pub fn from_worker(worker: Worker) -> Self {
        Self {
            worker,
            last_timestamp: 0,
            sequence: 0
        }
    }

    pub fn init_scoped(app_name: String) -> Self {
        Self {
            worker: Worker {
                distributed_id: (MAX_WORKER_ID - 1) as u64,
                name: app_name.clone(),
                created_at: Utc::now()
            },
            last_timestamp: 0,
            sequence: 0
        }
    }

    pub fn init_scoped_with_id(id: u32, name: String) -> Self {
        Self {
            worker: Worker {
                distributed_id: id as u64,
                name: name.clone(),
                created_at: Utc::now()
            },
            last_timestamp: 0,
            sequence: 0
        }
    }

    /// Generates a 64-bit ID with the layout:
    /// [41 bits: timestamp in ms since EPOCH_SINCE]
    /// [13 bits: worker ID]
    /// [10 bits: sequence for that ms]
    pub async fn next_id(&mut self) -> Result<u64, DistributedIdError> {
        let now_utc = Utc::now();
        let current_ms = now_utc.timestamp_millis();

        let mut timestamp = current_ms as u64 - EPOCH_SINCE;

        if timestamp == self.last_timestamp {
            self.sequence += 1;

            if self.sequence >= MAX_ID_IN_MS {
                // Wait until we move to the next millisec
                while timestamp == self.last_timestamp {
                    Delay::new(Duration::from_millis(1)).await;
                    let new_now_utc = Utc::now();
                    let new_ms = new_now_utc.timestamp_millis() as u64 - EPOCH_SINCE;
                    if new_ms > timestamp {
                        timestamp = new_ms;
                        break;
                    }
                }

                self.sequence = 0;
                self.last_timestamp = timestamp;
            }
        } else {
            self.sequence = 0;
            self.last_timestamp = timestamp;
        }

        // Construct the 64-bit ID
        // 41 bits of timestamp | 13 bits of worker | 10 bits of sequence
        let timestamp_41 = timestamp & ((1 << 41) - 1);
        let worker_13 = self.worker.distributed_id & ((1 << 13) - 1);
        let seq_10 = self.sequence & ((1 << 10) - 1);

        let id = (timestamp_41 << 23) | (worker_13 << 10) | seq_10;

        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_next_id_increments() -> Result<(), Box<dyn std::error::Error>> {
        let worker = Worker {
            distributed_id: 42,
            name: "test_worker".to_string(),
            created_at: Utc::now()
        };

        let mut generator = DistributedIdGenerator {
            worker,
            last_timestamp: 0,
            sequence: 0
        };

        let first_id = generator.next_id().await?;
        let second_id = generator.next_id().await?;

        assert!(second_id > first_id, "second_id should be greater than first_id");

        let mut prev_id = second_id;
        for _ in 0..10 {
            let next = generator.next_id().await?;
            assert!(next > prev_id, "IDs should be strictly increasing");
            prev_id = next;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_sequence_rollover() -> Result<(), Box<dyn std::error::Error>> {
        let worker = Worker {
            distributed_id: 1,
            name: "rollover_test".to_string(),
            created_at: Utc::now()
        };

        let mut generator = DistributedIdGenerator {
            worker,
            last_timestamp: 1000,       // pretend we're at "second 1000"
            sequence: MAX_ID_IN_MS - 1  // about to overflow
        };

        let first_id = generator.next_id().await?;
        assert_eq!(generator.sequence, 0, "sequence should have reset to 0");
        assert!(generator.last_timestamp >= 1000, "timestamp should move forward if we waited");

        let second_id = generator.next_id().await?;
        assert!(second_id > first_id, "IDs should keep increasing even after rollover");

        Ok(())
    }
}
