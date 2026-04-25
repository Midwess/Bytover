use crate::config::AppStoreConfig;
use crate::di_container::DiContainer;
use crate::http::webhooks::events::{classify, AppStoreConnectEvent, EventParseError, WebhookEnvelope};
use crate::http::webhooks::ingestor::{ingest_app_store_release, IngestError};
use crate::http::webhooks::verify::{VerifyError, WebhookSecretVerifier};
use crate::repositories::app_release::AppReleaseRepository;
use actix_web::http::header::HeaderMap;
use actix_web::{post, web, HttpRequest, HttpResponse};

#[derive(Debug, PartialEq, Eq)]
pub enum HandlerOutcome {
    Accepted,
    Ignored,
    Skipped,
    Unauthorized,
    BadRequest,
    InternalError,
}

impl HandlerOutcome {
    fn into_response(self) -> HttpResponse {
        match self {
            HandlerOutcome::Accepted | HandlerOutcome::Ignored => HttpResponse::Ok().finish(),
            HandlerOutcome::Skipped => HttpResponse::ServiceUnavailable().finish(),
            HandlerOutcome::Unauthorized => HttpResponse::Unauthorized().finish(),
            HandlerOutcome::BadRequest => HttpResponse::BadRequest().finish(),
            HandlerOutcome::InternalError => HttpResponse::InternalServerError().finish(),
        }
    }
}

pub async fn process_webhook(
    headers: &HeaderMap,
    body: &[u8],
    verifier: Option<&WebhookSecretVerifier>,
    config: &AppStoreConfig,
    repo: &dyn AppReleaseRepository,
) -> HandlerOutcome {
    let Some(verifier) = verifier else {
        log::warn!(
            "APP_STORE_CONNECT_WEBHOOK_SECRET not set; rejecting inbound webhook with 503 so Apple retries"
        );
        return HandlerOutcome::Skipped;
    };

    if let Err(err) = verifier.verify(headers, body) {
        log::warn!("Webhook verification failed: {}", err);
        return match err {
            VerifyError::MissingSignature
            | VerifyError::MalformedSignature
            | VerifyError::SignatureMismatch => HandlerOutcome::Unauthorized,
        };
    }

    let envelope: WebhookEnvelope = match serde_json::from_slice(body) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("Webhook body is not valid JSON envelope: {}", e);
            return HandlerOutcome::BadRequest;
        }
    };

    let event = match classify(&envelope) {
        Ok(e) => e,
        Err(EventParseError::InvalidJson(e)) => {
            log::warn!("Webhook payload failed serde: {}", e);
            return HandlerOutcome::BadRequest;
        }
        Err(EventParseError::MissingData(kind)) => {
            log::warn!("Webhook payload missing data for {}", kind);
            return HandlerOutcome::BadRequest;
        }
    };

    match event {
        AppStoreConnectEvent::AppStoreReleaseUpdated(data) => {
            log::info!(
                "Ingesting App Store release: platform={}, version={}, delivery_id={:?}",
                data.platform,
                data.version,
                envelope.notification_id,
            );
            let fallback_url = config.default_store_url_for(&data.platform);
            match ingest_app_store_release(repo, data, fallback_url).await {
                Ok(()) => HandlerOutcome::Accepted,
                Err(IngestError::InvalidVersion(v)) => {
                    log::warn!("Rejecting non-semver version: {}", v);
                    HandlerOutcome::BadRequest
                }
                Err(IngestError::EmptyPlatform) => HandlerOutcome::BadRequest,
                Err(IngestError::MissingStoreUrl(p)) => {
                    log::error!("No App Store URL configured for platform {}", p);
                    HandlerOutcome::BadRequest
                }
                Err(IngestError::Database(e)) => {
                    log::error!("Webhook upsert failed: {:?}", e);
                    HandlerOutcome::InternalError
                }
            }
        }
        AppStoreConnectEvent::TestFlightExternalUpdated
        | AppStoreConnectEvent::TestFlightInternalCreated
        | AppStoreConnectEvent::AssetPackVersionUpdated => {
            log::info!(
                "Ignoring non-release event: type={}, delivery_id={:?}",
                envelope.notification_type,
                envelope.notification_id,
            );
            HandlerOutcome::Ignored
        }
        AppStoreConnectEvent::WebhookPing => {
            log::info!(
                "Acknowledging webhook test ping: type={}, delivery_id={:?}",
                envelope.notification_type,
                envelope.notification_id,
            );
            HandlerOutcome::Ignored
        }
        AppStoreConnectEvent::Unknown(ref t) => {
            log::info!("Ignoring unknown notification type: {}", t);
            HandlerOutcome::Ignored
        }
    }
}

