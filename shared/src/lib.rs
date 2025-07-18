// Only compile these modules when "lib" feature is enabled

static _CURRENT_VERSION: &str = "1.0.0";

pub mod app;
pub mod executor;
pub mod core_api;
pub mod core_transfer_protocol;
pub mod entities;
pub mod errors;
pub mod rpc;
pub mod utils;

pub use app::CoreOperation;
