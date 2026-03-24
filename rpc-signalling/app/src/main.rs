mod locator;
mod scope_key;
mod signaller;
mod websocket;
mod turn_manager;
mod turn_server_registry;

use crate::locator::LocatorServer;
use crate::websocket::SignallingServer;
use crate::turn_manager::TurnManager;
use core_services::logger;
use devlog_sdk::distributed_id::init_scoped_id_generator;
use std::sync::Arc;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    logger::setup();
    init_scoped_id_generator("rpc-signalling".to_string());
    let turn_manager = match TurnManager::new().await {
        Ok(manager) => manager,
        Err(e) => {
            log::error!("Failed to initialize TurnManager: {}", e);
            return;
        }
    };
    let turn_manager = std::sync::Arc::new(turn_manager);
    let turn_registry = turn_manager.get_registry();

    let scope_task = signaller::Signaller::new();
    let scope_request_tx = scope_task.request_tx();
    let signalling_server = SignallingServer::new(scope_request_tx, Arc::clone(&turn_manager));
    let locator_server = LocatorServer::new();

    tokio::select! {
        result = signalling_server.run() => {
            log::info!("Signalling server stopped {:?}", result);
        }
        result = locator_server.run() => {
            log::info!("Locator server stopped: {:?}", result);
        }
        result = scope_task.run() => {
            log::info!("Scope task stopped: {:?}", result);
        }
        _ = turn_registry.run() => {
            log::info!("TURN registry stopped");
        }
    }
}
