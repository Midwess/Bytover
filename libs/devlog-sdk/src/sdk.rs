#[cfg(feature = "s3")]
use core_services::s3::S3Client;
#[cfg(feature = "s3")]
use tokio::sync::OnceCell;

#[async_trait::async_trait]
pub trait DependenciesInjection {
    #[cfg(feature = "s3")]
    fn s3_client(&self) -> S3Client;
}

pub struct DevlogSdk {
    #[cfg(feature = "s3")]
    s3_client: OnceCell<S3Client>
}

impl Default for DevlogSdk {
    fn default() -> Self {
        Self::new()
    }
}

impl DevlogSdk {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "s3")]
            s3_client: OnceCell::new()
        }
    }
}

#[async_trait::async_trait]
impl DependenciesInjection for DevlogSdk {
    #[cfg(feature = "s3")]
    fn s3_client(&self) -> S3Client {
        use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
        use aws_sdk_s3::Client;
        use std::env;
        use std::sync::Arc;

        if let Some(s3_client) = self.s3_client.get() {
            return s3_client.clone();
        }

        let region = env::var("AWS_S3_REGION").unwrap_or("us-east-1".to_string());
        let access_key = env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID must be defined");
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY must be defined");
        let endpoint_url = env::var("AWS_ENDPOINT_URL").expect("AWS_ENDPOINT_URL must be defined");

        let credentials = Credentials::new(access_key, secret_key, None, None, "cloudflare");

        let mut s3_config_builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region))
            .credentials_provider(credentials);

        s3_config_builder = s3_config_builder.endpoint_url(endpoint_url).force_path_style(true);

        let client = Client::from_conf(s3_config_builder.build());

        let s3_client = S3Client {
            client: Arc::new(client)
        };

        let _ = self.s3_client.set(s3_client.clone());

        s3_client
    }
}
