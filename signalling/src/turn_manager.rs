use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::net::IpAddr;
use base64::Engine;
use base64::engine::general_purpose;
use hmac::{Hmac, Mac};
use tokio::sync::Mutex;
use thiserror::Error;
use maxminddb::geoip2;
use schema::devlog::rpc_signalling::server::IceConfig;
use sha1::Sha1;

use crate::turn_server_registry::TurnServerRegistry;

type HmacSha1 = Hmac<Sha1>;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum Continent {
    AS,
    NorthAS,
    Tokyo,
    Singapore,
    HKG,
    EU,
    NA,
    SJC,
    SA,
    OC,
    AF,
    Unknown,
}

impl Continent {
    fn priority_order(&self) -> &'static [Continent] {
        match self {
            Continent::AS => &[Continent::AS, Continent::HKG, Continent::Singapore, Continent::Tokyo, Continent::NorthAS, Continent::OC, Continent::EU, Continent::NA, Continent::SJC, Continent::SA, Continent::AF],
            Continent::NorthAS => &[Continent::NorthAS, Continent::Tokyo, Continent::AS, Continent::HKG, Continent::Singapore, Continent::EU, Continent::OC, Continent::NA, Continent::SJC, Continent::SA, Continent::AF],
            Continent::Tokyo => &[Continent::Tokyo, Continent::NorthAS, Continent::AS, Continent::HKG, Continent::Singapore, Continent::OC, Continent::SJC, Continent::NA, Continent::EU, Continent::SA, Continent::AF],
            Continent::Singapore => &[Continent::Singapore, Continent::HKG, Continent::AS, Continent::OC, Continent::Tokyo, Continent::NorthAS, Continent::EU, Continent::NA, Continent::SJC, Continent::AF, Continent::SA],
            Continent::HKG => &[Continent::HKG, Continent::AS, Continent::Singapore, Continent::Tokyo, Continent::NorthAS, Continent::OC, Continent::EU, Continent::NA, Continent::SJC, Continent::SA, Continent::AF],
            Continent::EU => &[Continent::EU, Continent::NorthAS, Continent::NA, Continent::AS, Continent::HKG, Continent::Tokyo, Continent::Singapore, Continent::SJC, Continent::AF, Continent::OC, Continent::SA],
            Continent::NA => &[Continent::NA, Continent::SJC, Continent::SA, Continent::Tokyo, Continent::EU, Continent::NorthAS, Continent::Singapore, Continent::HKG, Continent::AS, Continent::OC, Continent::AF],
            Continent::SJC => &[Continent::SJC, Continent::NA, Continent::Tokyo, Continent::SA, Continent::OC, Continent::EU, Continent::NorthAS, Continent::Singapore, Continent::HKG, Continent::AS, Continent::AF],
            Continent::SA => &[Continent::SA, Continent::NA, Continent::SJC, Continent::EU, Continent::AF, Continent::AS, Continent::HKG, Continent::Singapore, Continent::NorthAS, Continent::Tokyo, Continent::OC],
            Continent::OC => &[Continent::OC, Continent::Singapore, Continent::HKG, Continent::AS, Continent::Tokyo, Continent::SJC, Continent::NorthAS, Continent::NA, Continent::EU, Continent::SA, Continent::AF],
            Continent::AF => &[Continent::AF, Continent::EU, Continent::Singapore, Continent::HKG, Continent::AS, Continent::Tokyo, Continent::NorthAS, Continent::NA, Continent::SJC, Continent::SA, Continent::OC],
            Continent::Unknown => &[Continent::AS, Continent::HKG, Continent::Singapore, Continent::Tokyo, Continent::NorthAS, Continent::EU, Continent::NA, Continent::SJC, Continent::SA, Continent::OC, Continent::AF],
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PeerPair {
    peer1: String,
    peer2: String,
}

impl PeerPair {
    pub fn new(peer1: String, peer2: String) -> Self {
        // peer1 is the main peer (from), peer2 is the target (to)
        // (p1, p2) and (p2, p1) are now different
        Self { peer1, peer2 }
    }
}

impl Hash for PeerPair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash both peers in the established order
        self.peer1.hash(state);
        self.peer2.hash(state);
    }
}

