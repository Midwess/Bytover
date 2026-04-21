const DEFAULT_REGION_CODE: &str = "local";
const DEFAULT_CONNECTION_FANOUT: usize = 2;
const MAX_CONNECTION_FANOUT: usize = 8;

#[derive(Debug, Clone)]
pub struct SignallingConfig {
    pub region_code: String,
    pub signalling_route: String,
    pub connection_fanout: usize,
}

impl SignallingConfig {
    pub fn from_env() -> Self {
        let region_code = resolve_region_code(
            env_trimmed("BYTOVER_REGION_CODE").as_deref(),
            normalize_railway_region(env_trimmed("RAILWAY_REPLICA_REGION").as_deref()).as_deref(),
        );

        let connection_fanout = resolve_connection_fanout(env_trimmed("BYTOVER_CONNECTION_FANOUT").as_deref());

        Self {
            signalling_route: format!("rpc-signalling-{region_code}"),
            region_code,
            connection_fanout,
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

fn resolve_connection_fanout(value: Option<&str>) -> usize {
    value
        .and_then(|raw| raw.parse::<usize>().ok())
        .map(|n| n.clamp(1, MAX_CONNECTION_FANOUT))
        .unwrap_or(DEFAULT_CONNECTION_FANOUT)
}

fn normalize_railway_region(region: Option<&str>) -> Option<String> {
    region
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.split('-').find(|segment| !segment.is_empty()).unwrap_or(value).to_string())
}

#[cfg(test)]
mod tests {
    use super::{normalize_railway_region, resolve_connection_fanout, resolve_region_code, MAX_CONNECTION_FANOUT};

    #[test]
    fn connection_fanout_defaults_to_two_when_unset() {
        assert_eq!(resolve_connection_fanout(None), 2);
    }

    #[test]
    fn connection_fanout_parses_positive_value() {
        assert_eq!(resolve_connection_fanout(Some("3")), 3);
    }

    #[test]
    fn connection_fanout_clamps_zero_up_to_one() {
        assert_eq!(resolve_connection_fanout(Some("0")), 1);
    }

    #[test]
    fn connection_fanout_clamps_excessive_value() {
        assert_eq!(resolve_connection_fanout(Some("999")), MAX_CONNECTION_FANOUT);
    }

    #[test]
    fn connection_fanout_ignores_non_numeric_value() {
        assert_eq!(resolve_connection_fanout(Some("abc")), 2);
    }


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
