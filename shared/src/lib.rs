// Only compile these modules when "lib" feature is enabled

static _CURRENT_VERSION: &str = "1.0.0";

pub mod app;
pub mod entities;
pub mod errors;
pub mod protocol;
pub mod repository;
pub mod shell;

pub use app::CoreOperation;
