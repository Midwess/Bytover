use std::path::PathBuf;

use crux_core::typegen::TypeGen;
use native::native::message_to_shell::{MessageToShell, MessageToShellResponse};
use native::repository::path_resolver::{PathResolverMessage, PathResolverResponseMessage};
use schema::devlog::bitbridge::peer_message_body::Response;
use schema::value::device::DeviceType;
use schema::value::platform::Platform;
use schema::value::static_resource::static_resource::Source;
use shared::app::file_system::file::{LocalResource, LocalResourcePath, ResourceType};
use shared::app::modules::authentication::{AuthenticationEvent, AuthenticationModel};
use shared::app::modules::environment::{EnvironmentEvent, EnvironmentModel};
use shared::app::modules::nearby::NearbyEvent;
use shared::app::modules::transfer::{TransferEvent, TransferModel};
use shared::app::nearby::finding_scope::FindingScope;
use shared::app::operations::device::{DeviceOperation, DeviceOperationOutput, GeoLocation, OpenOperation};
use shared::app::operations::dialog::{AlertDialog, DialogOperation, DialogOperationOutput};
use shared::app::operations::internet::{InternetOperation, InternetOperationOutput};
use shared::app::operations::p2p::{P2POperation, P2POperationOutput};
use shared::app::operations::persistent::{
    LocalResourcePersistentOperation,
    LocalResourcePersistentOperationOutput,
    PersistentOperation,
    PersistentOperationOutput,
    SessionPersistentOperation,
    SessionPersistentOperationOutput,
    TransferSessionOperationOutput,
    TransferSessionPersistentOperation
};
use shared::app::operations::rpc::{RpcOperation, RpcOperationOutput};
use shared::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use shared::app::operations::CoreOperationOutput;
use shared::app::transfer::file_selection_service::ResourceSelection;
use shared::app::transfer::session::{TransferSessionStatus, TransferStatus, TransferType};
use shared::app::transfer::target::TransferTarget;
use shared::app::transfer::transfer_selection::TransferMethodSelection;
use shared::app::BitBridge;
use shared::entities::session::{Session, SessionType};
use shared::entities::token::Token;
use shared::entities::user::User;
use shared::errors::NetworkError;

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
    gen.register_type::<ResourceType>()?;
    gen.register_type::<LocalResource>()?;
    gen.register_type::<LocalResourcePath>()?;
    gen.register_type::<ResourceSelection>()?;
    gen.register_type::<FindingScope>()?;
    gen.register_type::<GeoLocation>()?;
    gen.register_type::<AlertDialog>()?;

    // Register operation enums
    gen.register_type::<DialogOperation>()?;
    gen.register_type::<DialogOperationOutput>()?;
    gen.register_type::<PersistentOperation>()?;
    gen.register_type::<PersistentOperationOutput>()?;
    gen.register_type::<RpcOperation>()?;
    gen.register_type::<RpcOperationOutput>()?;
    gen.register_type::<SessionPersistentOperation>()?;
    gen.register_type::<SessionPersistentOperationOutput>()?;
    gen.register_type::<TransferOperation>()?;
    gen.register_type::<TransferOperationOutput>()?;
    gen.register_type::<LocalResourcePersistentOperation>()?;
    gen.register_type::<LocalResourcePersistentOperationOutput>()?;
    gen.register_type::<TransferSessionPersistentOperation>()?;
    gen.register_type::<TransferSessionOperationOutput>()?;
    gen.register_type::<InternetOperation>()?;
    gen.register_type::<InternetOperationOutput>()?;
    gen.register_type::<DeviceOperation>()?;
    gen.register_type::<DeviceOperationOutput>()?;
    gen.register_type::<P2POperation>()?;
    gen.register_type::<P2POperationOutput>()?;
    gen.register_type::<NearbyEvent>()?;
    gen.register_type::<OpenOperation>()?;
    gen.register_type::<PathResolverMessage>()?;
    gen.register_type::<PathResolverResponseMessage>()?;

    // Register module types
    gen.register_type::<EnvironmentEvent>()?;
    gen.register_type::<EnvironmentModel>()?;
    gen.register_type::<AuthenticationEvent>()?;
    gen.register_type::<AuthenticationModel>()?;
    gen.register_type::<TransferEvent>()?;
    gen.register_type::<TransferModel>()?;
    gen.register_type::<TransferMethodSelection>()?;
    gen.register_type::<DeviceType>()?;
    gen.register_type::<TransferStatus>()?;
    gen.register_type::<TransferType>()?;
    gen.register_type::<Response>()?;
    gen.register_type::<TransferSessionStatus>()?;

    gen.register_type::<CoreOperationOutput>()?;

    gen.register_type::<Source>()?;

    // Register executor msg
    gen.register_type::<MessageToShellResponse>()?;
    gen.register_type::<MessageToShell>()?;

    gen.register_type::<TransferTarget>()?;

    gen.register_app::<BitBridge>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;
    gen.java("com.devlog.bitbridge.shared_types", output_root.join("java"))?;
    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
