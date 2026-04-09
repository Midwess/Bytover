const DEFAULT_REGION_CODE: &str = "local";

#[derive(Debug, Clone)]
pub struct SignallingConfig {
    pub region_code: String,
    pub signalling_route: String
}

impl SignallingConfig {
    pub fn from_env() -> Self {
        let region_code = std::env::var("BYTOVER_REGION_CODE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_REGION_CODE.to_string());

        Self {
            signalling_route: format!("rpc-signalling-{region_code}"),
            region_code
        }
    }

    pub fn pinned_upstream_url(&self, public_host: &str, port: u16) -> String {
        env_url("BYTOVER_SIGNALLING_PINNED_UPSTREAM_URL")
            .or_else(|| env_host("BYTOVER_SIGNALLING_PRIVATE_HOST").map(|host| format!("http://{host}:{port}")))
            .unwrap_or_else(|| format!("http://{public_host}:{port}"))
    }
}

fn env_url(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn env_host(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}
