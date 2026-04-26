use schema::devlog::rpc_signalling::server::IceConfig;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

const SESSION_PICK_TTL: Duration = Duration::from_secs(45);
const PRUNE_INTERVAL: Duration = Duration::from_secs(5);
const COUNTER_RESET_INTERVAL: Duration = Duration::from_secs(300);
const COUNTER_RESET_TICKS: u32 = (COUNTER_RESET_INTERVAL.as_secs() / PRUNE_INTERVAL.as_secs()) as u32;
const RELAY_TIMEOUT: Duration = Duration::from_secs(30);

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

#[derive(Debug, Clone)]
struct SessionPick {
    relay_ids: Vec<String>,
    chosen_at: Instant,
}

pub struct TurnManager {
    relays: Arc<Mutex<HashMap<String, RegisteredRelay>>>,
    session_picks: Arc<Mutex<HashMap<String, SessionPick>>>,
}

impl TurnManager {
    pub async fn new() -> Self {
        let manager = Self {
            relays: Arc::new(Mutex::new(HashMap::new())),
            session_picks: Arc::new(Mutex::new(HashMap::new())),
        };

        let relays_clone = Arc::clone(&manager.relays);
        let picks_clone = Arc::clone(&manager.session_picks);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(PRUNE_INTERVAL);
            let mut tick: u32 = 0;
            loop {
                interval.tick().await;
                tick = tick.wrapping_add(1);
                let now = Instant::now();

                {
                    let mut relays = relays_clone.lock().await;
                    relays.retain(|id, relay| {
                        if now.duration_since(relay.last_ping) > RELAY_TIMEOUT {
                            log::info!(
                                "Relay {} (IPv4: {:?}, IPv6: {:?}, host: {}) timed out, removing",
                                id,
                                relay.public_ipv4,
                                relay.public_ipv6,
                                relay.relay_host
                            );
                            false
                        } else {
                            true
                        }
                    });

                    if tick % COUNTER_RESET_TICKS == 0 {
                        for relay in relays.values() {
                            relay.counter.store(0, Ordering::Relaxed);
                        }
                    }
                }

                {
                    let mut picks = picks_clone.lock().await;
                    picks.retain(|_, pick| now.duration_since(pick.chosen_at) < SESSION_PICK_TTL);
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

    pub async fn get_assigned_relays(&self, client_id: &str, n: usize) -> Vec<RegisteredRelay> {
        let n = n.max(1);
        let mut picks = self.session_picks.lock().await;
        let relays = self.relays.lock().await;

        if let Some(pick) = picks.get(client_id) {
            if pick.chosen_at.elapsed() < SESSION_PICK_TTL {
                let resolved: Vec<RegisteredRelay> = pick
                    .relay_ids
                    .iter()
                    .filter_map(|id| relays.get(id).cloned())
                    .collect();
                if !resolved.is_empty() {
                    return resolved;
                }
            }
        }

        if relays.is_empty() {
            return Vec::new();
        }

        let mut picked: Vec<RegisteredRelay> = Vec::with_capacity(n);
        for _ in 0..n {
            let next = relays
                .values()
                .min_by_key(|r| r.counter.load(Ordering::Relaxed))
                .cloned()
                .expect("relays non-empty checked above");
            next.counter.fetch_add(1, Ordering::Relaxed);
            picked.push(next);
        }

        picks.insert(
            client_id.to_string(),
            SessionPick {
                relay_ids: picked.iter().map(|r| r.id.clone()).collect(),
                chosen_at: Instant::now(),
            },
        );

        picked
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

        let relay = manager.get_assigned_relays("client-1", 1).await.into_iter().next().unwrap();

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

        let relay = manager.get_relay_configs("client-1", 1).await.into_iter().next().unwrap();

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
    async fn assigned_relays_spreads_across_relays_when_balanced() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;
        register(&manager, "198.51.100.11", 19102).await;
        register(&manager, "198.51.100.12", 19103).await;

        let assigned = manager.get_assigned_relays("client-1", 3).await;
        assert_eq!(assigned.len(), 3);

        let distinct_ids: std::collections::HashSet<_> = assigned.iter().map(|r| r.id.clone()).collect();
        assert_eq!(distinct_ids.len(), 3);

        let relays = manager.relays.lock().await;
        for relay in relays.values() {
            assert_eq!(relay.counter.load(std::sync::atomic::Ordering::Relaxed), 1);
        }
    }

    #[tokio::test]
    async fn assigned_relays_returns_n_entries_even_when_fewer_relays_registered() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;

        let assigned = manager.get_assigned_relays("client-1", 3).await;
        assert_eq!(assigned.len(), 3);

        let only_id = assigned[0].id.clone();
        assert!(assigned.iter().all(|r| r.id == only_id));

        let relays = manager.relays.lock().await;
        let relay = relays.values().next().unwrap();
        assert_eq!(relay.counter.load(std::sync::atomic::Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn assigned_relays_repeat_least_loaded_under_skew() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;
        register(&manager, "198.51.100.11", 19102).await;
        register(&manager, "198.51.100.12", 19103).await;

        let saturated_ids: Vec<String> = {
            let relays = manager.relays.lock().await;
            let mut iter = relays.values();
            let _cold = iter.next().unwrap();
            let hot_a = iter.next().unwrap();
            let hot_b = iter.next().unwrap();
            hot_a.counter.fetch_add(10, std::sync::atomic::Ordering::Relaxed);
            hot_b.counter.fetch_add(10, std::sync::atomic::Ordering::Relaxed);
            vec![hot_a.id.clone(), hot_b.id.clone()]
        };

        let assigned = manager.get_assigned_relays("client-1", 3).await;
        assert_eq!(assigned.len(), 3);

        let cold_id = assigned[0].id.clone();
        assert!(!saturated_ids.contains(&cold_id));
        assert!(assigned.iter().all(|r| r.id == cold_id));

        let relays = manager.relays.lock().await;
        let cold = relays.get(&cold_id).unwrap();
        assert_eq!(cold.counter.load(std::sync::atomic::Ordering::Relaxed), 3);
    }

    #[tokio::test]
    async fn get_relay_configs_treats_zero_as_one() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;

        let configs = manager.get_relay_configs("client-1", 0).await;
        assert_eq!(configs.len(), 1);
    }

    #[tokio::test]
    async fn get_relay_configs_is_stable_within_ttl() {
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
    async fn two_peer_consistency_within_ttl() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;
        register(&manager, "198.51.100.11", 19102).await;
        register(&manager, "198.51.100.12", 19103).await;

        let offerer = manager.get_relay_configs("peer-key", 2).await;
        let receiver = manager.get_relay_configs("peer-key", 2).await;

        let offerer_urls: Vec<_> = offerer.iter().flat_map(|c| c.urls.clone()).collect();
        let receiver_urls: Vec<_> = receiver.iter().flat_map(|c| c.urls.clone()).collect();
        assert_eq!(offerer_urls, receiver_urls);
    }

    #[tokio::test]
    async fn counter_increments_per_pick() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;
        register(&manager, "198.51.100.11", 19102).await;

        let _ = manager.get_relay_configs("client-a", 2).await;
        let _ = manager.get_relay_configs("client-b", 2).await;

        let relays = manager.relays.lock().await;
        for relay in relays.values() {
            assert_eq!(relay.counter.load(std::sync::atomic::Ordering::Relaxed), 2);
        }
    }

    #[tokio::test]
    async fn distinct_peers_pick_least_loaded_first() {
        let manager = TurnManager::new().await;
        register(&manager, "198.51.100.10", 19101).await;
        register(&manager, "198.51.100.11", 19102).await;

        let _ = manager.get_relay_configs("client-a", 1).await;
        let _ = manager.get_relay_configs("client-b", 1).await;

        let relays = manager.relays.lock().await;
        let counts: Vec<usize> = relays
            .values()
            .map(|r| r.counter.load(std::sync::atomic::Ordering::Relaxed))
            .collect();
        assert_eq!(counts.iter().sum::<usize>(), 2);
        assert!(counts.iter().all(|&c| c == 1));
    }
}
