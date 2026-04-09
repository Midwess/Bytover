extern crate core;

#[cfg(feature = "s3")]
pub type S3Connection = aws_sdk_s3::client::Client;

#[cfg(feature = "smtp")]
pub type SmtpTransport = lettre::SmtpTransport;

#[cfg(feature = "smtp")]
pub mod smtp;

#[cfg(feature = "host_machine")]
pub mod shell;

#[cfg(feature = "retry")]
pub mod retry;

#[cfg(feature = "s3")]
pub mod s3;

pub mod local_storage;

pub mod db;
pub mod logger;
pub mod services;
pub mod utils;

#[cfg(feature = "token")]
pub mod token;

#[cfg(feature = "wasm")]
pub mod wasm;
