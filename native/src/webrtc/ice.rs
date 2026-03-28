use std::sync::Arc;
use str0m::{Candidate, Rtc};
use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use webrtc_ice::agent::agent_config::AgentConfig;
use webrtc_ice::agent::Agent;
use webrtc_ice::mdns::MulticastDnsMode;
use webrtc_ice::network_type::NetworkType;
use webrtc_ice::url::Url;

use schema::devlog::rpc_signalling::server::IceConfig;

#[derive(Debug, Error)]
pub enum IceError {
    #[error("ICE error: {0}")]
    Ice(#[from] webrtc_ice::Error),

    #[error("Candidate parsing error: {0}")]
    Parse(String)
}

pub struct IceAgent {
    agent: Agent,
    cache: Mutex<Vec<Candidate>>,
    handle: Arc<Mutex<Option<JoinHandle<()>>>>
}

impl IceAgent {
    pub async fn new(config: IceConfig) -> Result<Self, IceError> {
        let mut urls = vec![];
        for url_str in &config.urls {
            match Url::parse_url(url_str) {
                Ok(mut url) => {
                    if let Some(user) = &config.username {
                        url.username = user.clone();
                    }
                    if let Some(cred) = &config.credential {
                        url.password = cred.clone();
                    }
                    urls.push(url);
                }
                Err(e) => {
                    log::warn!("[webrtc-client] Failed to parse ICE URL {}: {}", url_str, e);
                }
            }
        }

        let agent_config = AgentConfig {
            urls,
            network_types: vec![
                NetworkType::Udp4,
                NetworkType::Udp6,
            ],
            multicast_dns_mode: MulticastDnsMode::QueryAndGather,
            ..Default::default()
        };

        let agent = Agent::new(agent_config).await?;

        Ok(Self {
            agent,
            cache: Mutex::new(vec![]),
            handle: Arc::new(Mutex::new(None))
        })
    }

    pub fn start_background_gathering(self: &Arc<Self>) {
        let weak_inner = Arc::downgrade(self);

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300));
            loop {
                interval.tick().await;

                let Some(inner) = weak_inner.upgrade() else {
                    break;
                };

                let mut cache = inner.cache.lock().await;
                cache.clear();

                let (tx, mut rx) = mpsc::channel(32);
                inner.agent.on_candidate(Box::new(move |c| {
                    let tx = tx.clone();
                    Box::pin(async move {
                        let _ = tx.send(c).await;
                    })
                }));

                if let Err(e) = inner.agent.restart(String::new(), String::new()).await {
                    log::error!("[webrtc-client] Failed to restart ICE agent: {:?}", e);
                    drop(cache);
                    continue;
                }

                if let Err(e) = inner.agent.gather_candidates() {
                    log::error!("[webrtc-client] Failed to start ICE gathering: {:?}", e);
                    drop(cache);
                    continue;
                }

                while let Some(c) = rx.recv().await {
                    if let Some(candidate) = c {
                        let mut sdp_line = format!("candidate:{}", candidate.marshal());

                        if sdp_line.contains(".local") {
                            let parts: Vec<&str> = sdp_line.split_whitespace().collect();
                            if parts.len() > 4 {
                                let ip = candidate.addr().ip().to_string();
                                let mut new_parts = parts;
                                new_parts[4] = &ip;
                                sdp_line = new_parts.join(" ");
                            }
                        }

                        match Candidate::from_sdp_string(&sdp_line) {
                            Ok(c) => {
                                if !cache.iter().any(|existing| existing.to_sdp_string() == sdp_line) {
                                    log::info!("[webrtc-client] Background gathered new candidate: {}", sdp_line);
                                    cache.push(c);
                                }
                            }
                            Err(e) => {
                                log::warn!(
                                    "[webrtc-client] Failed to parse gathered candidate: {} - error: {:?}",
                                    sdp_line,
                                    e
                                );
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
        });

        let handle_mutex = self.handle.clone();
        tokio::spawn(async move {
            let mut current_handle = handle_mutex.lock().await;
            if let Some(old) = current_handle.replace(handle) {
                old.abort();
            }
        });
    }

    pub async fn gather_candidates(&self, rtc: &mut Rtc) -> Result<(), IceError> {
        let candidates = self.cache.lock().await;

        if candidates.is_empty() {
            log::warn!("[webrtc-client] No candidates available in cache, ICE connection might fail");
        }

        for candidate in &*candidates {
            rtc.add_local_candidate(candidate.clone());
        }

        Ok(())
    }
}

impl Drop for IceAgent {
    fn drop(&mut self) {
        let handle_mutex = self.handle.clone();
        tokio::spawn(async move {
            if let Some(h) = handle_mutex.lock().await.take() {
                h.abort();
            }
        });
    }
}
