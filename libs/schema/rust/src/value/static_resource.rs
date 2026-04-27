use crate::errors::Errors;
use crate::value::static_resource::static_resource::Source;
use std::env;
use std::hash::{Hash, Hasher};

tonic::include_proto!("value.static_resource");

impl StaticResource {
    pub fn s3_path(s3_path: S3Path) -> Self {
        Self {
            source: Some(static_resource::Source::S3Path(s3_path)),
        }
    }
}

impl Hash for S3Path {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bucket.hash(state);
        self.prefix.hash(state);
    }
}

impl Hash for Source {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Source::S3Path(s3_path) => {
                "S3Path".hash(state);
                s3_path.hash(state)
            }
            Source::Url(url) => {
                "Url".hash(state);
                url.hash(state)
            }
            Source::Path(path) => {
                "Path".hash(state);
                path.hash(state)
            }
        }
    }
}

impl Hash for StaticResource {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.source.hash(state);
    }
}

impl Eq for StaticResource {}

impl Eq for Source {}

impl Eq for S3Path {}

impl S3Path {
    pub fn use_default_bucket(prefix: impl Into<String>) -> Self {
        Self::new(env::var("DEFAULT_S3_BUCKET").unwrap_or("apac".to_owned()), prefix)
    }

    pub fn new(bucket: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            prefix: prefix.into(),
        }
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

impl From<S3Path> for String {
    fn from(val: S3Path) -> Self {
        format!("{}://{}", val.bucket, val.prefix)
    }
}

impl TryFrom<String> for S3Path {
    type Error = Errors;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let parts = value.split("://").collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err(Errors::InvalidS3Path(value));
        }

        let prefix = parts[1].trim().trim_matches('/').trim().to_string();
        Ok(Self {
            bucket: parts[0].to_string(),
            prefix,
        })
    }
}
