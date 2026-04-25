use actix_web::http::header::HeaderMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

pub const SIGNATURE_HEADER: &str = "X-Apple-Signature";

const SIGNATURE_PREFIX: &str = "hmacsha256=";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VerifyError {
    #[error("missing signature header")]
    MissingSignature,
    #[error("malformed signature header")]
    MalformedSignature,
    #[error("signature mismatch")]
    SignatureMismatch,
}

pub struct WebhookSecretVerifier {
    secret: Vec<u8>,
}

impl WebhookSecretVerifier {
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self { secret: secret.into() }
    }

    pub fn verify(&self, headers: &HeaderMap, raw_body: &[u8]) -> Result<(), VerifyError> {
        let signature = headers
            .get(SIGNATURE_HEADER)
            .ok_or(VerifyError::MissingSignature)?
            .to_str()
            .map_err(|_| VerifyError::MalformedSignature)?;

        let signature_hex = signature
            .strip_prefix(SIGNATURE_PREFIX)
            .ok_or(VerifyError::MalformedSignature)?
            .trim();

        let provided = hex::decode(signature_hex).map_err(|_| VerifyError::MalformedSignature)?;

        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.secret).map_err(|_| VerifyError::MalformedSignature)?;
        mac.update(raw_body);
        let expected = mac.finalize().into_bytes();

        if expected.ct_eq(&provided).unwrap_u8() == 1 {
            Ok(())
        } else {
            Err(VerifyError::SignatureMismatch)
        }
    }
}

pub fn sign(secret: &[u8], raw_body: &[u8]) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(secret).expect("hmac key size");
    mac.update(raw_body);
    format!("{}{}", SIGNATURE_PREFIX, hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::header::{HeaderName, HeaderValue};

    fn build_headers(sig: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(HeaderName::from_static("x-apple-signature"), HeaderValue::from_str(sig).unwrap());
        h
    }

    #[test]
    fn accepts_valid_signature() {
        let verifier = WebhookSecretVerifier::new(b"topsecret".to_vec());
        let body = br#"{"notificationType":"APP_STORE_RELEASE_UPDATED"}"#;
        let sig = sign(b"topsecret", body);
        let headers = build_headers(&sig);
        verifier.verify(&headers, body).unwrap();
    }

    #[test]
    fn rejects_missing_signature() {
        let verifier = WebhookSecretVerifier::new(b"s".to_vec());
        let h = HeaderMap::new();
        assert_eq!(verifier.verify(&h, b"body").unwrap_err(), VerifyError::MissingSignature);
    }

    #[test]
    fn rejects_malformed_signature_prefix() {
        let verifier = WebhookSecretVerifier::new(b"s".to_vec());
        let headers = build_headers("md5=abcd");
        assert_eq!(verifier.verify(&headers, b"body").unwrap_err(), VerifyError::MalformedSignature);
    }

    #[test]
    fn rejects_tampered_body() {
        let verifier = WebhookSecretVerifier::new(b"topsecret".to_vec());
        let sig = sign(b"topsecret", b"original");
        let headers = build_headers(&sig);
        assert_eq!(
            verifier.verify(&headers, b"tampered").unwrap_err(),
            VerifyError::SignatureMismatch,
        );
    }

    #[test]
    fn rejects_wrong_secret() {
        let verifier = WebhookSecretVerifier::new(b"s1".to_vec());
        let body = b"body";
        let sig = sign(b"s2", body);
        let headers = build_headers(&sig);
        assert_eq!(
            verifier.verify(&headers, body).unwrap_err(),
            VerifyError::SignatureMismatch,
        );
    }
}