#[post("/webhooks/app-store-connect")]
pub async fn handle(req: HttpRequest, body: web::Bytes) -> actix_web::Result<HttpResponse> {
    let di = DiContainer::instance().await;
    let config = di.get_app_store_config();
    let verifier = di.get_webhook_verifier();
    let repo = di.get_app_release_repository().await;

    let outcome = process_webhook(
        req.headers(),
        body.as_ref(),
        verifier.as_ref(),
        config,
        &repo,
    )
    .await;

    Ok(outcome.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::app_release;
    use crate::http::webhooks::verify::sign;
    use crate::repositories::app_release::{AppReleaseRepository, StoreReleaseUpsert};
    use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
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

    fn test_config() -> AppStoreConfig {
        AppStoreConfig {
            webhook_secret: Some(b"test-secret".to_vec()),
            force_update_enabled: true,
            default_store_url_darwin: Some("https://apps.apple.com/app/bytover/id0000000000".into()),
            default_store_url_ios: None,
        }
    }

    fn signed_headers(body: &[u8]) -> HeaderMap {
        let sig = sign(b"test-secret", body);
        let mut h = HeaderMap::new();
        h.insert(
            HeaderName::from_static("x-apple-signature"),
            HeaderValue::from_str(&sig).unwrap(),
        );
        h
    }

    #[tokio::test]
    async fn accepts_valid_app_store_release() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = br#"{
            "notificationType":"APP_STORE_RELEASE_UPDATED",
            "notificationId":"abc",
            "data":{"platform":"darwin","version":"2.0.0"}
        }"#;
        let headers = signed_headers(body);
        let outcome = process_webhook(&headers, body, Some(&verifier), &test_config(), &repo).await;
        assert_eq!(outcome, HandlerOutcome::Accepted);
        let calls = repo.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].platform, "darwin");
        assert_eq!(calls[0].version, "2.0.0");
        assert!(calls[0].store_url.starts_with("https://apps.apple.com/"));
    }

    #[tokio::test]
    async fn redelivery_is_idempotent_at_handler_level() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = br#"{
            "notificationType":"APP_STORE_RELEASE_UPDATED",
            "notificationId":"abc",
            "data":{"platform":"darwin","version":"2.0.0"}
        }"#;
        let headers = signed_headers(body);
        let a = process_webhook(&headers, body, Some(&verifier), &test_config(), &repo).await;
        let b = process_webhook(&headers, body, Some(&verifier), &test_config(), &repo).await;
        assert_eq!(a, HandlerOutcome::Accepted);
        assert_eq!(b, HandlerOutcome::Accepted);
    }

    #[tokio::test]
    async fn testflight_events_ignored_without_db_write() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = br#"{"notificationType":"EXTERNAL_TESTFLIGHT_RELEASE_UPDATED"}"#;
        let headers = signed_headers(body);
        let outcome = process_webhook(&headers, body, Some(&verifier), &test_config(), &repo).await;
        assert_eq!(outcome, HandlerOutcome::Ignored);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn webhook_test_ping_is_acknowledged_without_db_write() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = br#"{"notificationType":"TEST","notificationId":"ping-1"}"#;
        let headers = signed_headers(body);
        let outcome = process_webhook(&headers, body, Some(&verifier), &test_config(), &repo).await;
        assert_eq!(outcome, HandlerOutcome::Ignored);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn webhook_ping_alias_is_acknowledged_without_db_write() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = br#"{"notificationType":"PING","notificationId":"ping-2"}"#;
        let headers = signed_headers(body);
        let outcome = process_webhook(&headers, body, Some(&verifier), &test_config(), &repo).await;
        assert_eq!(outcome, HandlerOutcome::Ignored);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unsigned_test_ping_is_rejected() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = br#"{"notificationType":"TEST"}"#;
        let outcome = process_webhook(
            &HeaderMap::new(),
            body,
            Some(&verifier),
            &test_config(),
            &repo,
        )
        .await;
        assert_eq!(outcome, HandlerOutcome::Unauthorized);
    }

    #[tokio::test]
    async fn asset_pack_events_ignored() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = br#"{"notificationType":"ASSET_PACK_VERSION_UPDATED"}"#;
        let headers = signed_headers(body);
        let outcome = process_webhook(&headers, body, Some(&verifier), &test_config(), &repo).await;
        assert_eq!(outcome, HandlerOutcome::Ignored);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unsigned_request_is_rejected() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = br#"{"notificationType":"APP_STORE_RELEASE_UPDATED"}"#;
        let headers = HeaderMap::new();
        let outcome = process_webhook(
            &headers,
            body,
            Some(&verifier),
            &test_config(),
            &repo,
        )
        .await;
        assert_eq!(outcome, HandlerOutcome::Unauthorized);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn no_secret_configured_returns_skipped_outcome() {
        let repo = FakeRepo::default();
        let body = br#"{"notificationType":"APP_STORE_RELEASE_UPDATED","data":{"platform":"darwin","version":"2.0.0"}}"#;
        let headers = signed_headers(body);
        let outcome = process_webhook(&headers, body, None, &test_config(), &repo).await;
        assert_eq!(outcome, HandlerOutcome::Skipped);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn skipped_outcome_maps_to_service_unavailable() {
        assert_eq!(HandlerOutcome::Skipped.into_response().status(), actix_web::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn non_semver_version_is_bad_request() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = br#"{
            "notificationType":"APP_STORE_RELEASE_UPDATED",
            "data":{"platform":"darwin","version":"not-a-semver"}
        }"#;
        let headers = signed_headers(body);
        let outcome = process_webhook(&headers, body, Some(&verifier), &test_config(), &repo).await;
        assert_eq!(outcome, HandlerOutcome::BadRequest);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn malformed_json_is_bad_request() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = b"not-json";
        let headers = signed_headers(body);
        let outcome = process_webhook(&headers, body, Some(&verifier), &test_config(), &repo).await;
        assert_eq!(outcome, HandlerOutcome::BadRequest);
    }
}
