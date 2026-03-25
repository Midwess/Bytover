use std::sync::Arc;

use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Bytes;
use prost::Message as ProstMessage;

use crate::client::Client;
use crate::client_manager::ClientManager;
use crate::turn_manager::TurnManager;

pub struct SignallingServer {
    client_manager: Arc<ClientManager>,
    turn_manager: Arc<TurnManager>,
    geoip_reader: Option<Arc<maxminddb::Reader<Vec<u8>>>>,
    port: u16,
    public_host: String,
}

impl SignallingServer {
    pub fn new(
        port: u16,
        public_host: String,
        turn_manager: Arc<TurnManager>,
    ) -> Self {
        let geoip_data = include_bytes!("../GeoLite2-Country.mmdb");
        let geoip_reader =
            maxminddb::Reader::from_source(geoip_data.to_vec()).ok().map(Arc::new);

        Self {
            client_manager: ClientManager::new(),
            turn_manager,
            geoip_reader,
            port,
            public_host,
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let geoip_reader = self.geoip_reader.clone();
        let port = self.port;
        let public_host = self.public_host.clone();
        let turn_manager_for_register = Arc::clone(&self.turn_manager);
        let client_manager_for_closure1 = Arc::clone(&self.client_manager);
        let client_manager_for_closure2 = Arc::clone(&self.client_manager);
        let client_manager_for_closure3 = Arc::clone(&self.client_manager);
        let turn_manager_for_closure = Arc::clone(&self.turn_manager);

        let server = actix_web::HttpServer::new(move || {
            let geoip_reader = geoip_reader.clone();

            actix_web::App::new()
                .route(
                    "/server/{key}",
                    web::get().to({
                        let geoip_reader = geoip_reader.clone();
                        let client_manager = Arc::clone(&client_manager_for_closure1);
                        let turn_manager = Arc::clone(&turn_manager_for_closure);
                        move |req: HttpRequest,
                              stream: web::Payload,
                              key: web::Path<String>| {
                            ws_handler(
                                req,
                                stream,
                                key.into_inner(),
                                client_manager.clone(),
                                turn_manager.clone(),
                                geoip_reader.clone(),
                            )
                        }
                    }),
                )
                .route(
                    "/offer/{key}",
                    web::post().to({
                        let client_manager = Arc::clone(&client_manager_for_closure2);
                        let turn_manager = Arc::clone(&turn_manager_for_closure);
                        move |key: web::Path<String>, body: Bytes| {
                            offer_handler(key.into_inner(), body, client_manager.clone(), turn_manager.clone())
                        }
                    }),
                )
                .route(
                    "/relay/{key}",
                    web::get().to({
                        let client_manager = Arc::clone(&client_manager_for_closure3);
                        let turn_manager = Arc::clone(&turn_manager_for_closure);
                        move |key: web::Path<String>| {
                            relay_handler(key.into_inner(), client_manager.clone(), turn_manager.clone())
                        }
                    }),
                )
        })
        .bind((
            std::net::IpAddr::from(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            port,
        ))?
        .run();

        log::info!(
            "RPC Signalling Server listening on {}:{}",
            public_host,
            port
        );

        self.register_gateway(&public_host, port, turn_manager_for_register).await?;

        server.await?;

        Ok(())
    }

    async fn register_gateway(
        &self,
        public_host: &str,
        port: u16,
        _turn_manager: Arc<TurnManager>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use devlog_sdk::api_gateway::client::ApiGatewayClient;
        use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
        use devlog_sdk::api_gateway::service::{
            GatewayRouteBuilder, GatewayRouteExpression, GatewayServiceBuilder,
        };
        use devlog_sdk::tcp::listener::find_tcp_listener;

        let _connection = find_tcp_listener(Some(port)).await?;

        let api_gateway =
            KongGatewayAdminClient::new(devlog_sdk::config::CONFIGS.kong.admin_url.clone());

        let service = GatewayServiceBuilder::new()
            .http(public_host.to_string(), port)
            .enable_cors(true)
            .name("rpc-signalling-server")
            .routes(vec![
                GatewayRouteBuilder::new()
                    .path(GatewayRouteExpression::start_with("/server"))
                    .http()
                    .methods(vec!["GET".to_owned(), "OPTIONS".to_owned()])
                    .strip_path(true)
                    .public(true)
                    .preserve_host(false)
                    .priority(10)
                    .name("devlog-rpc-signalling-server-ws")
                    .build(),
                GatewayRouteBuilder::new()
                    .path(GatewayRouteExpression::start_with("/offer"))
                    .http()
                    .methods(vec!["POST".to_owned(), "OPTIONS".to_owned()])
                    .strip_path(true)
                    .public(true)
                    .preserve_host(false)
                    .priority(10)
                    .name("devlog-rpc-signalling-server-http")
                    .build(),
                GatewayRouteBuilder::new()
                    .path(GatewayRouteExpression::start_with("/relay"))
                    .http()
                    .methods(vec!["GET".to_owned(), "OPTIONS".to_owned()])
                    .strip_path(true)
                    .public(true)
                    .preserve_host(false)
                    .priority(10)
                    .name("devlog-rpc-signalling-server-relay")
                    .build(),
            ])
            .build();

        api_gateway.register(service).await?;
        log::info!("Registered rpc-signalling service to gateway");

        Ok(())
    }
}

async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    key: String,
    client_manager: Arc<ClientManager>,
    turn_manager: Arc<TurnManager>,
    geoip_reader: Option<Arc<maxminddb::Reader<Vec<u8>>>>,
) -> Result<HttpResponse, actix_web::Error> {
    let peer_addr = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("0.0.0.0")
        .to_string();

    let ip_address = extract_ip_from_request(&req, &peer_addr);
    let continent =
        crate::turn_manager::detect_continent(&ip_address, geoip_reader.as_ref().map(|r| r.as_ref()));

    let (response, session, msg_stream) = actix_ws::handle(&req, stream)?;

    let client = Client::new(key.clone(), session, Arc::clone(&turn_manager));
    let client = Arc::new(client);

    client_manager.register(key.clone(), &client).await;
    turn_manager.register_client(key.clone(), continent).await;

    if let Err(e) = turn_manager.assign_relay_for_client(&key, continent).await {
        log::warn!("Failed to assign relay for client {}: {}", key, e);
    }

    let client_manager_clone = Arc::clone(&client_manager);
    let turn_manager_clone = Arc::clone(&turn_manager);
    let key_clone = key.clone();
    let client_for_spawn = Arc::clone(&client);

    tokio::task::spawn_local(async move {
        <Arc<Client> as Clone>::clone(&client_for_spawn).run(msg_stream).await;
        client_manager_clone.unregister(&key_clone).await;
        turn_manager_clone.unregister_client(&key_clone).await;
    });

    Ok(response)
}

async fn offer_handler(
    key: String,
    body: Bytes,
    client_manager: Arc<ClientManager>,
    turn_manager: Arc<TurnManager>,
) -> HttpResponse {
    let client = match client_manager.get(&key).await {
        Some(c) => c,
        None => {
            return HttpResponse::ServiceUnavailable()
                .body("client not connected");
        }
    };

    let mut message = match schema::devlog::rpc_signalling::server::Message::decode(&body[..]) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest()
                .body(format!("failed to decode message: {}", e));
        }
    };

    message.ice_config = turn_manager.get_relay_config(&key).await;

    match client.request(message).await {
        Ok(response) => {
            let mut buf = Vec::new();
            match response.encode(&mut buf) {
                Ok(()) => HttpResponse::Ok()
                    .content_type("application/octet-stream")
                    .body(buf),
                Err(e) => {
                    HttpResponse::InternalServerError()
                        .body(format!("failed to encode response: {}", e))
                }
            }
        }
        Err(crate::client::ClientError::Timeout(_)) => {
            HttpResponse::GatewayTimeout().body("request timed out")
        }
        Err(crate::client::ClientError::Disconnected) => {
            HttpResponse::ServiceUnavailable().body("client disconnected")
        }
        Err(e) => {
            HttpResponse::InternalServerError()
                .body(format!("internal error: {}", e))
        }
    }
}

