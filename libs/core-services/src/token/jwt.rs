use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims<T> {
    exp: usize,
    iat: usize,
    data: T
}

#[derive(Debug, Error)]
pub enum JwtErrors {
    #[error("JWT coding error: {0}")]
    CodingError(#[from] jsonwebtoken::errors::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Invalid secret")]
    InvalidSecret
}

pub fn create_jwt_token<T: Clone>(data: T, secret: &str, expiration: Duration) -> Result<String, JwtErrors>
where
    T: Serialize
{
    if secret.is_empty() {
        return Err(JwtErrors::InvalidSecret);
    }

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(secret.as_ref());

    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        exp: (Utc::now() + expiration).timestamp() as usize,
        iat: now,
        data
    };

    let token = encode(&header, &claims, &encoding_key)?;
    Ok(token)
}

pub fn decode_jwt_token<T: Clone>(token: &str, secret: &str) -> Result<T, JwtErrors>
where
    T: for<'de> Deserialize<'de>
{
    if secret.is_empty() {
        return Err(JwtErrors::InvalidSecret);
    }

    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<Claims<T>>(token, &decoding_key, &validation)?;
    Ok(token_data.claims.data)
}

pub fn decode_jwt_token_without_validation<T: Clone>(token: &str) -> Result<T, JwtErrors>
where
    T: for<'de> Deserialize<'de>
{
    let mut validation = Validation::new(Algorithm::HS256);
    #[allow(deprecated)]
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;

    let decoding_key = DecodingKey::from_secret(&[]);

    let token_data = decode::<Claims<T>>(token, &decoding_key, &validation)?;
    Ok(token_data.claims.data)
}
