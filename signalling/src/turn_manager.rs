use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH, Instant, Duration};
use base64::Engine;
use base64::engine::general_purpose;
use hmac::{Hmac, Mac};
use tokio::sync::Mutex;
use thiserror::Error;
use schema::devlog::rpc_signalling::server::IceConfig;
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

#[derive(Error, Debug)]
pub enum TurnManagerErrors {
    #[error("BYTOVER_TURN_SECRET not configured")]
    NoTurnSecret,
    #[error("JWT generation error: {0}")]
    JwtError(String),
    #[error("No relays available")]
    NoRelaysAvailable,
}

#[derive(Debug, Clone)]
pub struct RegisteredRelay {
    pub ip: String,
    pub stun_port: u16,
    pub relay_port: u16,
    pub last_ping: Instant,
    pub counter: Arc<AtomicUsize>,
}

pub struct TurnManager {
    relays: Arc<Mutex<HashMap<String, RegisteredRelay>>>,
    client_relay_configs: Arc<Mutex<HashMap<String, IceConfig>>>,
    turn_secret: Option<String>,
}

impl TurnManager {
    pub async fn new() -> Result<Self, TurnManagerErrors> {
        let turn_secret = std::env::var("BYTOVER_TURN_SECRET").ok();
        if turn_secret.is_none() {
            log::warn!("BYTOVER_TURN_SECRET not set, TURN credentials will not be generated");
        }

        let manager = Self {
            relays: Arc::new(Mutex::new(HashMap::new())),
            client_relay_configs: Arc::new(Mutex::new(HashMap::new())),
            turn_secret,
        };

        // Start background task to prune expired relays
        let relays_clone = Arc::clone(&manager.relays);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                let mut relays = relays_clone.lock().await;
                let now = Instant::now();
                relays.retain(|_, relay| {
                    if now.duration_since(relay.last_ping) > Duration::from_secs(30) {
                        log::info!("Relay {} timed out, removing", relay.ip);
                        false
                    } else {
                        true
                    }
                });
            }
        });

        Ok(manager)
    }

    pub async fn register_relay(&self, ip: String, stun_port: u16, relay_port: u16) {
        let mut relays = self.relays.lock().await;
        let entry = relays.entry(ip.clone()).or_insert_with(|| {
            log::info!("New relay registered: {} (STUN: {}, Relay: {})", ip, stun_port, relay_port);
            RegisteredRelay {
                ip: ip.clone(),
                stun_port,
                relay_port,
                last_ping: Instant::now(),
                counter: Arc::new(AtomicUsize::new(0)),
            }
        });
        
        entry.stun_port = stun_port;
        entry.relay_port = relay_port;
        entry.last_ping = Instant::now();
    }

    pub async fn unregister_client(&self, client_id: &str) {
        let mut relay_configs = self.client_relay_configs.lock().await;
        relay_configs.remove(client_id);
    }

    pub async fn assign_relay_for_client(
        &self,
        client_id: &str,
    ) -> Result<IceConfig, TurnManagerErrors> {
        let relays = self.relays.lock().await;
        
        // Simple load balancing: pick relay with lowest counter
        let relay = relays.values()
            .min_by_key(|r| r.counter.load(Ordering::Relaxed))
            .ok_or(TurnManagerErrors::NoRelaysAvailable)?
            .clone();

        relay.counter.fetch_add(1, Ordering::Relaxed);

        let turn_url_udp = format!("turn:{}:{}?transport=udp", relay.ip, relay.stun_port);
        let turn_url_tcp = format!("turn:{}:{}?transport=tcp", relay.ip, relay.stun_port);

        let (username, credential, _ttl) = self.generate_turn_credential(client_id, "relay")?;

        let ice_config = IceConfig {
            urls: vec![
                format!("stun:{}:{}", relay.ip, relay.stun_port),
                turn_url_udp,
                turn_url_tcp,
            ],
            username: Some(username),
            credential: Some(credential),
        };

        let mut relay_configs = self.client_relay_configs.lock().await;
        relay_configs.insert(client_id.to_string(), ice_config.clone());

        Ok(ice_config)
    }

    pub async fn get_relay_config(&self, client_id: &str) -> Option<IceConfig> {
        let relay_configs = self.client_relay_configs.lock().await;
        relay_configs.get(client_id).cloned()
    }

    pub fn generate_turn_credential(
        &self,
        p1_uuid: &str,
        p2_uuid: &str,
    ) -> Result<(String, String, u64), TurnManagerErrors> {
        use sha1::{Sha1, Digest};

        let secret = self.turn_secret.as_ref().ok_or(TurnManagerErrors::NoTurnSecret)?;

        let ttl: u64 = 60 * 60 * 24; // 24 hours

        // Calculate expiry timestamp (current time + ttl)
        let expiry = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + ttl;

        // Create short unique identifier from peer pair
        let mut hasher = Sha1::new();
        hasher.update(format!("{p1_uuid}:{p2_uuid}").as_bytes());
        let hash = hasher.finalize();
        let user_identifier = general_purpose::URL_SAFE_NO_PAD.encode(&hash[..9]);

        // Create time-limited username: {expiry_timestamp}:{user_identifier}
        let username = format!("{expiry}:{user_identifier}");

        // Compute password: base64(HMAC-SHA1(secret, username))
        let mut mac = HmacSha1::new_from_slice(secret.as_bytes())
            .map_err(|e| TurnManagerErrors::JwtError(format!("Invalid shared secret: {}", e)))?;

        mac.update(username.as_bytes());

        let password = general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        Ok((username, password, ttl))
    }
}