use std::sync::Arc;

use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::web::Bytes;
use devlog_sdk::tcp::listener::find_tcp_listener;
use prost::Message as ProstMessage;

use crate::client::Client;
use crate::client_manager::ClientManager;
use crate::turn_manager::TurnManager;

pub struct SignallingServer {
    client_manager: Arc<ClientManager>,
    turn_manager: Arc<TurnManager>,
}

impl SignallingServer {
    pub fn new(
        turn_manager: Arc<TurnManager>,
    ) -> Self {
        Self {
            client_manager: ClientManager::new(),
            turn_manager,
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connection = find_tcp_listener(Some(9221)).await?;
        let port = connection.port;
        let public_host = connection.public_host.clone();
        let std_listener = connection.listener.into_std()?;

        let turn_manager_for_register = Arc::clone(&self.turn_manager);
        let client_manager_for_closure1 = Arc::clone(&self.client_manager);
        let client_manager_for_closure2 = Arc::clone(&self.client_manager);
        let client_manager_for_closure3 = Arc::clone(&self.client_manager);
        let client_manager_for_closure4 = Arc::clone(&self.client_manager);
        let turn_manager_for_closure = Arc::clone(&self.turn_manager);

        let server = actix_web::HttpServer::new(move || {
            let turn_manager_route1 = Arc::clone(&turn_manager_for_closure);
            let turn_manager_route2 = Arc::clone(&turn_manager_for_closure);
            let turn_manager_route3 = Arc::clone(&turn_manager_for_closure);
            let turn_manager_route4 = Arc::clone(&turn_manager_for_closure);
            let turn_manager_route5 = Arc::clone(&turn_manager_for_closure);

            actix_web::App::new()
                .route(
                    "/server/{key}",
                    web::get().to({
                        let client_manager = Arc::clone(&client_manager_for_closure1);
                        let turn_manager = Arc::clone(&turn_manager_route1);
                        move |req: HttpRequest,
                              stream: web::Payload,
                              key: web::Path<String>| {
                            ws_handler(
                                req,
                                stream,
                                key.into_inner(),
                                client_manager.clone(),
                                turn_manager.clone(),
                            )
                        }
                    }),
                )
                .route(
                    "/offer/{key}",
                    web::post().to({
                        let client_manager = Arc::clone(&client_manager_for_closure2);
                        let turn_manager = Arc::clone(&turn_manager_route2);
                        move |key: web::Path<String>, body: Bytes| {
                            offer_handler(key.into_inner(), body, client_manager.clone(), turn_manager.clone())
                        }
                    }),
                )
                .route(
                    "/relay/{key}",
                    web::get().to({
                        let client_manager = Arc::clone(&client_manager_for_closure3);
                        let turn_manager = Arc::clone(&turn_manager_route3);
                        move |key: web::Path<String>| {
                            relay_handler(key.into_inner(), client_manager.clone(), turn_manager.clone())
                        }
                    }),
                )
                .route(
                    "/relay/{key}",
                    web::post().to({
                        let client_manager = Arc::clone(&client_manager_for_closure4);
                        let turn_manager = Arc::clone(&turn_manager_route4);
                        move |key: web::Path<String>, body: Bytes| {
                            relay_proxy_handler(key.into_inner(), body, client_manager.clone(), turn_manager.clone())
                        }
                    }),
                )
                .route(
                    "/register-relay",
                    web::post().to({
                        let turn_manager = Arc::clone(&turn_manager_route5);
                        move |req: HttpRequest, body: web::Json<RegisterRelayRequest>| {
                            register_relay_handler(req, body, turn_manager.clone())
                        }
                    }),
                )
        })
        .listen(std_listener)?
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

        let api_gateway =
            KongGatewayAdminClient::new(devlog_sdk::config::CONFIGS.kong.admin_url.clone());

        let service = GatewayServiceBuilder::new()
            .http(public_host.to_string(), port)
            .enable_cors(true)
            .name("rpc-signalling-server")
            .routes(vec![
                GatewayRouteBuilder::new()
                    .path(GatewayRouteExpression::start_with("/rpc-signalling"))
                    .http()
                    .methods(vec!["POST".to_owned(), "GET".to_owned(), "OPTIONS".to_owned()])
                    .strip_path(true)
                    .public(true)
                    .preserve_host(true)
                    .priority(10)
                    .name("devlog-rpc-signalling-server-ws")
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
) -> Result<HttpResponse, actix_web::Error> {
    let peer_addr = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("0.0.0.0")
        .to_string();

    let _ip_address = extract_ip_from_request(&req, &peer_addr);

    let (response, session, msg_stream) = actix_ws::handle(&req, stream)?;

    let client = Client::new(key.clone(), session);
    let client = Arc::new(client);

    client_manager.register(key.clone(), &client).await;

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

    let offer_request = match schema::devlog::rpc_signalling::server::OfferRequest::decode(&body[..]) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest()
                .body(format!("failed to decode offer request: {}", e));
        }
    };

    let mut message = schema::devlog::rpc_signalling::server::Message {
        offer: Some(schema::devlog::rpc_signalling::server::OfferMessage {
            sdp: offer_request.offer.sdp,
            peer: Some(offer_request.peer),
        }),
        session_id: offer_request.session_id,
        ..Default::default()
    };

    message.ice_config = turn_manager.get_relay_config(&key).await;

    match client.request(message).await {
        Ok(response) => {
            let answer = match response.answer {
                Some(a) => a,
                None => return HttpResponse::InternalServerError().body("no answer in response"),
            };
            let peer = match answer.peer.clone() {
                Some(p) => p,
                None => return HttpResponse::InternalServerError().body("no peer info in response"),
            };

            let offer_response = schema::devlog::rpc_signalling::server::OfferResponse {
                answer: schema::devlog::rpc_signalling::server::AnswerMessage {
                    sdp: answer.sdp,
                    peer: Some(peer.clone()),
                },
                peer,
            };

            let mut buf = Vec::new();
            match offer_response.encode(&mut buf) {
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

async fn relay_proxy_handler(
    key: String,
    body: Bytes,
    client_manager: Arc<ClientManager>,
    turn_manager: Arc<TurnManager>,
) -> HttpResponse {
    use schema::devlog::bitbridge::relay_service_client::RelayServiceClient;
    use base64::Engine;
    use tonic::metadata::MetadataValue;

    if client_manager.get(&key).await.is_none() {
        return HttpResponse::ServiceUnavailable().body("client not connected");
    }

    let connect_req = match schema::devlog::bitbridge::ConnectRequest::decode(&body[..]) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("failed to decode ConnectRequest: {}", e));
        }
    };

    let ice_config = match turn_manager.get_relay_config(&key).await {
        Some(c) => c,
        None => return HttpResponse::ServiceUnavailable().body("no relay config assigned to client"),
    };

    let stun_url = match ice_config.urls.iter().find(|u| u.starts_with("stun:")) {
        Some(u) => u,
        None => return HttpResponse::InternalServerError().body("no stun URL found in client config"),
    };

    // Parse domain from stun:domain:3478
    let domain = stun_url
        .strip_prefix("stun:")
        .and_then(|s: &str| s.split(':').next())
        .unwrap_or("");

    if domain.is_empty() {
        return HttpResponse::InternalServerError().body("invalid STUN URL format");
    }

    let url = format!("http://{}:9101", domain);
    
    let channel = match tonic::transport::Channel::from_shared(url) {
        Ok(endpoint) => match endpoint.connect().await {
            Ok(ch) => ch,
            Err(e) => return HttpResponse::InternalServerError().body(format!("failed to connect to relay server channel: {}", e))
        },
        Err(e) => return HttpResponse::InternalServerError().body(format!("invalid relay server url: {}", e))
    };
    let mut client = RelayServiceClient::new(channel);

    let secret = std::env::var("RELAY_SERVER_SECRET").unwrap_or_else(|_| "supersecret".to_string());
    let auth_str = format!("user:{}", secret);
    let b64_auth = base64::engine::general_purpose::STANDARD.encode(auth_str);
    let header_value = format!("Basic {}", b64_auth);

    let mut request = tonic::Request::new(connect_req);
    if let Ok(meta_value) = MetadataValue::try_from(header_value) {
        request.metadata_mut().insert("authorization", meta_value);
    }

    match client.connect(request).await {
        Ok(response) => {
            let mut buf = Vec::new();
            let msg = response.into_inner();
            if let Err(e) = ProstMessage::encode(&msg, &mut buf) {
                return HttpResponse::InternalServerError().body(format!("failed to encode response: {}", e));
            }
            HttpResponse::Ok()
                .content_type("application/octet-stream")
                .body(buf)
        }
        Err(status) => {
            HttpResponse::InternalServerError().body(format!("relay server gRPC error: {}", status))
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

    let mut ip = peer_addr.to_string();
    if ip == "::1" || ip == "localhost" {
        ip = "127.0.0.1".to_string();
    }
    ip
}

#[derive(serde::Deserialize)]
struct RegisterRelayRequest {
    stun_port: u16,
    relay_port: u16,
}

async fn register_relay_handler(
    req: HttpRequest,
    body: web::Json<RegisterRelayRequest>,
    turn_manager: Arc<TurnManager>,
) -> HttpResponse {
    use base64::Engine;

    // Basic Auth Check
    let auth_header = match req.headers().get("authorization").and_then(|h| h.to_str().ok()) {
        Some(h) => h,
        None => return HttpResponse::Unauthorized().body("missing authorization header"),
    };

    if !auth_header.starts_with("Basic ") {
        return HttpResponse::Unauthorized().body("invalid authorization format");
    }

    let b64_part = &auth_header[6..];
    let decoded = match base64::engine::general_purpose::STANDARD.decode(b64_part) {
        Ok(d) => d,
        Err(_) => return HttpResponse::Unauthorized().body("failed to decode base64 auth"),
    };

    let auth_str = String::from_utf8_lossy(&decoded);
    let mut parts = auth_str.split(':');
    let _user = parts.next().unwrap_or("");
    let secret = parts.next().unwrap_or("");

    let expected_secret = std::env::var("RELAY_SERVER_SECRET").unwrap_or_else(|_| "supersecret".to_string());
    if secret != expected_secret {
        return HttpResponse::Forbidden().body("invalid secret");
    }

    let peer_addr = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("0.0.0.0")
        .to_string();
    
    let ip_address = extract_ip_from_request(&req, &peer_addr);

    turn_manager.register_relay(ip_address, body.stun_port, body.relay_port).await;

    HttpResponse::Ok().finish()
}
