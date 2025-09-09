use js_sys::Uint8Array;

pub mod executor;
pub mod codec;
pub mod core;
pub mod main;

pub type CoreOperationEncoded = Uint8Array;
pub type CoreOperationOutputEncoded = Uint8Array;
pub type AppEventEncoded = Uint8Array;
