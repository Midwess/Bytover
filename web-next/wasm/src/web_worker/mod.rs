use js_sys::Uint8Array;

pub mod codec;
pub mod core;
pub mod bridge;

pub type CoreOperationEncoded = Uint8Array;
pub type CoreOperationOutputEncoded = Uint8Array;
pub type AppEventEncoded = Uint8Array;
