use crux_core::typegen::TypeGen;
use schema::value::platform::Platform;
use shared::app::{modules::{environment::{EnvironmentEvent, EnvironmentModel}, authentication::{AuthenticationEvent, AuthenticationModel}}, BitBridge};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_type::<Platform>()?;
    gen.register_type::<EnvironmentEvent>()?;
    gen.register_type::<EnvironmentModel>()?;
    gen.register_type::<AuthenticationEvent>()?;
    gen.register_type::<AuthenticationModel>()?;
    gen.register_app::<BitBridge>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;

    gen.java(
        "com.devlog.bitbridge.shared_types",
        output_root.join("java"),
    )?;

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
