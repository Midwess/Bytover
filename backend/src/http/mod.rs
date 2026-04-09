pub mod update;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/bitbridge/api/v1").service(update::get_update_manifest));
}
