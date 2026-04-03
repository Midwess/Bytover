use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use devlog_sdk::tcp::listener::find_tcp_listener;

mod app_gateway;
mod di;
mod locator_client;
mod middleware;

use di::DiContainer;

pub struct RelayServer {
    #[allow(dead_code)]
    connections: Vec<String>,
}

impl Default for RelayServer {
    fn default() -> Self {
        Self {
            connections: Vec::new(),
        }
    }
}

impl RelayServer {
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let di = DiContainer::init().await;
        let public_ip = di.public_ip.clone();

        let connection = find_tcp_listener(Some(9101)).await?;
        let port = connection.port;
        let std_listener = connection.listener.into_std()?;

        let server = actix_web::HttpServer::new(move || {
            let client = di.app_gateway_client();
            actix_web::App::new()
                .app_data(web::Data::new(client))
                .wrap(crate::middleware::auth::Auth)
                .route(
                    "/connect",
                    web::post().to(connect_handler),
                )
        })
        .listen(std_listener)?
        .run();

        log::info!("Relay Server listening on {}:{}", public_ip, port);

        server.await?;

        Ok(())
    }
}

async fn connect_handler(
    req: HttpRequest,
) -> HttpResponse {
    let peer_addr = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("0.0.0.0")
        .to_string();

    let auth_context = match req.extensions().get::<app_gateway::client::AuthContext>() {
        Some(auth) => auth.clone(),
        None => {
            log::error!("AuthContext not found in extensions for {}", peer_addr);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }));
        }
    };

    log::info!("Connect request from user {} ({}) at {}",
        auth_context.user.user_name, auth_context.user.id.id, peer_addr);

    HttpResponse::Ok().json(serde_json::json!({
        "status": "connected",
        "user_id": auth_context.user.id.id
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    log::info!("Starting relay server...");

    RelayServer::default()
        .run()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