pub fn detect_continent(ip: &str, reader: Option<&maxminddb::Reader<Vec<u8>>>) -> Continent {
    if let Some(reader) = reader {
        if let Ok(addr) = ip.parse::<IpAddr>() {
            if let Ok(city) = reader.lookup::<geoip2::City>(addr) {
                if let Some(country_info) = city.country {
                    if let Some(iso_code) = country_info.iso_code {
                        if iso_code == "US" {
                            if let Some(subdivisions) = &city.subdivisions {
                                if let Some(first_sub) = subdivisions.first() {
                                    if let Some(sub_code) = first_sub.iso_code {
                                        return match sub_code {
                                            "CA" | "OR" | "WA" | "NV" | "AZ" => Continent::SJC,
                                            _ => Continent::NA,
                                        };
                                    }
                                }
                            }
                            return Continent::NA;
                        }

                        return match iso_code {
                            "JP" => Continent::Tokyo,
                            "SG" | "MY" | "ID" | "TH" | "BN" => Continent::Singapore,
                            "CN" | "KR" | "IN" | "VN" | "PH" | "TW" | "HK" | "MO" | "KH" | "LA" | "MM" | "BD" | "PK" | "LK" | "NP" | "BT" | "MV" | "AF" | "UZ" | "TM" | "TJ" | "KG" => Continent::AS,
                            "RU" | "MN" | "KZ" => Continent::NorthAS,
                            "GB" | "FR" | "DE" | "IT" | "ES" | "NL" | "BE" | "PL" | "RO" | "CZ" | "PT" | "GR" | "HU" | "SE" | "AT" | "BG" | "DK" | "FI" | "SK" | "NO" | "IE" | "HR" | "SI" | "LT" | "LV" | "EE" | "LU" | "MT" | "CY" | "IS" | "CH" | "UA" | "BY" | "MD" | "RS" | "BA" | "AL" | "MK" | "ME" | "XK" => Continent::EU,
                            "CA" | "MX" => Continent::NA,
                            "BR" | "AR" | "CO" | "PE" | "VE" | "CL" | "EC" | "BO" | "PY" | "UY" | "GY" | "SR" | "GF" => Continent::SA,
                            "AU" | "NZ" | "PG" | "FJ" | "NC" | "PF" | "SB" | "VU" | "WS" | "KI" | "TO" | "FM" | "MH" | "PW" | "NR" | "TV" => Continent::OC,
                            "ZA" | "EG" | "NG" | "KE" | "ET" | "GH" | "TZ" | "UG" | "DZ" | "SD" | "MA" | "AO" | "MZ" | "MG" | "CM" | "CI" | "NE" | "BF" | "ML" | "MW" | "ZM" | "SN" | "SO" | "TD" | "ZW" | "GN" | "RW" | "BJ" | "TN" | "BI" | "SS" | "TG" | "SL" | "LY" | "LR" | "MR" | "CF" | "ER" | "GM" | "BW" | "GA" | "GW" | "MU" | "SZ" | "DJ" | "RE" | "KM" | "CV" | "ST" | "SC" => Continent::AF,
                            _ => Continent::Unknown,
                        };
                    }
                }
            }
        }
    }
    Continent::Unknown
}

#[derive(Error, Debug)]
pub enum TurnManagerErrors {
    #[error("GeoIP error: {0}")]
    GeoIpError(#[from] maxminddb::MaxMindDBError),
    #[error("BYTOVER_TURN_SECRET not configured")]
    NoTurnSecret,
    #[error("BYTOVER_TURN_SECRET not configured")]
    NoTurnRealm,
    #[error("JWT generation error: {0}")]
    JwtError(String),
    #[error("Registry error: {0}")]
    RegistryError(#[from] crate::turn_server_registry::RegistryError),
}

#[derive(Debug, Clone)]
pub struct TurnServer {
    pub ip: String,
    pub domain: String,
    pub continent: Continent,
    pub counter: Arc<AtomicUsize>,
}

impl Hash for TurnServer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ip.hash(state);
        self.domain.hash(state);
        self.continent.hash(state);
    }
}

