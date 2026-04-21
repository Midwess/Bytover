use schema::devlog::rpc_signalling::server::IceConfig;
use std::collections::{HashMap, HashSet};
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
    pub counter: Arc<AtomicUsize>,
}

pub struct TurnManager {
    relays: Arc<Mutex<HashMap<String, RegisteredRelay>>>,
    client_relay_assignments: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl TurnManager {
    pub async fn new() -> Self {
        let manager = Self {
            relays: Arc::new(Mutex::new(HashMap::new())),
            client_relay_assignments: Arc::new(Mutex::new(HashMap::new())),
        };

        let relays_clone = Arc::clone(&manager.relays);
        let assignments_clone = Arc::clone(&manager.client_relay_assignments);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let mut relays = relays_clone.lock().await;
                let now = Instant::now();

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

                if !removed_ids.is_empty() {
                    let mut assignments = assignments_clone.lock().await;
                    assignments.retain(|_, relay_ids| {
                        relay_ids.retain(|id| !removed_ids.contains(id));
                        !relay_ids.is_empty()
                    });
                }
            }
        });

        manager
    }

    pub async fn register_relay(
        &self,
        public_ipv4: Option<String>,
        public_ipv6: Option<String>,
        stun_port: u16,
        relay_port: u16,
        turn_port: u16,
        turn_username: Option<String>,
        turn_password: Option<String>,
    ) {
        let mut relays = self.relays.lock().await;
        let relay_host = public_ipv4
            .clone()
            .or(public_ipv6.clone())
            .expect("relay registration requires at least one public IP");

        let existing_id = relays
            .values()
            .find(|r| {
                r.public_ipv4 == public_ipv4
                    && r.public_ipv6 == public_ipv6
                    && r.relay_host == relay_host
                    && r.stun_port == stun_port
                    && r.relay_port == relay_port
                    && r.turn_port == turn_port
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
                    counter: Arc::new(AtomicUsize::new(0)),
                },
            );
        }
    }

    pub async fn unregister_client(&self, client_id: &str) {
        let mut assignments = self.client_relay_assignments.lock().await;
        if let Some(relay_ids) = assignments.remove(client_id) {
            let relays = self.relays.lock().await;
            for relay_id in relay_ids {
                if let Some(relay) = relays.get(&relay_id) {
                    relay.counter.fetch_sub(1, Ordering::Relaxed);
                }
            }
        }
    }

    pub async fn get_assigned_relay(&self, client_id: &str) -> Option<RegisteredRelay> {
        self.get_assigned_relays(client_id, 1).await.into_iter().next()
    }

    pub async fn get_assigned_relays(&self, client_id: &str, n: usize) -> Vec<RegisteredRelay> {
        let n = n.max(1);
        let mut assignments = self.client_relay_assignments.lock().await;
        let relays = self.relays.lock().await;

        let existing_ids: Vec<String> = assignments.get(client_id).cloned().unwrap_or_default();
        let mut picked: Vec<RegisteredRelay> = existing_ids
            .iter()
            .filter_map(|id| relays.get(id).cloned())
            .collect();

        if picked.len() >= n {
            picked.truncate(n);
            assignments.insert(client_id.to_string(), picked.iter().map(|r| r.id.clone()).collect());
            return picked;
        }

        let already: HashSet<String> = picked.iter().map(|r| r.id.clone()).collect();
        let mut candidates: Vec<RegisteredRelay> = relays
            .values()
            .filter(|r| !already.contains(&r.id))
            .cloned()
            .collect();
        candidates.sort_by_key(|r| r.counter.load(Ordering::Relaxed));

        let need = n - picked.len();
        let take = need.min(candidates.len());
        let available = picked.len() + take;

        if available < n {
            log::warn!(
                "connection fanout requested n={} but only {} distinct relays available for client {}",
                n,
                available,
                client_id
            );
        }

        for relay in candidates.into_iter().take(take) {
            relay.counter.fetch_add(1, Ordering::Relaxed);
            picked.push(relay);
        }

        assignments.insert(client_id.to_string(), picked.iter().map(|r| r.id.clone()).collect());
        picked
    }

    pub async fn get_relay_config(&self, client_id: &str) -> Option<IceConfig> {
        self.get_relay_configs(client_id, 1).await.into_iter().next()
    }

    pub async fn get_relay_configs(&self, client_id: &str, n: usize) -> Vec<IceConfig> {
        self.get_assigned_relays(client_id, n)
            .await
            .into_iter()
            .map(relay_to_ice_config)
            .collect()
    }
}

