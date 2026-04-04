use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Instant, Duration};
use tokio::sync::Mutex;
use uuid::Uuid;
use schema::devlog::rpc_signalling::server::IceConfig;

#[derive(Debug, Clone)]
pub struct RegisteredRelay {
    pub id: String,
    pub ip: String,
    pub stun_port: u16,
    pub relay_port: u16,
    pub last_ping: Instant,
    pub counter: Arc<AtomicUsize>,
}

pub struct TurnManager {
    relays: Arc<Mutex<HashMap<String, RegisteredRelay>>>, // relay_id -> RegisteredRelay
    client_relay_assignments: Arc<Mutex<HashMap<String, String>>>, // client_id -> relay_id
}

impl TurnManager {
    pub async fn new() -> Self {
        let manager = Self {
            relays: Arc::new(Mutex::new(HashMap::new())),
            client_relay_assignments: Arc::new(Mutex::new(HashMap::new())),
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
                        log::info!("Relay {} (IP: {}) timed out, removing", id, relay.ip);
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

    pub async fn register_relay(&self, ip: String, stun_port: u16, relay_port: u16) {
        let mut relays = self.relays.lock().await;
        
        // Use composite characteritics to find an existing relay instance
        let existing_id = relays.values()
            .find(|r| r.ip == ip && r.stun_port == stun_port && r.relay_port == relay_port)
            .map(|r| r.id.clone());

        if let Some(id) = existing_id {
            if let Some(relay) = relays.get_mut(&id) {
                relay.last_ping = Instant::now();
            }
        } else {
            let id = Uuid::new_v4().to_string();
            log::info!("New relay registered: {} (ID: {}, STUN: {}, Relay: {})", ip, id, stun_port, relay_port);
            relays.insert(id.clone(), RegisteredRelay {
                id,
                ip,
                stun_port,
                relay_port,
                last_ping: Instant::now(),
                counter: Arc::new(AtomicUsize::new(0)),
            });
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

    pub async fn get_relay_config(&self, client_id: &str) -> Option<IceConfig> {
        let mut assignments = self.client_relay_assignments.lock().await;
        let relays = self.relays.lock().await;

        let assigned_relay_id = assignments.get(client_id);
        
        // Check if an existing assignment is still healthy
        let healthy_relay = if let Some(id) = assigned_relay_id {
            relays.get(id).cloned()
        } else {
            None
        };

        let final_relay = match healthy_relay {
            Some(r) => r,
            None => {
                // Initial assignment or reassigning if the previous relay is gone
                let best_relay = relays.values()
                    .min_by_key(|r| r.counter.load(Ordering::Relaxed))?
                    .clone();
                
                best_relay.counter.fetch_add(1, Ordering::Relaxed);
                assignments.insert(client_id.to_string(), best_relay.id.clone());
                best_relay
            }
        };

        let ip_url = if final_relay.ip.contains(':') {
            format!("[{}]", final_relay.ip)
        } else {
            final_relay.ip.clone()
        };

        Some(IceConfig {
            urls: vec![format!("stun:{}:{}", ip_url, final_relay.stun_port)],
            username: None,
            credential: None,
        })
    }
}