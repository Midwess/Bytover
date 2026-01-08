// Only compile these modules when "lib" feature is enabled

static _CURRENT_VERSION: &str = "1.0.0";

pub mod app;
pub mod entities;
pub mod errors;
pub mod protocol;
pub mod repository;
pub mod shell;
pub mod utils;

pub use app::CoreOperation;

pub fn gen_shelf_id() -> u64 {
    devlog_sdk::distributed_id::gen_id_sync()
}