impl PartialEq for TurnServer {
    fn eq(&self, other: &Self) -> bool {
        self.ip == other.ip && self.domain == other.domain && self.continent == other.continent
    }
}

impl Eq for TurnServer {}

pub struct TurnManager {
    server_registry: Arc<TurnServerRegistry>,
    peer_turn_cache: Arc<Mutex<HashMap<PeerPair, IceConfig>>>,
    client_continents: Arc<Mutex<HashMap<String, Continent>>>,
    turn_secret: Option<String>,
    turn_realm: Option<String>,
}

impl TurnManager {
    pub async fn new() -> Result<Self, TurnManagerErrors> {
        let geoip_data = include_bytes!("../GeoLite2-City.mmdb");
        let geoip_reader = maxminddb::Reader::from_source(geoip_data.to_vec())
            .ok()
            .map(Arc::new);

        if geoip_reader.is_none() {
            log::warn!("GeoIP database not found or invalid, continent detection will use Unknown");
        }

        let turn_secret = std::env::var("BYTOVER_TURN_SECRET").ok();
        if turn_secret.is_none() {
            log::warn!("BYTOVER_TURN_SECRET not set, TURN credentials will not be generated");
        }

        let cf_api_token = std::env::var("CLOUD_FLARE_API_TOKEN").ok();
        let cf_zone_id = std::env::var("CLOUD_FLARE_ZONE_ID").ok();
        if cf_api_token.is_none() || cf_zone_id.is_none() {
            log::warn!("Cloudflare API credentials not set, TURN discovery will be disabled");
        }

        let server_registry = Arc::new(TurnServerRegistry::new(
            cf_api_token,
            cf_zone_id,
            geoip_reader,
        ));

        Ok(Self {
            server_registry,
            turn_realm: Some(std::env::var("BYTOVER_TURN_REALM").ok().unwrap_or("bytover.com".to_string())),
            peer_turn_cache: Arc::new(Mutex::new(HashMap::new())),
            client_continents: Arc::new(Mutex::new(HashMap::new())),
            turn_secret,
        })
    }

    pub fn get_registry(&self) -> Arc<TurnServerRegistry> {
        self.server_registry.clone()
    }

    pub async fn register_client(&self, client_id: String, continent: Continent) {
        let mut continents = self.client_continents.lock().await;
        continents.insert(client_id, continent);
    }

    pub async fn unregister_client(&self, client_id: &str) {
        let mut continents = self.client_continents.lock().await;
        continents.remove(client_id);
    }

    /// Generate TURN REST API credentials
    ///
    /// Uses the standard TURN REST API algorithm:
    /// - username = "{expiry_timestamp}:{user_identifier}"
    /// - password = base64(HMAC-SHA1(secret, username))
    ///
    /// The user_identifier is a short hash of the peer pair for uniqueness.
    ///
    /// Reference: http://tools.ietf.org/html/draft-uberti-behave-turn-rest-00
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

        // Create short unique identifier from peer pair (12 chars, ~72 bits entropy)
        let mut hasher = Sha1::new();
        hasher.update(format!("{p1_uuid}:{p2_uuid}").as_bytes());
        let hash = hasher.finalize();
        let user_identifier = general_purpose::URL_SAFE_NO_PAD.encode(&hash[..9]); // 12 chars

        // Create time-limited username: {expiry_timestamp}:{user_identifier}
        let username = format!("{expiry}:{user_identifier}");

        // Compute password: base64(HMAC-SHA1(secret, username))
        let mut mac = HmacSha1::new_from_slice(secret.as_bytes())
            .map_err(|e| TurnManagerErrors::JwtError(format!("Invalid shared secret: {}", e)))?;

        mac.update(username.as_bytes());

