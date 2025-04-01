use chrono::Utc;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;
use tokio::sync::Mutex;

pub struct Instrument {
    sys: Mutex<System>,
    pid: sysinfo::Pid
}

impl Default for Instrument {
    fn default() -> Self {
        Self::new()
    }
}

impl Instrument {
    pub fn new() -> Self {
        let sys = System::new();
        let pid = sysinfo::Pid::from_u32(process::id());

        Self { sys: Mutex::new(sys), pid }
    }

    pub fn get_current_ns() -> i64 {
        Utc::now().timestamp_nanos()
    }

    pub async fn mem_log(&self) -> MemoryStats {
        let mut sys = self.sys.lock().await;
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[self.pid]), false);

        if let Some(process) = sys.process(self.pid) {
            let memory_bytes = process.memory();
            let virtual_memory_bytes = process.virtual_memory();
            let cpu_usage = process.cpu_usage();
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

            MemoryStats {
                timestamp,
                memory_mb: memory_bytes as f64 / (1024.0 * 1024.0),
                virtual_memory_mb: virtual_memory_bytes as f64 / (1024.0 * 1024.0),
                cpu_usage
            }
        } else {
            MemoryStats::default()
        }
    }
}

#[derive(Debug, Default)]
pub struct MemoryStats {
    pub timestamp: u64,
    pub memory_mb: f64,
    pub virtual_memory_mb: f64,
    pub cpu_usage: f32
}

// Example usage:
impl MemoryStats {
    pub fn to_string(&self) -> String {
        format!(
            "[{}] Memory: {:.2} MB, Virtual: {:.2} MB, CPU: {:.2}%",
            self.timestamp, self.memory_mb, self.virtual_memory_mb, self.cpu_usage
        )
    }
}
