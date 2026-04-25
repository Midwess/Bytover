pub mod app_store_connect;
pub mod asc_api;
pub mod events;
pub mod ingestor;
pub mod verify;

use axum::routing::post;
use axum::Router;

pub fn router() -> Router {
    Router::new().route("/webhooks/app-store-connect", post(app_store_connect::handle))
}
