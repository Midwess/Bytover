use crate::config::AppStoreConfig;
use crate::di_container::DiContainer;
use crate::http::webhooks::asc_api::{AppStoreConnectApi, AscApiError};
use crate::http::webhooks::events::{
    classify, map_apple_platform, AppStoreConnectEvent, AppStoreVersionStateUpdate,
    EventParseError, WebhookEnvelope, APP_STORE_STATE_READY_FOR_SALE,
};
use crate::http::webhooks::ingestor::{
    ingest_app_store_release, AppStoreReleaseUpdatedData, IngestError,
};
use crate::http::webhooks::verify::{VerifyError, WebhookSecretVerifier};
use crate::repositories::app_release::AppReleaseRepository;
use axum::body::Bytes;
use http::{HeaderMap, StatusCode};

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
    fn status(self) -> StatusCode {
        match self {
            HandlerOutcome::Accepted | HandlerOutcome::Ignored => StatusCode::OK,
            HandlerOutcome::Skipped => StatusCode::SERVICE_UNAVAILABLE,
            HandlerOutcome::Unauthorized => StatusCode::UNAUTHORIZED,
            HandlerOutcome::BadRequest => StatusCode::BAD_REQUEST,
            HandlerOutcome::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub async fn process_webhook(
    headers: &HeaderMap,
    body: &[u8],
    verifier: Option<&WebhookSecretVerifier>,
    config: &AppStoreConfig,
    repo: &dyn AppReleaseRepository,
    api: Option<&dyn AppStoreConnectApi>,
) -> HandlerOutcome {
    log_inbound_request(headers, body);

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
        Err(EventParseError::MissingInstanceId(kind)) => {
            log::warn!("Webhook payload missing instance id for {}", kind);
            return HandlerOutcome::BadRequest;
        }
    };

    match event {
        AppStoreConnectEvent::AppStoreVersionStateUpdated(update) => {
            handle_version_state_update(update, envelope.data.id.as_str(), config, repo, api).await
        }
        AppStoreConnectEvent::WebhookPing => {
            log::info!("Acknowledging webhook ping: id={}", envelope.data.id);
            HandlerOutcome::Ignored
        }
        AppStoreConnectEvent::BuildUploadStateUpdated
        | AppStoreConnectEvent::ExternalBuildStateUpdated
        | AppStoreConnectEvent::BetaFeedback => {
            log::info!(
                "Ignoring non-release event: type={}, id={}",
                envelope.data.event_type,
                envelope.data.id,
            );
            HandlerOutcome::Ignored
        }
        AppStoreConnectEvent::Unknown(ref t) => {
            log::info!("Ignoring unknown notification type: {}", t);
            HandlerOutcome::Ignored
        }
    }
}

fn log_inbound_request(headers: &HeaderMap, body: &[u8]) {
    let header_dump: Vec<String> = headers
        .iter()
        .map(|(name, value)| {
            let v = value.to_str().unwrap_or("<non-ascii>");
            format!("{}: {}", name, v)
        })
        .collect();
    log::info!(
        "App Store Connect webhook inbound: headers=[{}]",
        header_dump.join(" | "),
    );
    match std::str::from_utf8(body) {
        Ok(s) => log::info!("App Store Connect webhook body ({} bytes): {}", body.len(), s),
        Err(_) => log::info!(
            "App Store Connect webhook body ({} bytes, non-utf8): {}",
            body.len(),
            hex::encode(body),
        ),
    }
}

async fn handle_version_state_update(
    update: AppStoreVersionStateUpdate,
    delivery_id: &str,
    config: &AppStoreConfig,
    repo: &dyn AppReleaseRepository,
    api: Option<&dyn AppStoreConnectApi>,
) -> HandlerOutcome {
    let new_value = update.new_value.as_deref().unwrap_or("");
    if new_value != APP_STORE_STATE_READY_FOR_SALE {
        log::info!(
            "Ignoring version state transition delivery_id={} app_store_version_id={} {:?} -> {:?}",
            delivery_id,
            update.app_store_version_id,
            update.old_value,
            update.new_value,
        );
        return HandlerOutcome::Ignored;
    }

    let Some(api) = api else {
        log::error!(
            "Cannot ingest READY_FOR_SALE event delivery_id={}: App Store Connect API credentials not configured",
            delivery_id,
        );
        return HandlerOutcome::Skipped;
    };

    let info = match api.fetch_app_store_version(&update.app_store_version_id).await {
        Ok(info) => info,
        Err(err) => {
            log::error!(
                "Failed to fetch app store version delivery_id={} id={}: {}",
                delivery_id,
                update.app_store_version_id,
                err,
            );
            return match &err {
                AscApiError::Status(status) => match status.as_u16() {
                    401 | 403 | 408 | 429 => HandlerOutcome::Skipped,
                    code if (500..=599).contains(&code) => HandlerOutcome::Skipped,
                    _ => HandlerOutcome::BadRequest,
                },
                AscApiError::JwtSigning(_)
                | AscApiError::Http(_)
                | AscApiError::MissingField(_) => HandlerOutcome::Skipped,
            };
        }
    };

    let Some(platform) = map_apple_platform(&info.apple_platform) else {
        log::warn!(
            "Ignoring unsupported Apple platform '{}' delivery_id={}",
            info.apple_platform,
            delivery_id,
        );
        return HandlerOutcome::Ignored;
    };

    log::info!(
        "Ingesting App Store release: platform={}, version={}, delivery_id={}",
        platform,
        info.version_string,
        delivery_id,
    );

    let fallback_url = config.default_store_url_for(platform);
    let event = AppStoreReleaseUpdatedData {
        platform: platform.to_string(),
        version: info.version_string,
        app_store_url: None,
        release_notes: None,
    };

    match ingest_app_store_release(repo, event, fallback_url).await {
        Ok(()) => HandlerOutcome::Accepted,
        Err(IngestError::InvalidVersion(v)) => {
            log::warn!("Rejecting non-semver version: {}", v);
            HandlerOutcome::BadRequest
        }
        Err(IngestError::EmptyPlatform) => HandlerOutcome::BadRequest,
        Err(IngestError::MissingStoreUrl(p)) => {
            log::error!(
                "No App Store URL configured for platform {} delivery_id={}; returning Skipped so Apple retries",
                p,
                delivery_id,
            );
            HandlerOutcome::Skipped
        }
        Err(IngestError::Database(e)) => {
            log::error!("Webhook upsert failed: {:?}", e);
            HandlerOutcome::InternalError
        }
    }
}

pub async fn handle(headers: HeaderMap, body: Bytes) -> StatusCode {
    let di = DiContainer::instance().await;
    let config = di.get_app_store_config();
    let verifier = di.get_webhook_verifier();
    let repo = di.get_app_release_repository().await;
    let api = di.get_app_store_connect_api();

    process_webhook(
        &headers,
        body.as_ref(),
        verifier.as_ref(),
        config,
        &repo,
        api.as_deref(),
    )
    .await
    .status()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppStoreConnectApiCredentials;
    use crate::entities::app_release;
    use crate::http::webhooks::asc_api::AppStoreVersionInfo;
    use crate::http::webhooks::verify::sign;
    use crate::repositories::app_release::{AppReleaseRepository, StoreReleaseUpsert};
    use http::header::{HeaderMap, HeaderName, HeaderValue};
    use async_trait::async_trait;
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

    struct FakeApi {
        info: AppStoreVersionInfo,
        failure_status: Option<http::StatusCode>,
        last_id: Mutex<Option<String>>,
    }

    impl FakeApi {
        fn ready(version: &str, apple_platform: &str) -> Self {
            Self {
                info: AppStoreVersionInfo {
                    version_string: version.to_string(),
                    apple_platform: apple_platform.to_string(),
                },
                failure_status: None,
                last_id: Mutex::new(None),
            }
        }

        fn failing(status: http::StatusCode) -> Self {
            Self {
                info: AppStoreVersionInfo {
                    version_string: "0.0.0".into(),
                    apple_platform: "MAC_OS".into(),
                },
                failure_status: Some(status),
                last_id: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl AppStoreConnectApi for FakeApi {
        async fn fetch_app_store_version(
            &self,
            id: &str,
        ) -> Result<AppStoreVersionInfo, AscApiError> {
            *self.last_id.lock().unwrap() = Some(id.to_string());
            if let Some(status) = self.failure_status {
                return Err(AscApiError::Status(reqwest::StatusCode::from_u16(status.as_u16()).unwrap()));
            }
            Ok(self.info.clone())
        }
    }

    fn test_config() -> AppStoreConfig {
        AppStoreConfig {
            webhook_secret: Some(b"test-secret".to_vec()),
            force_update_enabled: true,
            default_store_url_darwin: Some("https://apps.apple.com/app/bytover/id0000000000".into()),
            default_store_url_ios: None,
            connect_api: Some(AppStoreConnectApiCredentials {
                issuer_id: "issuer".into(),
                key_id: "key".into(),
                private_key_pem: "pem".into(),
            }),
            connect_api_base_url: "https://api.appstoreconnect.apple.com".into(),
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

    fn ready_for_sale_body(instance_id: &str) -> Vec<u8> {
        format!(
            r#"{{
                "data": {{
                    "type": "appStoreVersionAppVersionStateUpdated",
                    "id": "evt-1",
                    "attributes": {{ "newValue": "READY_FOR_SALE", "oldValue": "PENDING_DEVELOPER_RELEASE" }},
                    "relationships": {{
                        "instance": {{
                            "data": {{ "type": "appStoreVersions", "id": "{}" }}
                        }}
                    }}
                }}
            }}"#,
            instance_id
        )
        .into_bytes()
    }

    #[tokio::test]
    async fn accepts_ready_for_sale_event_and_calls_api() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::ready("2.0.0", "MAC_OS");

        let body = ready_for_sale_body("asv-9");
        let headers = signed_headers(&body);

        let outcome = process_webhook(
            &headers,
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;

        assert_eq!(outcome, HandlerOutcome::Accepted);
        assert_eq!(api.last_id.lock().unwrap().as_deref(), Some("asv-9"));
        let calls = repo.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].platform, "darwin");
        assert_eq!(calls[0].version, "2.0.0");
        assert!(calls[0].store_url.starts_with("https://apps.apple.com/"));
    }

    #[tokio::test]
    async fn ignores_non_ready_state_transitions() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::ready("2.0.0", "MAC_OS");

        let body = br#"{
            "data": {
                "type": "appStoreVersionAppVersionStateUpdated",
                "id": "evt-2",
                "attributes": { "newValue": "IN_REVIEW", "oldValue": "WAITING_FOR_REVIEW" },
                "relationships": { "instance": { "data": { "type": "appStoreVersions", "id": "asv-1" } } }
            }
        }"#;
        let headers = signed_headers(body);

        let outcome = process_webhook(
            &headers,
            body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;

        assert_eq!(outcome, HandlerOutcome::Ignored);
        assert!(api.last_id.lock().unwrap().is_none());
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn webhook_ping_is_acknowledged_without_db_write() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::ready("2.0.0", "MAC_OS");

        let body = br#"{"data":{"type":"webhookPingCreated","id":"ping-1"}}"#;
        let headers = signed_headers(body);

        let outcome = process_webhook(
            &headers,
            body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;

        assert_eq!(outcome, HandlerOutcome::Ignored);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn build_and_feedback_events_are_ignored() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::ready("2.0.0", "MAC_OS");

        for ty in [
            "buildUploadStateUpdated",
            "buildBetaDetailExternalBuildStateUpdated",
            "betaFeedbackScreenshotSubmissionCreated",
            "betaFeedbackCrashSubmissionCreated",
        ] {
            let body = format!(r#"{{"data":{{"type":"{}","id":"e"}}}}"#, ty).into_bytes();
            let headers = signed_headers(&body);
            let outcome = process_webhook(
                &headers,
                &body,
                Some(&verifier),
                &test_config(),
                &repo,
                Some(&api),
            )
            .await;
            assert_eq!(outcome, HandlerOutcome::Ignored, "{} should be Ignored", ty);
        }
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unsigned_request_is_rejected() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::ready("2.0.0", "MAC_OS");
        let body = ready_for_sale_body("asv-1");
        let outcome = process_webhook(
            &HeaderMap::new(),
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;
        assert_eq!(outcome, HandlerOutcome::Unauthorized);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn missing_secret_returns_skipped() {
        let repo = FakeRepo::default();
        let api = FakeApi::ready("2.0.0", "MAC_OS");
        let body = ready_for_sale_body("asv-1");
        let headers = signed_headers(&body);
        let outcome =
            process_webhook(&headers, &body, None, &test_config(), &repo, Some(&api)).await;
        assert_eq!(outcome, HandlerOutcome::Skipped);
    }

    #[tokio::test]
    async fn ready_event_without_api_credentials_is_skipped() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let body = ready_for_sale_body("asv-1");
        let headers = signed_headers(&body);
        let outcome = process_webhook(
            &headers,
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            None,
        )
        .await;
        assert_eq!(outcome, HandlerOutcome::Skipped);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn unsupported_apple_platform_is_ignored() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::ready("2.0.0", "ANDROID");
        let body = ready_for_sale_body("asv-1");
        let headers = signed_headers(&body);
        let outcome = process_webhook(
            &headers,
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;
        assert_eq!(outcome, HandlerOutcome::Ignored);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn malformed_json_is_bad_request() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::ready("2.0.0", "MAC_OS");
        let body = b"not-json";
        let headers = signed_headers(body);
        let outcome = process_webhook(
            &headers,
            body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;
        assert_eq!(outcome, HandlerOutcome::BadRequest);
    }

    #[tokio::test]
    async fn skipped_outcome_maps_to_service_unavailable() {
        assert_eq!(HandlerOutcome::Skipped.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn ios_release_without_default_url_is_skipped_for_retry() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::ready("3.0.0", "IOS");
        let body = ready_for_sale_body("asv-ios");
        let headers = signed_headers(&body);

        let outcome = process_webhook(
            &headers,
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;

        assert_eq!(outcome, HandlerOutcome::Skipped);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn asc_api_unauthorized_is_skipped_so_apple_retries() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::failing(http::StatusCode::UNAUTHORIZED);
        let body = ready_for_sale_body("asv-1");
        let headers = signed_headers(&body);

        let outcome = process_webhook(
            &headers,
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;

        assert_eq!(outcome, HandlerOutcome::Skipped);
        assert!(repo.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn asc_api_forbidden_is_skipped() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::failing(http::StatusCode::FORBIDDEN);
        let body = ready_for_sale_body("asv-1");
        let headers = signed_headers(&body);

        let outcome = process_webhook(
            &headers,
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;

        assert_eq!(outcome, HandlerOutcome::Skipped);
    }

    #[tokio::test]
    async fn asc_api_rate_limit_is_skipped() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::failing(http::StatusCode::TOO_MANY_REQUESTS);
        let body = ready_for_sale_body("asv-1");
        let headers = signed_headers(&body);

        let outcome = process_webhook(
            &headers,
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;

        assert_eq!(outcome, HandlerOutcome::Skipped);
    }

    #[tokio::test]
    async fn asc_api_server_error_is_skipped() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::failing(http::StatusCode::INTERNAL_SERVER_ERROR);
        let body = ready_for_sale_body("asv-1");
        let headers = signed_headers(&body);

        let outcome = process_webhook(
            &headers,
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;

        assert_eq!(outcome, HandlerOutcome::Skipped);
    }

    #[tokio::test]
    async fn asc_api_not_found_is_bad_request() {
        let repo = FakeRepo::default();
        let verifier = WebhookSecretVerifier::new(b"test-secret".to_vec());
        let api = FakeApi::failing(http::StatusCode::NOT_FOUND);
        let body = ready_for_sale_body("asv-1");
        let headers = signed_headers(&body);

        let outcome = process_webhook(
            &headers,
            &body,
            Some(&verifier),
            &test_config(),
            &repo,
            Some(&api),
        )
        .await;

        assert_eq!(outcome, HandlerOutcome::BadRequest);
    }
}
