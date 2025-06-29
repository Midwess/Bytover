// Only compile these modules when "lib" feature is enabled

extern crate core;

use uniffi::setup_scaffolding;

static _CURRENT_VERSION: &str = "1.0.0";

pub mod app;
pub mod entities;
pub mod errors;
pub mod grpc;
pub mod core_api;
mod core_transfer_protocol;

pub use app::CoreOperation as CoreOperation;

setup_scaffolding!("shared");