fn relay_to_ice_config(relay: RegisteredRelay) -> IceConfig {
    let mut urls = Vec::new();

    if let Some(public_ipv4) = relay.public_ipv4.as_ref() {
        urls.push(format!("stun:{}:{}", public_ipv4, relay.stun_port));
    }

    if let Some(public_ipv6) = relay.public_ipv6.as_ref() {
        urls.push(format!("stun:[{}]:{}", public_ipv6, relay.stun_port));
    }

    if relay.turn_port > 0 {
        if let Some(public_ipv4) = relay.public_ipv4.as_ref() {
            urls.push(format!("turn:{}:{}", public_ipv4, relay.turn_port));
        }

        if let Some(public_ipv6) = relay.public_ipv6.as_ref() {
            urls.push(format!("turn:[{}]:{}", public_ipv6, relay.turn_port));
        }
    }

    IceConfig {
        urls,
        username: relay.turn_username,
        credential: relay.turn_password,
    }
}

#[cfg(test)]
mod tests {
    use super::TurnManager;

    async fn register(manager: &TurnManager, ipv4: &str, port: u16) {
        manager
            .register_relay(
                Some(ipv4.to_string()),
                None,
                port,
                port,
                port,
                Some("relay".to_string()),
                Some("relay-secret".to_string()),
            )
            .await;
    }

    #[tokio::test]
    async fn assigned_relay_keeps_registered_relay_port() {
        let manager = TurnManager::new().await;
        manager
            .register_relay(
                Some("198.51.100.10".to_string()),
                Some("2001:db8::10".to_string()),
                19101,
                19101,
                19101,
                Some("relay".to_string()),
                Some("relay-secret".to_string()),
            )
            .await;

        let relay = manager.get_assigned_relay("client-1").await.unwrap();

        assert_eq!(relay.public_ipv4.as_deref(), Some("198.51.100.10"));
        assert_eq!(relay.public_ipv6.as_deref(), Some("2001:db8::10"));
        assert_eq!(relay.relay_host, "198.51.100.10");
        assert_eq!(relay.stun_port, 19101);
        assert_eq!(relay.relay_port, 19101);
    }

    #[tokio::test]
    async fn relay_config_publishes_dual_stack_urls() {
        let manager = TurnManager::new().await;
        manager
            .register_relay(
                Some("198.51.100.10".to_string()),
                Some("2001:db8::10".to_string()),
                19101,
                19101,
                19101,
                Some("relay".to_string()),
                Some("relay-secret".to_string()),
            )
            .await;

        let relay = manager.get_relay_config("client-1").await.unwrap();

        assert_eq!(
            relay.urls,
            vec![
                "stun:198.51.100.10:19101".to_string(),
                "stun:[2001:db8::10]:19101".to_string(),
                "turn:198.51.100.10:19101".to_string(),
                "turn:[2001:db8::10]:19101".to_string()
            ]
        );
        assert_eq!(relay.username.as_deref(), Some("relay"));
        assert_eq!(relay.credential.as_deref(), Some("relay-secret"));
    }

    #[tokio::test]
    async fn get_relay_configs_returns_distinct_relays_up_to_n() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;
        register(&manager, "198.51.100.11", 19102).await;
        register(&manager, "198.51.100.12", 19103).await;

        let configs = manager.get_relay_configs("client-1", 2).await;
        assert_eq!(configs.len(), 2);

        let hosts: Vec<String> = configs.iter().flat_map(|c| c.urls.iter()).cloned().collect();
        let distinct_relay_ports: std::collections::HashSet<_> = hosts.iter().filter_map(|u| u.rsplit(':').next()).collect();
        assert_eq!(distinct_relay_ports.len(), 2);
    }

    #[tokio::test]
    async fn get_relay_configs_caps_at_registered_count() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;
        register(&manager, "198.51.100.11", 19102).await;

        let configs = manager.get_relay_configs("client-1", 5).await;
        assert_eq!(configs.len(), 2);
    }

    #[tokio::test]
    async fn get_relay_configs_treats_zero_as_one() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;

        let configs = manager.get_relay_configs("client-1", 0).await;
        assert_eq!(configs.len(), 1);
    }

    #[tokio::test]
    async fn get_relay_configs_is_sticky_across_calls() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;
        register(&manager, "198.51.100.11", 19102).await;
        register(&manager, "198.51.100.12", 19103).await;

        let first = manager.get_relay_configs("client-1", 2).await;
        let second = manager.get_relay_configs("client-1", 2).await;

        let first_urls: Vec<_> = first.iter().flat_map(|c| c.urls.clone()).collect();
        let second_urls: Vec<_> = second.iter().flat_map(|c| c.urls.clone()).collect();
        assert_eq!(first_urls, second_urls);
    }

    #[tokio::test]
    async fn unregister_client_decrements_counters_for_all_assigned_relays() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;
        register(&manager, "198.51.100.11", 19102).await;

        let _ = manager.get_relay_configs("client-1", 2).await;
        manager.unregister_client("client-1").await;

        let relays = manager.relays.lock().await;
        for relay in relays.values() {
            assert_eq!(relay.counter.load(std::sync::atomic::Ordering::Relaxed), 0);
        }
    }
}
