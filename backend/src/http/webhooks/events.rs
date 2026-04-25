use serde::Deserialize;

pub const APP_STORE_VERSION_APP_VERSION_STATE_UPDATED: &str = "appStoreVersionAppVersionStateUpdated";
pub const WEBHOOK_PING_CREATED: &str = "webhookPingCreated";
pub const BUILD_UPLOAD_STATE_UPDATED: &str = "buildUploadStateUpdated";
pub const BUILD_BETA_DETAIL_EXTERNAL_BUILD_STATE_UPDATED: &str =
    "buildBetaDetailExternalBuildStateUpdated";
pub const BETA_FEEDBACK_SCREENSHOT_SUBMISSION_CREATED: &str =
    "betaFeedbackScreenshotSubmissionCreated";
pub const BETA_FEEDBACK_CRASH_SUBMISSION_CREATED: &str = "betaFeedbackCrashSubmissionCreated";

pub const APP_STORE_STATE_READY_FOR_SALE: &str = "READY_FOR_SALE";

#[derive(Debug, Deserialize)]
pub struct WebhookEnvelope {
    pub data: WebhookEventData,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEventData {
    #[serde(rename = "type")]
    pub event_type: String,
    pub id: String,
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
    #[serde(default)]
    pub relationships: Option<serde_json::Value>,
}

impl WebhookEventData {
    pub fn instance_id(&self) -> Option<String> {
        self.relationships
            .as_ref()?
            .get("instance")?
            .get("data")?
            .get("id")?
            .as_str()
            .map(|s| s.to_string())
    }
}

#[derive(Debug)]
pub enum AppStoreConnectEvent {
    AppStoreVersionStateUpdated(AppStoreVersionStateUpdate),
    WebhookPing,
    BuildUploadStateUpdated,
    ExternalBuildStateUpdated,
    BetaFeedback,
    Unknown(String),
}

#[derive(Debug)]
pub struct AppStoreVersionStateUpdate {
    pub app_store_version_id: String,
    pub new_value: Option<String>,
    pub old_value: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum EventParseError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("missing relationships.instance.data.id for {0}")]
    MissingInstanceId(&'static str),
}

pub fn classify(envelope: &WebhookEnvelope) -> Result<AppStoreConnectEvent, EventParseError> {
    match envelope.data.event_type.as_str() {
        APP_STORE_VERSION_APP_VERSION_STATE_UPDATED => {
            let app_store_version_id = envelope
                .data
                .instance_id()
                .ok_or(EventParseError::MissingInstanceId(APP_STORE_VERSION_APP_VERSION_STATE_UPDATED))?;
            let attributes = envelope.data.attributes.as_ref();
            let new_value = attributes
                .and_then(|a| a.get("newValue"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let old_value = attributes
                .and_then(|a| a.get("oldValue"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Ok(AppStoreConnectEvent::AppStoreVersionStateUpdated(
                AppStoreVersionStateUpdate {
                    app_store_version_id,
                    new_value,
                    old_value,
                },
            ))
        }
        WEBHOOK_PING_CREATED => Ok(AppStoreConnectEvent::WebhookPing),
        BUILD_UPLOAD_STATE_UPDATED => Ok(AppStoreConnectEvent::BuildUploadStateUpdated),
        BUILD_BETA_DETAIL_EXTERNAL_BUILD_STATE_UPDATED => {
            Ok(AppStoreConnectEvent::ExternalBuildStateUpdated)
        }
        BETA_FEEDBACK_SCREENSHOT_SUBMISSION_CREATED
        | BETA_FEEDBACK_CRASH_SUBMISSION_CREATED => Ok(AppStoreConnectEvent::BetaFeedback),
        other => Ok(AppStoreConnectEvent::Unknown(other.to_string())),
    }
}

pub fn map_apple_platform(apple_platform: &str) -> Option<&'static str> {
    match apple_platform {
        "MAC_OS" => Some("darwin"),
        "IOS" => Some("ios"),
        "TV_OS" => Some("tvos"),
        "VISION_OS" => Some("visionos"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_app_store_version_state_updated_body() -> &'static str {
        r#"{
            "data": {
                "type": "appStoreVersionAppVersionStateUpdated",
                "id": "evt-1",
                "version": 1,
                "attributes": {
                    "newValue": "READY_FOR_SALE",
                    "oldValue": "PENDING_DEVELOPER_RELEASE",
                    "timestamp": "2026-04-26T10:00:00Z"
                },
                "relationships": {
                    "instance": {
                        "data": { "type": "appStoreVersions", "id": "asv-123" },
                        "links": { "self": "https://api.appstoreconnect.apple.com/v1/appStoreVersions/asv-123" }
                    }
                }
            }
        }"#
    }

    #[test]
    fn parses_app_store_version_state_updated() {
        let envelope: WebhookEnvelope =
            serde_json::from_str(build_app_store_version_state_updated_body()).unwrap();
        let evt = classify(&envelope).unwrap();
        match evt {
            AppStoreConnectEvent::AppStoreVersionStateUpdated(update) => {
                assert_eq!(update.app_store_version_id, "asv-123");
                assert_eq!(update.new_value.as_deref(), Some("READY_FOR_SALE"));
                assert_eq!(update.old_value.as_deref(), Some("PENDING_DEVELOPER_RELEASE"));
            }
            other => panic!("wrong variant: {:?}", other),
        }
    }

    #[test]
    fn classifies_webhook_ping() {
        let body = r#"{"data":{"type":"webhookPingCreated","id":"ping-1"}}"#;
        let envelope: WebhookEnvelope = serde_json::from_str(body).unwrap();
        let evt = classify(&envelope).unwrap();
        assert!(matches!(evt, AppStoreConnectEvent::WebhookPing));
    }

    #[test]
    fn classifies_build_upload_and_external_and_feedback() {
        for ty in [
            BUILD_UPLOAD_STATE_UPDATED,
            BUILD_BETA_DETAIL_EXTERNAL_BUILD_STATE_UPDATED,
            BETA_FEEDBACK_SCREENSHOT_SUBMISSION_CREATED,
            BETA_FEEDBACK_CRASH_SUBMISSION_CREATED,
        ] {
            let body = format!(r#"{{"data":{{"type":"{}","id":"e"}}}}"#, ty);
            let envelope: WebhookEnvelope = serde_json::from_str(&body).unwrap();
            let evt = classify(&envelope).unwrap();
            assert!(matches!(
                evt,
                AppStoreConnectEvent::BuildUploadStateUpdated
                    | AppStoreConnectEvent::ExternalBuildStateUpdated
                    | AppStoreConnectEvent::BetaFeedback
            ));
        }
    }

    #[test]
    fn unknown_event_classifies_as_unknown() {
        let body = r#"{"data":{"type":"someFutureEvent","id":"x"}}"#;
        let envelope: WebhookEnvelope = serde_json::from_str(body).unwrap();
        let evt = classify(&envelope).unwrap();
        assert!(matches!(evt, AppStoreConnectEvent::Unknown(ref s) if s == "someFutureEvent"));
    }

    #[test]
    fn missing_instance_id_errors() {
        let body = r#"{"data":{"type":"appStoreVersionAppVersionStateUpdated","id":"e","attributes":{"newValue":"READY_FOR_SALE"}}}"#;
        let envelope: WebhookEnvelope = serde_json::from_str(body).unwrap();
        let err = classify(&envelope).unwrap_err();
        assert!(matches!(err, EventParseError::MissingInstanceId(_)));
    }

    #[test]
    fn maps_apple_platforms() {
        assert_eq!(map_apple_platform("MAC_OS"), Some("darwin"));
        assert_eq!(map_apple_platform("IOS"), Some("ios"));
        assert_eq!(map_apple_platform("ANDROID"), None);
    }
}
