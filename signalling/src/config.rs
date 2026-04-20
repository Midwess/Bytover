const DEFAULT_REGION_CODE: &str = "local";

#[derive(Debug, Clone)]
pub struct SignallingConfig {
    pub region_code: String,
    pub signalling_route: String,
}

impl SignallingConfig {
    pub fn from_env() -> Self {
        let region_code = resolve_region_code(
            env_trimmed("BYTOVER_REGION_CODE").as_deref(),
            normalize_railway_region(env_trimmed("RAILWAY_REPLICA_REGION").as_deref()).as_deref(),
        );

        Self {
            signalling_route: format!("rpc-signalling-{region_code}"),
            region_code,
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

fn env_trimmed(key: &str) -> Option<String> {
    std::env::var(key).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn resolve_region_code(bytover_region_code: Option<&str>, railway_replica_region: Option<&str>) -> String {
    [
        bytover_region_code,
        railway_replica_region,
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|value| !value.is_empty())
    .unwrap_or(DEFAULT_REGION_CODE)
    .to_string()
}

fn normalize_railway_region(region: Option<&str>) -> Option<String> {
    region
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.split('-').find(|segment| !segment.is_empty()).unwrap_or(value).to_string())
}

#[cfg(test)]
mod tests {
    use super::{normalize_railway_region, resolve_region_code};

    #[test]
    fn falls_back_to_railway_replica_region() {
        let region_code = resolve_region_code(None, Some("eu-west"));

        assert_eq!(region_code, "eu-west");
    }

    #[test]
    fn prefers_explicit_bytover_region_code() {
        let region_code = resolve_region_code(Some("ap-southeast"), Some("eu-west"));

        assert_eq!(region_code, "ap-southeast");
    }

    #[test]
    fn canonicalizes_provider_formatted_railway_region() {
        let region_code = normalize_railway_region(Some("europe-west4-drams3a"));

        assert_eq!(region_code.as_deref(), Some("europe"));
    }

    #[test]
    fn preserves_already_short_railway_region() {
        let region_code = normalize_railway_region(Some("europe"));

        assert_eq!(region_code.as_deref(), Some("europe"));
    }

    #[test]
    fn defaults_to_local_when_both_region_envs_are_missing() {
        let region_code = resolve_region_code(None, None);

        assert_eq!(region_code, "local");
    }
}
