use lazy_static::lazy_static;
use std::env;

#[derive(Debug, Clone)]
pub struct KongGatewayConfig {
    pub admin_url: String,
    pub url: String,
    pub host: String,
    pub port: u16,
    pub public_host: String,
    pub public_port: u16,
    pub public_with_ssl: bool
}

impl Default for KongGatewayConfig {
    fn default() -> Self {
        Self {
            admin_url: env::var("KONG_GATEWAY_ADMIN_URL").unwrap_or("http://localhost:8001".to_owned()),
            url: env::var("KONG_GATEWAY_URL").unwrap_or("http://localhost:8000".to_owned()),
            host: env::var("KONG_GATEWAY_HOST").unwrap_or("localhost".to_owned()),
            port: env::var("KONG_GATEWAY_PORT").unwrap_or("8000".to_owned()).parse().unwrap_or(8000),
            public_host: env::var("PUBLIC_GATEWAY_HOST").unwrap_or("localhost".to_owned()),
            public_port: env::var("PUBLIC_GATEWAY_PORT").ok().and_then(|it| it.parse().ok()).unwrap_or(8000),
            public_with_ssl: env::var("PUBLIC_GATEWAY_WITH_SSL").ok().and_then(|it| it.parse().ok()).unwrap_or(false)
        }
    }
}

impl KongGatewayConfig {
    pub fn public_url(&self) -> String {
        if self.public_with_ssl {
            format!("https://{}:{}", self.public_host, self.public_port)
        } else {
            format!("http://{}:{}", self.public_host, self.public_port)
        }
    }
}

pub struct Config {
    pub kong: KongGatewayConfig,
    pub environment: String
}

impl Default for Config {
    fn default() -> Self {
        let environment = env::var("ENVIRONMENT").unwrap_or("development".to_owned());
        Self {
            kong: KongGatewayConfig::default(),
            environment
        }
    }
}

lazy_static! {
    pub static ref CONFIGS: Config = Default::default();
}
