use actix_web::http::header::HeaderMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

pub const SIGNATURE_HEADER: &str = "X-Apple-Store-Notification-Signature";
pub const TIMESTAMP_HEADER: &str = "X-Apple-Store-Notification-Timestamp";

const SIGNATURE_PREFIX: &str = "sha256=";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VerifyError {
    #[error("missing signature header")]
    MissingSignature,
    #[error("missing timestamp header")]
    MissingTimestamp,
    #[error("malformed signature header")]
    MalformedSignature,
    #[error("malformed timestamp header")]
    MalformedTimestamp,
    #[error("signature mismatch")]
    SignatureMismatch,
    #[error("timestamp outside allowed skew")]
    StaleTimestamp,
}

pub struct WebhookSecretVerifier {
    secret: Vec<u8>,
    max_skew: Duration,
}

impl WebhookSecretVerifier {
    pub fn new(secret: impl Into<Vec<u8>>, max_skew: Duration) -> Self {
        Self {
            secret: secret.into(),
            max_skew,
        }
    }

    pub fn verify(&self, headers: &HeaderMap, raw_body: &[u8], now: SystemTime) -> Result<(), VerifyError> {
        let signature = headers
            .get(SIGNATURE_HEADER)
            .ok_or(VerifyError::MissingSignature)?
            .to_str()
            .map_err(|_| VerifyError::MalformedSignature)?;

        let timestamp = headers
            .get(TIMESTAMP_HEADER)
            .ok_or(VerifyError::MissingTimestamp)?
            .to_str()
            .map_err(|_| VerifyError::MalformedTimestamp)?;

        let ts_secs: u64 = timestamp.trim().parse().map_err(|_| VerifyError::MalformedTimestamp)?;

        let now_secs = now.duration_since(UNIX_EPOCH).map_err(|_| VerifyError::StaleTimestamp)?.as_secs();

        let delta = now_secs.saturating_sub(ts_secs).max(ts_secs.saturating_sub(now_secs));
        if delta > self.max_skew.as_secs() {
            return Err(VerifyError::StaleTimestamp);
        }

        let signature_hex = signature
            .strip_prefix(SIGNATURE_PREFIX)
            .ok_or(VerifyError::MalformedSignature)?
            .trim();

        let provided = hex::decode(signature_hex).map_err(|_| VerifyError::MalformedSignature)?;

        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.secret).map_err(|_| VerifyError::MalformedSignature)?;
        mac.update(timestamp.as_bytes());
        mac.update(b".");
        mac.update(raw_body);
        let expected = mac.finalize().into_bytes();

        if expected.ct_eq(&provided).unwrap_u8() == 1 {
            Ok(())
        } else {
            Err(VerifyError::SignatureMismatch)
        }
    }
}

pub fn sign(secret: &[u8], timestamp_secs: u64, raw_body: &[u8]) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret).expect("hmac key size");
    let ts = timestamp_secs.to_string();
    mac.update(ts.as_bytes());
    mac.update(b".");
    mac.update(raw_body);
    format!("{}{}", SIGNATURE_PREFIX, hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::header::{HeaderName, HeaderValue};

    fn build_headers(sig: &str, ts: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(HeaderName::from_static("x-apple-store-notification-signature"), HeaderValue::from_str(sig).unwrap());
        h.insert(HeaderName::from_static("x-apple-store-notification-timestamp"), HeaderValue::from_str(ts).unwrap());
        h
    }

    fn now_at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn accepts_valid_signature_and_fresh_timestamp() {
        let verifier = WebhookSecretVerifier::new(b"topsecret".to_vec(), Duration::from_secs(300));
        let body = br#"{"notificationType":"APP_STORE_RELEASE_UPDATED"}"#;
        let ts = 1_750_000_000u64;
        let sig = sign(b"topsecret", ts, body);
        let headers = build_headers(&sig, &ts.to_string());
        verifier.verify(&headers, body, now_at(ts + 5)).unwrap();
    }

    #[test]
    fn rejects_missing_signature() {
        let verifier = WebhookSecretVerifier::new(b"s".to_vec(), Duration::from_secs(300));
        let mut h = HeaderMap::new();
        h.insert(HeaderName::from_static("x-apple-store-notification-timestamp"), HeaderValue::from_static("1"));
        assert_eq!(verifier.verify(&h, b"body", now_at(1)).unwrap_err(), VerifyError::MissingSignature);
    }

    #[test]
    fn rejects_missing_timestamp() {
        let verifier = WebhookSecretVerifier::new(b"s".to_vec(), Duration::from_secs(300));
        let mut h = HeaderMap::new();
        h.insert(HeaderName::from_static("x-apple-store-notification-signature"), HeaderValue::from_static("sha256=abc"));
        assert_eq!(verifier.verify(&h, b"body", now_at(1)).unwrap_err(), VerifyError::MissingTimestamp);
    }

    #[test]
    fn rejects_malformed_signature_prefix() {
        let verifier = WebhookSecretVerifier::new(b"s".to_vec(), Duration::from_secs(300));
        let headers = build_headers("md5=abcd", "10");
        assert_eq!(verifier.verify(&headers, b"body", now_at(10)).unwrap_err(), VerifyError::MalformedSignature);
    }

    #[test]
    fn rejects_tampered_body() {
        let verifier = WebhookSecretVerifier::new(b"topsecret".to_vec(), Duration::from_secs(300));
        let ts = 1_750_000_000u64;
        let sig = sign(b"topsecret", ts, b"original");
        let headers = build_headers(&sig, &ts.to_string());
        assert_eq!(
            verifier.verify(&headers, b"tampered", now_at(ts)).unwrap_err(),
            VerifyError::SignatureMismatch,
        );
    }

    #[test]
    fn rejects_stale_timestamp() {
        let verifier = WebhookSecretVerifier::new(b"topsecret".to_vec(), Duration::from_secs(60));
        let ts = 1_750_000_000u64;
        let body = b"body";
        let sig = sign(b"topsecret", ts, body);
        let headers = build_headers(&sig, &ts.to_string());
        let err = verifier.verify(&headers, body, now_at(ts + 3600)).unwrap_err();
        assert_eq!(err, VerifyError::StaleTimestamp);
    }

    #[test]
    fn rejects_future_timestamp_beyond_skew() {
        let verifier = WebhookSecretVerifier::new(b"topsecret".to_vec(), Duration::from_secs(60));
        let ts = 1_750_000_000u64;
        let body = b"body";
        let sig = sign(b"topsecret", ts, body);
        let headers = build_headers(&sig, &ts.to_string());
        let err = verifier.verify(&headers, body, now_at(ts - 3600)).unwrap_err();
        assert_eq!(err, VerifyError::StaleTimestamp);
    }

    #[test]
    fn rejects_wrong_secret() {
        let verifier = WebhookSecretVerifier::new(b"s1".to_vec(), Duration::from_secs(300));
        let ts = 1_750_000_000u64;
        let body = b"body";
        let sig = sign(b"s2", ts, body);
        let headers = build_headers(&sig, &ts.to_string());
        assert_eq!(
            verifier.verify(&headers, body, now_at(ts)).unwrap_err(),
            VerifyError::SignatureMismatch,
        );
    }
}
