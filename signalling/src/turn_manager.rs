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
    pub public_ipv4: Option<String>,
    pub public_ipv6: Option<String>,
    pub relay_host: String,
    pub stun_port: u16,
    pub relay_port: u16,
    pub turn_port: u16,
    pub turn_username: Option<String>,
    pub turn_password: Option<String>,
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
                            "Relay {} (IPv4: {:?}, IPv6: {:?}, host: {}) timed out, removing",
                            id,
                            relay.public_ipv4,
                            relay.public_ipv6,
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

    pub async fn register_relay(&self, public_ipv4: Option<String>, public_ipv6: Option<String>, stun_port: u16, relay_port: u16, turn_port: u16, turn_username: Option<String>, turn_password: Option<String>) {
        let mut relays = self.relays.lock().await;
        let relay_host = public_ipv4
            .clone()
            .or(public_ipv6.clone())
            .expect("relay registration requires at least one public IP");

        let existing_id = relays
            .values()
            .find(|r| {
                r.public_ipv4 == public_ipv4 &&
                    r.public_ipv6 == public_ipv6 &&
                    r.relay_host == relay_host &&
                    r.stun_port == stun_port &&
                    r.relay_port == relay_port &&
                    r.turn_port == turn_port
            })
            .map(|r| r.id.clone());

        if let Some(id) = existing_id {
            if let Some(relay) = relays.get_mut(&id) {
                relay.last_ping = Instant::now();
            }
        } else {
            let id = Uuid::new_v4().to_string();
            log::info!(
                "New relay registered: public_ipv4={:?} public_ipv6={:?} host={} (ID: {}, STUN: {}, Relay: {}, TURN: {})",
                public_ipv4,
                public_ipv6,
                relay_host,
                id,
                stun_port,
                relay_port,
                turn_port
            );
            relays.insert(
                id.clone(),
                RegisteredRelay {
                    id,
                    public_ipv4,
                    public_ipv6,
                    relay_host,
                    stun_port,
                    relay_port,
                    turn_port,
                    turn_username,
                    turn_password,
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
        let mut urls = Vec::new();

        if let Some(public_ipv4) = final_relay.public_ipv4.as_ref() {
            urls.push(format!("stun:{}:{}", public_ipv4, final_relay.stun_port));
        }

        if let Some(public_ipv6) = final_relay.public_ipv6.as_ref() {
            urls.push(format!("stun:[{}]:{}", public_ipv6, final_relay.stun_port));
        }

        if final_relay.turn_port > 0 {
            if let Some(public_ipv4) = final_relay.public_ipv4.as_ref() {
                urls.push(format!("turn:{}:{}", public_ipv4, final_relay.turn_port));
            }

            if let Some(public_ipv6) = final_relay.public_ipv6.as_ref() {
                urls.push(format!("turn:[{}]:{}", public_ipv6, final_relay.turn_port));
            }
        }

        Some(IceConfig {
            urls,
            username: final_relay.turn_username.clone(),
            credential: final_relay.turn_password.clone()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TurnManager;

    #[tokio::test]
    async fn assigned_relay_keeps_registered_relay_port() {
        let manager = TurnManager::new().await;
        manager
            .register_relay(Some("198.51.100.10".to_string()), Some("2001:db8::10".to_string()), 3478, 19101, 19101)
            .await;

        let relay = manager.get_assigned_relay("client-1").await.unwrap();

        assert_eq!(relay.public_ipv4.as_deref(), Some("198.51.100.10"));
        assert_eq!(relay.public_ipv6.as_deref(), Some("2001:db8::10"));
        assert_eq!(relay.relay_host, "198.51.100.10");
        assert_eq!(relay.stun_port, 3478);
        assert_eq!(relay.relay_port, 19101);
    }

    #[tokio::test]
    async fn relay_config_publishes_dual_stack_urls() {
        let manager = TurnManager::new().await;
        manager
            .register_relay(Some("198.51.100.10".to_string()), Some("2001:db8::10".to_string()), 3478, 19101, 19101)
            .await;

        let relay = manager.get_relay_config("client-1").await.unwrap();

        assert_eq!(
            relay.urls,
            vec![
                "stun:198.51.100.10:3478".to_string(),
                "stun:[2001:db8::10]:3478".to_string()
            ]
        );
    }
}
