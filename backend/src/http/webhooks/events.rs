use serde::Deserialize;

pub const APP_STORE_RELEASE_UPDATED: &str = "APP_STORE_RELEASE_UPDATED";
pub const EXTERNAL_TESTFLIGHT_RELEASE_UPDATED: &str = "EXTERNAL_TESTFLIGHT_RELEASE_UPDATED";
pub const INTERNAL_TESTFLIGHT_RELEASE_CREATED: &str = "INTERNAL_TESTFLIGHT_RELEASE_CREATED";
pub const ASSET_PACK_VERSION_UPDATED: &str = "ASSET_PACK_VERSION_UPDATED";
pub const WEBHOOK_TEST: &str = "TEST";
pub const WEBHOOK_PING: &str = "PING";

#[derive(Debug, Deserialize)]
pub struct WebhookEnvelope {
    #[serde(rename = "notificationType")]
    pub notification_type: String,
    #[serde(rename = "notificationId")]
    pub notification_id: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AppStoreReleaseUpdatedData {
    pub platform: String,
    pub version: String,
    #[serde(rename = "appStoreUrl")]
    pub app_store_url: Option<String>,
    #[serde(rename = "releaseNotes")]
    pub release_notes: Option<String>,
}

#[derive(Debug)]
pub enum AppStoreConnectEvent {
    AppStoreReleaseUpdated(AppStoreReleaseUpdatedData),
    TestFlightExternalUpdated,
    TestFlightInternalCreated,
    AssetPackVersionUpdated,
    WebhookPing,
    Unknown(String),
}

#[derive(Debug, thiserror::Error)]
pub enum EventParseError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("missing data payload for {0}")]
    MissingData(&'static str),
}

pub fn classify(envelope: &WebhookEnvelope) -> Result<AppStoreConnectEvent, EventParseError> {
    match envelope.notification_type.as_str() {
        APP_STORE_RELEASE_UPDATED => {
            let raw = envelope
                .data
                .clone()
                .ok_or(EventParseError::MissingData(APP_STORE_RELEASE_UPDATED))?;
            let data: AppStoreReleaseUpdatedData = serde_json::from_value(raw)?;
            Ok(AppStoreConnectEvent::AppStoreReleaseUpdated(data))
        }
        EXTERNAL_TESTFLIGHT_RELEASE_UPDATED => Ok(AppStoreConnectEvent::TestFlightExternalUpdated),
        INTERNAL_TESTFLIGHT_RELEASE_CREATED => Ok(AppStoreConnectEvent::TestFlightInternalCreated),
        ASSET_PACK_VERSION_UPDATED => Ok(AppStoreConnectEvent::AssetPackVersionUpdated),
        WEBHOOK_TEST | WEBHOOK_PING => Ok(AppStoreConnectEvent::WebhookPing),
        other => Ok(AppStoreConnectEvent::Unknown(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_app_store_release_updated() {
        let body = r#"{
            "notificationType": "APP_STORE_RELEASE_UPDATED",
            "notificationId": "abc",
            "data": {
                "platform": "darwin",
                "version": "2.1.0",
                "appStoreUrl": "https://apps.apple.com/app/bytover/id1234567890",
                "releaseNotes": "Bugs squashed"
            }
        }"#;
        let envelope: WebhookEnvelope = serde_json::from_str(body).unwrap();
        let evt = classify(&envelope).unwrap();
        match evt {
            AppStoreConnectEvent::AppStoreReleaseUpdated(d) => {
                assert_eq!(d.platform, "darwin");
                assert_eq!(d.version, "2.1.0");
                assert_eq!(
                    d.app_store_url.as_deref(),
                    Some("https://apps.apple.com/app/bytover/id1234567890"),
                );
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn classifies_testflight_and_asset_pack_variants() {
        let cases = [
            EXTERNAL_TESTFLIGHT_RELEASE_UPDATED,
            INTERNAL_TESTFLIGHT_RELEASE_CREATED,
            ASSET_PACK_VERSION_UPDATED,
        ];
        for nt in cases {
            let envelope = WebhookEnvelope {
                notification_type: nt.to_string(),
                notification_id: None,
                data: None,
            };
            let evt = classify(&envelope).unwrap();
            assert!(matches!(
                evt,
                AppStoreConnectEvent::TestFlightExternalUpdated
                    | AppStoreConnectEvent::TestFlightInternalCreated
                    | AppStoreConnectEvent::AssetPackVersionUpdated
            ));
        }
    }

    #[test]
    fn classifies_test_and_ping_as_webhook_ping() {
        for nt in [WEBHOOK_TEST, WEBHOOK_PING] {
            let envelope = WebhookEnvelope {
                notification_type: nt.to_string(),
                notification_id: None,
                data: None,
            };
            let evt = classify(&envelope).unwrap();
            assert!(matches!(evt, AppStoreConnectEvent::WebhookPing), "{nt} should classify as WebhookPing");
        }
    }

    #[test]
    fn unknown_event_classifies_as_unknown() {
        let envelope = WebhookEnvelope {
            notification_type: "SOMETHING_NEW".to_string(),
            notification_id: None,
            data: None,
        };
        let evt = classify(&envelope).unwrap();
        assert!(matches!(evt, AppStoreConnectEvent::Unknown(ref s) if s == "SOMETHING_NEW"));
    }

    #[test]
    fn app_store_release_missing_data_errors() {
        let envelope = WebhookEnvelope {
            notification_type: APP_STORE_RELEASE_UPDATED.to_string(),
            notification_id: None,
            data: None,
        };
        let err = classify(&envelope).unwrap_err();
        assert!(matches!(err, EventParseError::MissingData(_)));
    }
}
