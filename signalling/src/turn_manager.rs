use schema::devlog::rpc_signalling::server::IceConfig;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RegisteredRelay {
    pub id: String,
    pub public_ip: String,
    pub relay_host: String,
    pub stun_port: u16,
    pub relay_port: u16,
    pub last_ping: Instant,
    pub counter: Arc<AtomicUsize>
}

pub struct TurnManager {
    relays: Arc<Mutex<HashMap<String, RegisteredRelay>>>, // relay_id -> RegisteredRelay
    client_relay_assignments: Arc<Mutex<HashMap<String, String>>>  // client_id -> relay_id
}

impl TurnManager {
    pub async fn new() -> Self {
        let manager = Self {
            relays: Arc::new(Mutex::new(HashMap::new())),
            client_relay_assignments: Arc::new(Mutex::new(HashMap::new()))
        };

        // Start background task to prune expired relays
        let relays_clone = Arc::clone(&manager.relays);
        let assignments_clone = Arc::clone(&manager.client_relay_assignments);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let mut relays = relays_clone.lock().await;
                let now = Instant::now();

                // Identify IDs of relays that will be removed
                let mut removed_ids = Vec::new();
                relays.retain(|id, relay| {
                    if now.duration_since(relay.last_ping) > Duration::from_secs(30) {
                        log::info!(
                            "Relay {} (IP: {}, host: {}) timed out, removing",
                            id,
                            relay.public_ip,
                            relay.relay_host
                        );
                        removed_ids.push(id.clone());
                        false
                    } else {
                        true
                    }
                });

                // Clean up any assignments pointing to those relays
                if !removed_ids.is_empty() {
                    let mut assignments = assignments_clone.lock().await;
                    assignments.retain(|_, relay_id| !removed_ids.contains(relay_id));
                }
            }
        });

        manager
    }

    pub async fn register_relay(&self, public_ip: String, relay_host: String, stun_port: u16, relay_port: u16) {
        let mut relays = self.relays.lock().await;

        // Use composite characteritics to find an existing relay instance
        let existing_id = relays
            .values()
            .find(|r| r.public_ip == public_ip && r.relay_host == relay_host && r.stun_port == stun_port && r.relay_port == relay_port)
            .map(|r| r.id.clone());

        if let Some(id) = existing_id {
            if let Some(relay) = relays.get_mut(&id) {
                relay.last_ping = Instant::now();
            }
        } else {
            let id = Uuid::new_v4().to_string();
            log::info!(
                "New relay registered: public_ip={} host={} (ID: {}, STUN: {}, Relay: {})",
                public_ip,
                relay_host,
                id,
                stun_port,
                relay_port
            );
            relays.insert(
                id.clone(),
                RegisteredRelay {
                    id,
                    public_ip,
                    relay_host,
                    stun_port,
                    relay_port,
                    last_ping: Instant::now(),
                    counter: Arc::new(AtomicUsize::new(0))
                }
            );
        }
    }

    pub async fn unregister_client(&self, client_id: &str) {
        let mut assignments = self.client_relay_assignments.lock().await;
        if let Some(relay_id) = assignments.remove(client_id) {
            let relays = self.relays.lock().await;
            if let Some(relay) = relays.get(&relay_id) {
                relay.counter.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }

    pub async fn get_assigned_relay(&self, client_id: &str) -> Option<RegisteredRelay> {
        let mut assignments = self.client_relay_assignments.lock().await;
        let relays = self.relays.lock().await;

        let assigned_relay_id = assignments.get(client_id);

        // Check if an existing assignment is still healthy
        let healthy_relay = if let Some(id) = assigned_relay_id {
            relays.get(id).cloned()
        } else {
            None
        };

        Some(match healthy_relay {
            Some(r) => r,
            None => {
                // Initial assignment or reassigning if the previous relay is gone
                let best_relay = relays.values().min_by_key(|r| r.counter.load(Ordering::Relaxed))?.clone();

                best_relay.counter.fetch_add(1, Ordering::Relaxed);
                assignments.insert(client_id.to_string(), best_relay.id.clone());
                best_relay
            }
        })
    }

    pub async fn get_relay_config(&self, client_id: &str) -> Option<IceConfig> {
        let final_relay = self.get_assigned_relay(client_id).await?;

        let ip_url = if final_relay.public_ip.contains(':') {
            format!("[{}]", final_relay.public_ip)
        } else {
            final_relay.public_ip.clone()
        };

        Some(IceConfig {
            urls: vec![format!(
                "stun:{}:{}",
                ip_url, final_relay.stun_port
            )],
            username: None,
            credential: None
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TurnManager;

    #[tokio::test]
    async fn assigned_relay_keeps_registered_relay_port() {
        let manager = TurnManager::new().await;
        manager.register_relay("198.51.100.10".to_string(), "127.0.0.1".to_string(), 3478, 19101).await;

        let relay = manager.get_assigned_relay("client-1").await.unwrap();

        assert_eq!(relay.public_ip, "198.51.100.10");
        assert_eq!(relay.relay_host, "127.0.0.1");
        assert_eq!(relay.stun_port, 3478);
        assert_eq!(relay.relay_port, 19101);
    }
}
