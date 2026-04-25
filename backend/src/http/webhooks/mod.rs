pub mod app_store_connect;
pub mod asc_api;
pub mod events;
pub mod ingestor;
pub mod verify;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(app_store_connect::handle);
}
