use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;
use tokio::sync::Mutex;
use serde::Deserialize;
use thiserror::Error;

use crate::turn_manager::{TurnServer, detect_continent};

#[derive(Debug, Deserialize)]
struct CloudflareDnsResponse {
    result: Vec<CloudflareDnsRecord>,
}

#[derive(Debug, Deserialize)]
struct CloudflareDnsRecord {
    name: String,
    content: String,
}

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Cloudflare API error: {0}")]
    CloudflareError(String),
    #[error("No TURN servers available")]
    NoServersAvailable,
}

pub struct TurnServerRegistry {
    http_client: reqwest::Client,
    cf_api_token: Option<String>,
    cf_zone_id: Option<String>,
    discovered_servers: Arc<Mutex<HashSet<TurnServer>>>,
    geoip_reader: Option<Arc<maxminddb::Reader<Vec<u8>>>>,
}

impl TurnServerRegistry {
    pub fn new(
        cf_api_token: Option<String>,
        cf_zone_id: Option<String>,
        geoip_reader: Option<Arc<maxminddb::Reader<Vec<u8>>>>,
    ) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            cf_api_token,
            cf_zone_id,
            discovered_servers: Arc::new(Mutex::new(HashSet::new())),
            geoip_reader,
        }
    }

    pub async fn get_servers(&self) -> Result<Vec<TurnServer>, RegistryError> {
        let servers = self.discovered_servers.lock().await;
        if servers.is_empty() {
            return Err(RegistryError::NoServersAvailable);
        }

        Ok(servers.iter().cloned().collect())
    }

    async fn discover_turn_servers(&self) -> Result<(), RegistryError> {
        let (token, zone_id) = match (&self.cf_api_token, &self.cf_zone_id) {
            (Some(t), Some(z)) => (t, z),
            _ => return Err(RegistryError::CloudflareError("Missing credentials".into())),
        };

        log::info!("Discovering TURNs via Cloudflare API...");
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records?type=A",
            zone_id
        );

        let resp = self.http_client.get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| RegistryError::CloudflareError(e.to_string()))?;

        let data: CloudflareDnsResponse = resp.json().await
            .map_err(|e| RegistryError::CloudflareError(e.to_string()))?;

        let current_servers_info: HashSet<_> = data.result.iter()
            .filter(|r| r.name.starts_with("turn"))
            .map(|r| {
                let continent = detect_continent(&r.content, self.geoip_reader.as_ref().map(|x| x.as_ref()));
                (r.content.clone(), r.name.clone(), continent)
            })
            .collect();

        let mut discovered = self.discovered_servers.lock().await;
        let existing_map: HashMap<_, _> = discovered.iter()
            .map(|s| ((s.ip.clone(), s.domain.clone(), s.continent), s.clone()))
            .collect();

        let mut new_server_set = HashSet::new();
        let mut added_servers = Vec::new();

        for (ip, domain, continent) in current_servers_info {
            let server = if let Some(existing) = existing_map.get(&(ip.clone(), domain.clone(), continent)) {
                existing.clone()
            } else {
                let new_server = TurnServer {
                    ip: ip.clone(),
                    domain: domain.clone(),
                    continent,
                    counter: Arc::new(AtomicUsize::new(0)),
                };
                added_servers.push(new_server.clone());
                new_server
            };
            new_server_set.insert(server);
        }

        let _removed_servers: Vec<_> = discovered.difference(&new_server_set).cloned().collect();

        *discovered = new_server_set;
        Ok(())
    }

    pub async fn run(self: Arc<Self>) {
        let mut ticker = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if let Err(e) = self.discover_turn_servers().await {
                        log::debug!("TURN discovery error: {}", e);
                    }
                }
            }
        }
    }
}
