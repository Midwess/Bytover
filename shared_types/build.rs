use crux_core::typegen::TypeGen;
use schema::value::platform::Platform;
use shared::app::modules::authentication::{AuthenticationEvent, AuthenticationModel};
use shared::app::modules::environment::{EnvironmentEvent, EnvironmentModel};
use shared::app::operations::database::{
    DatabaseOperation,
    DatabaseOperationOutput,
    SessionOperation,
    SessionOperationOutput
};
use shared::app::operations::rpc::{RpcOperation, RpcOperationOutput};
use shared::app::BitBridge;
use shared::entities::session::{Session, SessionType};
use shared::entities::token::Token;
use shared::entities::user::User;
use shared::errors::NetworkError;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    // Register base types
    gen.register_type::<Token>()?;
    gen.register_type::<NetworkError>()?;
    gen.register_type::<Session>()?;
    gen.register_type::<SessionType>()?;
    gen.register_type::<User>()?;
    gen.register_type::<Platform>()?;

    // Register operation enums
    gen.register_type::<DatabaseOperation>()?;
    gen.register_type::<DatabaseOperationOutput>()?;
    gen.register_type::<RpcOperation>()?;
    gen.register_type::<RpcOperationOutput>()?;
    gen.register_type::<SessionOperation>()?;
    gen.register_type::<SessionOperationOutput>()?;

    // Register module types
    gen.register_type::<EnvironmentEvent>()?;
    gen.register_type::<EnvironmentModel>()?;
    gen.register_type::<AuthenticationEvent>()?;
    gen.register_type::<AuthenticationModel>()?;

    gen.register_app::<BitBridge>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;
    gen.java("com.devlog.bitbridge.shared_types", output_root.join("java"))?;
    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
