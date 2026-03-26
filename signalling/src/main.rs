mod client;
mod client_manager;
mod locator;
mod server;
mod turn_manager;
mod turn_server_registry;

use std::sync::Arc;

use crate::locator::LocatorServer;
use crate::server::SignallingServer;
use crate::turn_manager::TurnManager;
use core_services::logger;
use devlog_sdk::distributed_id::init_scoped_id_generator;

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
    let signalling_server = SignallingServer::new(Arc::clone(&turn_manager));
    let locator_server = LocatorServer::new();

    tokio::select! {
        result = signalling_server.run() => {
            log::info!("Signalling server stopped {:?}", result);
        }
        result = locator_server.run() => {
            log::info!("Locator server stopped: {:?}", result);
        }
        _ = turn_registry.run() => {
            log::info!("TURN registry stopped");
        }
    }
}