        let password = general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        Ok((username, password, ttl))
    }

    pub async fn get_turn_for_message(&self, from_id: &str, to_id: Option<&str>) -> Option<IceConfig> {
        let to_id = to_id?;
        let continents = self.client_continents.lock().await;

        let from_continent = continents.get(from_id).copied()?;
        let to_continent = continents.get(to_id).copied()?;

        drop(continents);

        match self.closest_turn_for_peers(
            from_id.to_string(),
            from_continent,
            to_id.to_string(),
            to_continent
        ).await {
            Ok(config) => Some(config),
            Err(e) => {
                log::warn!("Failed to get TURN config for peers {:?} {:?}: {:?}", from_id, to_id, e);
                None
            }
        }
    }

    pub async fn closest_turn_for_peers(&self, peer1_id: String, peer1_continent: Continent, peer2_id: String, peer2_continent: Continent) -> Result<IceConfig, TurnManagerErrors> {
        let peer_pair = PeerPair::new(peer1_id.clone(), peer2_id.clone());
        let mut cache = self.peer_turn_cache.lock().await;
        if let Some(cached_config) = cache.get(&peer_pair) {
            log::debug!("Using cached ICE config for peer pair");
            return Ok(cached_config.clone());
        }

        let servers_vec = self.server_registry.get_servers().await?;

        let peer1_stun_server = self.select_stun_for_peer(peer1_continent, &servers_vec);

        let peer1_turn_server = self.select_turn_for_peer(peer1_continent, &servers_vec);

        let peer2_stun_server = self.select_stun_for_peer(peer2_continent, &servers_vec);

        let peer2_turn_server = self.select_turn_for_peer(peer2_continent, &servers_vec);

        let selected_turn_server = if peer1_turn_server.counter.load(Ordering::Relaxed)
            <= peer2_turn_server.counter.load(Ordering::Relaxed) {
            peer1_turn_server.counter.fetch_add(1, Ordering::Relaxed);
            peer1_turn_server
        } else {
            peer2_turn_server.counter.fetch_add(1, Ordering::Relaxed);
            peer2_turn_server
        };

        let turn_url_udp = format!("turn:{}:3478?transport=udp", selected_turn_server.domain);
        let turn_url_tcp = format!("turn:{}:3478?transport=tcp", selected_turn_server.domain);

        let (username1, credential1, _ttl1) = self.generate_turn_credential(
            &peer1_id,
            &peer2_id,
        )?;

        let (username2, credential2, _ttl2) = self.generate_turn_credential(
            &peer2_id,
            &peer1_id,
        )?;

        let ice_config1 = IceConfig {
            urls: vec![format!("stun:{}", peer1_stun_server.domain), turn_url_udp.clone(), turn_url_tcp.clone()],
            username: Some(username1.clone()),
            credential: Some(credential1.clone()),
        };

        let ice_config2 = IceConfig {
            urls: vec![format!("stun:{}", peer2_stun_server.domain), turn_url_udp, turn_url_tcp],
            username: Some(username2.clone()),
            credential: Some(credential2.clone()),
        };

        let result = ice_config1.clone();
        cache.insert(PeerPair::new(peer1_id.clone(), peer2_id.clone()), ice_config1);
        cache.insert(PeerPair::new(peer2_id, peer1_id), ice_config2);

        Ok(result)
    }

    #[inline]
    fn select_server_for_peer(&self, peer_continent: Continent, servers: &[TurnServer]) -> TurnServer {
        let priority_order = peer_continent.priority_order();

        for &target_continent in priority_order {
            let candidates: Vec<&TurnServer> = servers.iter()
                .filter(|s| s.continent == target_continent)
                .collect();

            if !candidates.is_empty() {
                return (*candidates.iter()
                    .min_by_key(|s| s.counter.load(Ordering::Relaxed))
                    .unwrap())
                    .clone();
            }
        }

        servers.iter()
            .min_by_key(|s| s.counter.load(Ordering::Relaxed))
            .unwrap()
            .clone()
    }

    #[inline]
    fn select_stun_for_peer(&self, peer_continent: Continent, servers: &[TurnServer]) -> TurnServer {
        self.select_server_for_peer(peer_continent, servers)
    }

    #[inline]
    fn select_turn_for_peer(&self, peer_continent: Continent, servers: &[TurnServer]) -> TurnServer {
        self.select_server_for_peer(peer_continent, servers)
    }
}