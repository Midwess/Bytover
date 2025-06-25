// Only compile these modules when "lib" feature is enabled

use uniffi::setup_scaffolding;

static _CURRENT_VERSION: &str = "1.0.0";

pub mod app;
pub mod entities;
pub mod errors;
pub mod persistence;
pub mod grpc;

setup_scaffolding!();
