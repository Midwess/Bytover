use super::bit_bridge::oauth_server::Oauth;
pub use super::bit_bridge::oauth_server::OauthServer;
use super::bit_bridge::{OauthGoogleRequest, OauthGoogleResponse};
use serde::Deserialize;
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct GoogleTokenInfo {
    email: String,
    email_verified: String,
    sub: String,
    name: Option<String>,
    picture: Option<String>,
    aud: String
}

#[derive(Debug)]
pub struct MyOauth {
    pub db: Pool<Postgres>
}

#[tonic::async_trait]
impl Oauth for MyOauth {
    async fn oauth_google(&self, request: Request<OauthGoogleRequest>) -> Result<Response<OauthGoogleResponse>, Status> {
        // TODO: cache Google's public key to verify locally
        let client = reqwest::Client::new();
        let response = client
            .get("https://oauth2.googleapis.com/tokeninfo")
            .query(&[("id_token", request.into_inner().id_token)])
            .send()
            .await
            .map_err(|e| Status::invalid_argument(format!("verify id token: {}", e)))?;

        if !response.status().is_success() {
            return Err(Status::invalid_argument("invalid id token"));
        }

        let token_info: GoogleTokenInfo =
            response.json().await.map_err(|e| Status::internal(format!("Failed to parse token info: {}", e)))?;

        if token_info.aud != std::env::var("DEVLOG_GOOGLE_CLIENT_ID").unwrap() {
            return Err(Status::invalid_argument("invalid id token"));
        }

        if token_info.email_verified != "true" {
            return Err(Status::invalid_argument("email not verified"));
        }

        if !token_info.email.ends_with("@gmail.com") {
            return Err(Status::unauthenticated("only Gmail accounts are allowed"));
        }

        let _user_id = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM \"user\" WHERE sub = $1")
            .bind(&token_info.sub)
            .fetch_optional(&self.db)
            .await
            .map_err(|e| Status::internal(format!("Database query error: {}", e)))?
        {
            Some(id) => id,
            None => {
                let new_user_id = Uuid::new_v4();
                sqlx::query("INSERT INTO \"user\" (id, name, email, sub, picture) VALUES ($1, $2, $3, $4, $5)")
                    .bind(new_user_id)
                    .bind(&token_info.name)
                    .bind(&token_info.email)
                    .bind(&token_info.sub)
                    .bind(&token_info.picture)
                    .execute(&self.db)
                    .await
                    .map_err(|e| Status::internal(format!("Failed to insert user: {}", e)))?;
                new_user_id
            }
        };

        // TODO: generate server's jwt
        let reply = OauthGoogleResponse {
            access_token: "test".to_string()
        };

        Ok(Response::new(reply))
    }
}
