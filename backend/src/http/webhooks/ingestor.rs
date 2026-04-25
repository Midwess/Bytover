use crate::repositories::app_release::{AppReleaseRepository, StoreReleaseUpsert};
use core_services::db::repository::abstraction::errors::RepositoryError;

#[derive(Debug, Clone)]
pub struct AppStoreReleaseUpdatedData {
    pub platform: String,
    pub version: String,
    pub app_store_url: Option<String>,
    pub release_notes: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("invalid semver version '{0}'")]
    InvalidVersion(String),
    #[error("empty platform")]
    EmptyPlatform,
    #[error("missing App Store URL for platform '{0}'")]
    MissingStoreUrl(String),
    #[error("database error: {0}")]
    Database(#[from] RepositoryError),
}

pub async fn ingest_app_store_release(
    repo: &dyn AppReleaseRepository,
    event: AppStoreReleaseUpdatedData,
    fallback_store_url: Option<&str>,
) -> Result<(), IngestError> {
    if event.platform.trim().is_empty() {
        return Err(IngestError::EmptyPlatform);
    }

    if semver::Version::parse(&event.version).is_err() {
        return Err(IngestError::InvalidVersion(event.version));
    }

    let store_url = event
        .app_store_url
        .or_else(|| fallback_store_url.map(|s| s.to_string()))
        .ok_or_else(|| IngestError::MissingStoreUrl(event.platform.clone()))?;

    let upsert = StoreReleaseUpsert {
        platform: event.platform,
        version: event.version,
        store_url,
        release_notes: event.release_notes,
    };

    repo.upsert_store_release(upsert).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::app_release;
    use core_services::db::repository::abstraction::errors::RepositoryError;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeRepo {
        calls: Mutex<Vec<StoreReleaseUpsert>>,
    }

    #[async_trait::async_trait]
    impl AppReleaseRepository for FakeRepo {
        async fn upsert_store_release(&self, row: StoreReleaseUpsert) -> Result<(), RepositoryError> {
            self.calls.lock().unwrap().push(row);
            Ok(())
        }
        async fn latest_for_platform(&self, _platform: &str) -> Result<Option<app_release::Model>, RepositoryError> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn rejects_invalid_semver() {
        let repo = FakeRepo::default();
        let event = AppStoreReleaseUpdatedData {
            platform: "darwin".into(),
            version: "not-a-version".into(),
            app_store_url: Some("https://apps.apple.com/app/bytover/id1".into()),
            release_notes: None,
        };
        let err = ingest_app_store_release(&repo, event, None).await.unwrap_err();
        assert!(matches!(err, IngestError::InvalidVersion(_)));
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_empty_platform() {
        let repo = FakeRepo::default();
        let event = AppStoreReleaseUpdatedData {
            platform: "   ".into(),
            version: "1.0.0".into(),
            app_store_url: Some("https://apps.apple.com/app/bytover/id1".into()),
            release_notes: None,
        };
        let err = ingest_app_store_release(&repo, event, None).await.unwrap_err();
        assert!(matches!(err, IngestError::EmptyPlatform));
    }

    #[tokio::test]
    async fn uses_event_store_url_when_present() {
        let repo = FakeRepo::default();
        let event = AppStoreReleaseUpdatedData {
            platform: "darwin".into(),
            version: "2.0.0".into(),
            app_store_url: Some("https://apps.apple.com/app/bytover/id1234567890".into()),
            release_notes: Some("notes".into()),
        };
        ingest_app_store_release(&repo, event, Some("https://fallback")).await.unwrap();
        let calls = repo.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].platform, "darwin");
        assert_eq!(calls[0].version, "2.0.0");
        assert_eq!(calls[0].store_url, "https://apps.apple.com/app/bytover/id1234567890");
        assert_eq!(calls[0].release_notes.as_deref(), Some("notes"));
    }

    #[tokio::test]
    async fn falls_back_to_configured_url_when_event_omits_it() {
        let repo = FakeRepo::default();
        let event = AppStoreReleaseUpdatedData {
            platform: "darwin".into(),
            version: "2.0.0".into(),
            app_store_url: None,
            release_notes: None,
        };
        ingest_app_store_release(&repo, event, Some("https://apps.apple.com/app/bytover/id0000000000")).await.unwrap();
        let calls = repo.calls.lock().unwrap();
        assert_eq!(calls[0].store_url, "https://apps.apple.com/app/bytover/id0000000000");
    }

    #[tokio::test]
    async fn errors_when_no_store_url_available() {
        let repo = FakeRepo::default();
        let event = AppStoreReleaseUpdatedData {
            platform: "darwin".into(),
            version: "2.0.0".into(),
            app_store_url: None,
            release_notes: None,
        };
        let err = ingest_app_store_release(&repo, event, None).await.unwrap_err();
        assert!(matches!(err, IngestError::MissingStoreUrl(_)));
    }
}
