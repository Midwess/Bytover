mod app_gateway;
mod client;
mod client_manager;
mod config;
mod server;
mod turn_manager;

use std::sync::Arc;

use crate::config::SignallingConfig;
use crate::server::SignallingServer;
use crate::turn_manager::TurnManager;
use core_services::logger;
use devlog_sdk::distributed_id::init_scoped_id_generator;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    logger::setup();
    let config = SignallingConfig::from_env();
    init_scoped_id_generator(config.signalling_route.clone());
    let turn_manager = TurnManager::new().await;
    let turn_manager = std::sync::Arc::new(turn_manager);
    let signalling_server = SignallingServer::new(config, Arc::clone(&turn_manager));

    match signalling_server.run().await {
        Ok(_) => log::info!("Signalling server stopped successfully"),
        Err(e) => log::error!("Signalling server stopped with error: {:?}", e),
    }
}