async fn relay_handler(
    key: String,
    client_manager: Arc<ClientManager>,
    turn_manager: Arc<TurnManager>,
) -> HttpResponse {
    let _ = client_manager.get(&key).await;

    let relay_config = match turn_manager.get_relay_config(&key).await {
        Some(config) => config,
        None => {
            return HttpResponse::ServiceUnavailable()
                .body("client not connected");
        }
    };

    let mut buf = Vec::new();
    match relay_config.encode(&mut buf) {
        Ok(()) => HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(buf),
        Err(e) => {
            HttpResponse::InternalServerError()
                .body(format!("failed to encode response: {}", e))
        }
    }
}

fn extract_ip_from_request(req: &HttpRequest, peer_addr: &str) -> String {
    if let Some(cf_ip) = req.headers().get("CF-Connecting-IP") {
        if let Ok(ip) = cf_ip.to_str() {
            return ip.split(',').next().unwrap_or(ip).trim().to_string();
        }
    }

    if let Some(xff) = req.headers().get("X-Forwarded-For") {
        if let Ok(list) = xff.to_str() {
            return list.split(',').next().unwrap_or(list).trim().to_string();
        }
    }

    if let Some(xri) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = xri.to_str() {
            return ip.to_string();
        }
    }

    peer_addr.to_string()
}
