use crux_core::typegen::TypeGen;
use shared::app::{modules::counter::{CounterEvent, CounterViewModel}, BitBridge};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_type::<CounterEvent>()?;
    gen.register_type::<CounterViewModel>()?;
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
