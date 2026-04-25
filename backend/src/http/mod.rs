pub mod update;
pub mod webhooks;

use axum::routing::get;
use axum::Router;

pub fn router() -> Router {
    let api_v1 = Router::new()
        .route(
            "/update/{target}/{arch}/{current_version}",
            get(update::get_update_manifest),
        )
        .merge(webhooks::router());

    Router::new().nest("/bitbridge/api/v1", api_v1)
}
