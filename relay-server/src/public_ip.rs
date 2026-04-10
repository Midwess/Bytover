use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicAddresses {
    pub ipv4: Option<Ipv4Addr>,
    pub ipv6: Option<Ipv6Addr>
}

impl PublicAddresses {
    pub fn localhost() -> Self {
        Self {
            ipv4: Some(Ipv4Addr::LOCALHOST),
            ipv6: Some(Ipv6Addr::LOCALHOST)
        }
    }

    pub fn ensure_any(self) -> Result<Self, String> {
        if self.ipv4.is_none() && self.ipv6.is_none() {
            return Err("failed to discover any public IP address".to_string());
        }

        Ok(self)
    }
}

fn is_development_environment() -> bool {
    devlog_sdk::config::CONFIGS.environment.eq_ignore_ascii_case("development")
}

pub async fn discover_public_addresses() -> Result<PublicAddresses, String> {
    if is_development_environment() {
        return Ok(PublicAddresses::localhost());
    }

    let (ipv4, ipv6) = tokio::join!(public_ip::addr_v4(), public_ip::addr_v6());

    PublicAddresses { ipv4, ipv6 }.ensure_any()
}

#[cfg(test)]
mod tests {
    use super::PublicAddresses;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn localhost_addresses_use_ip_literals() {
        let addresses = PublicAddresses::localhost();

        assert_eq!(addresses.ipv4, Some(Ipv4Addr::LOCALHOST));
        assert_eq!(addresses.ipv6, Some(Ipv6Addr::LOCALHOST));
    }

    #[test]
    fn ensure_any_rejects_empty_addresses() {
        let error = PublicAddresses { ipv4: None, ipv6: None }.ensure_any().unwrap_err();

        assert!(error.contains("failed to discover any public IP address"));
    }
}
