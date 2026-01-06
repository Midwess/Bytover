use std::path::PathBuf;

use crux_core::typegen::TypeGen;
#[cfg(any(feature = "swift", feature = "java"))]
use native::native::message_to_shell::{MessageToShell, MessageToShellResponse};
#[cfg(any(feature = "swift", feature = "java"))]
use native::repository::path_resolver::{PathResolverMessage, PathResolverResponseMessage};
use schema::devlog::bitbridge::peer_message_body::Response;
use schema::devlog::bitbridge::{PeerErrorsMessage, ResourceNotificationRequest};
use schema::devlog::rpc_signalling::server::ScopeState;
use schema::value::device::DeviceType;
use schema::value::platform::Platform;
use schema::value::static_resource::static_resource::Source;
use schema::devlog::bitbridge::view_session_detail_response::Result as ViewSessionDetailResponseResult;
use shared::app::authentication::module::{AuthenticationEvent, AuthenticationModel};
use shared::app::environment::module::{EnvironmentEvent, EnvironmentModel};
use shared::app::p2p::module::P2PEvent;
use shared::app::operations::device::{DeviceOperation, GeoLocation};
use shared::app::operations::dialog::{AlertDialog, DialogOperation, MessageReason};
use shared::app::operations::internet::InternetOperation;
use shared::app::operations::p2p::{P2POperation, P2POperationOutput};
use shared::app::operations::persistent::{
    LocalResourcePersistentOperation,
    PersistentOperation,
    SessionPersistentOperation,
    TransferSessionPersistentOperation
};
use shared::app::operations::rpc::{RpcOperation, RpcOperationOutput};
use shared::app::operations::transfer::{TransferOperation, TransferOperationOutput};
use shared::app::operations::CoreOperationOutput;
use shared::app::shelf::module::{ResourceSelection, ShelfEvent, ShelfViewModel};
use shared::app::transfer::module::{TransferEvent, TransferModel};
use shared::app::BitBridge;
use shared::app::view_models::receive_session::ReceiveResourceViewModel;
use shared::entities::finding_scope::FindingScope;
use shared::entities::local_resource::{LocalResource, LocalResourcePath, ResourceType};
use shared::entities::session::{Session, SessionType};
use shared::entities::target::{P2PConnectionState, TransferTarget};
use shared::entities::token::Token;
use shared::entities::transfer_method::TransferMethodSelection;
use shared::entities::transfer_session::{TransferSession, TransferSessionStatus, TransferStatus, TransferType};
use shared::entities::user::User;
use shared::errors::CoreError;
use shared::repository::local_resource::LocalResourceId;
use shared::repository::transfer_session::{TransferSessionId, TransferTargetId};

fn main() {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    // Register module types
    gen.register_type::<EnvironmentEvent>().unwrap();
    gen.register_type::<EnvironmentModel>().unwrap();
    gen.register_type::<AuthenticationEvent>().unwrap();
    gen.register_type::<AuthenticationModel>().unwrap();
    gen.register_type::<TransferEvent>().unwrap();
    gen.register_type::<TransferSessionId>().unwrap();
    gen.register_type::<TransferTargetId>().unwrap();
    gen.register_type::<TransferTarget>().unwrap();
    gen.register_type::<TransferModel>().unwrap();
    gen.register_type::<TransferMethodSelection>().unwrap();
    gen.register_type::<DeviceType>().unwrap();
    gen.register_type::<TransferStatus>().unwrap();
    gen.register_type::<TransferType>().unwrap();
    gen.register_type::<Response>().unwrap();
    gen.register_type::<TransferSessionStatus>().unwrap();

    // Register base types
    gen.register_type::<Token>().unwrap();
    gen.register_type::<CoreError>().unwrap();
    gen.register_type::<Session>().unwrap();
    gen.register_type::<SessionType>().unwrap();
    gen.register_type::<User>().unwrap();
    gen.register_type::<Platform>().unwrap();
    gen.register_type::<ResourceType>().unwrap();
    gen.register_type::<LocalResource>().unwrap();
    gen.register_type::<LocalResourcePath>().unwrap();
    gen.register_type::<ResourceSelection>().unwrap();
    gen.register_type::<FindingScope>().unwrap();
    gen.register_type::<ScopeState>().unwrap();
    gen.register_type::<GeoLocation>().unwrap();
    gen.register_type::<AlertDialog>().unwrap();
    gen.register_type::<MessageReason>().unwrap();

    // Register operation enums
    gen.register_type::<DialogOperation>().unwrap();
    gen.register_type::<PersistentOperation>().unwrap();
    gen.register_type::<RpcOperation>().unwrap();
    gen.register_type::<RpcOperationOutput>().unwrap();
    gen.register_type::<SessionPersistentOperation>().unwrap();
    gen.register_type::<TransferOperation>().unwrap();
    gen.register_type::<TransferOperationOutput>().unwrap();
    gen.register_type::<LocalResourcePersistentOperation>().unwrap();
    gen.register_type::<TransferSessionPersistentOperation>().unwrap();
    gen.register_type::<TransferSession>().unwrap();
    gen.register_type::<InternetOperation>().unwrap();
    gen.register_type::<DeviceOperation>().unwrap();
    gen.register_type::<P2POperation>().unwrap();
    gen.register_type::<P2POperationOutput>().unwrap();
    gen.register_type::<P2PEvent>().unwrap();
    #[cfg(any(feature = "swift", feature = "java"))]
    gen.register_type::<PathResolverMessage>().unwrap();
    #[cfg(any(feature = "swift", feature = "java"))]
    gen.register_type::<PathResolverResponseMessage>().unwrap();

    gen.register_type::<CoreOperationOutput>().unwrap();

    gen.register_type::<Source>().unwrap();

    // Register executor msg
    #[cfg(any(feature = "swift", feature = "java"))]
    gen.register_type::<MessageToShellResponse>().unwrap();
    #[cfg(any(feature = "swift", feature = "java"))]
    gen.register_type::<MessageToShell>().unwrap();
    gen.register_type::<LocalResourceId>().unwrap();
    gen.register_type::<TransferSessionId>().unwrap();
    gen.register_type::<ShelfEvent>().unwrap();
    gen.register_type::<ShelfViewModel>().unwrap();
    gen.register_type::<ViewSessionDetailResponseResult>().unwrap();
    gen.register_type::<PeerErrorsMessage>().unwrap();
    gen.register_type::<P2POperationOutput>().unwrap();
    gen.register_type::<P2PConnectionState>().unwrap();
    gen.register_type::<ResourceNotificationRequest>().unwrap();

    gen.register_app::<BitBridge>().unwrap();
    gen.register_type::<ReceiveResourceViewModel>().unwrap();

    let output_root = PathBuf::from("./generated");

    #[cfg(feature = "swift")]
    gen.swift("SharedTypes", output_root.join("swift")).unwrap();

    #[cfg(feature = "java")]
    gen.java("com.devlog.bitbridge.shared_types", output_root.join("java")).unwrap();

    #[cfg(feature = "typescript")]
    gen.typescript("shared_types", output_root.join("typescript")).unwrap();
}
