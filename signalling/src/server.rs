use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use actix_web::web::Bytes;
use actix_web::{web, HttpRequest, HttpResponse};
use base64::Engine;
use devlog_sdk::tcp::listener::find_tcp_listener;
use prost::Message as ProstMessage;
use schema::devlog::rpc_signalling::server::{AnswerMessage, Message, OfferRequest, OfferResponse};

use crate::client::Client;
use crate::client_manager::ClientManager;
use crate::config::SignallingConfig;
use crate::turn_manager::TurnManager;

#[derive(Clone)]
struct ServerState {
    client_manager: Arc<ClientManager>,
    turn_manager: Arc<TurnManager>
}

pub struct SignallingServer {
    config: SignallingConfig,
    client_manager: Arc<ClientManager>,
    turn_manager: Arc<TurnManager>
}

impl SignallingServer {
    pub fn new(config: SignallingConfig, turn_manager: Arc<TurnManager>) -> Self {
        Self {
            config,
            client_manager: ClientManager::new(),
            turn_manager
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connection = find_tcp_listener(Some(9221)).await?;
        let port = connection.port;
        let public_host = connection.public_host.clone();
        let std_listener = connection.listener.into_std()?;

        let state = web::Data::new(ServerState {
            client_manager: Arc::clone(&self.client_manager),
            turn_manager: Arc::clone(&self.turn_manager)
        });

        let server = actix_web::HttpServer::new(move || {
            actix_web::App::new()
                .app_data(state.clone())
                .route("/server/{key}", web::get().to(ws_handler))
                .route("/offer/{key}", web::post().to(offer_handler))
                .route("/relay/{key}", web::get().to(relay_handler))
                .route("/relay/{key}", web::post().to(relay_proxy_handler))
                .route("/register-relay", web::post().to(register_relay_handler))
        })
        .listen(std_listener)?
        .run();

        log::info!(
            "RPC Signalling Server listening on {}:{} (region={}, route={})",
            public_host,
            port,
            self.config.region_code,
            self.config.signalling_route
        );

        self.register_gateway(&public_host, port).await?;

        server.await?;

        Ok(())
    }

    async fn register_gateway(&self, public_host: &str, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use devlog_sdk::api_gateway::client::ApiGatewayClient;
        use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
        use devlog_sdk::api_gateway::service::GatewayServiceBuilder;

        let api_gateway = KongGatewayAdminClient::new(devlog_sdk::config::CONFIGS.kong.admin_url.clone());

        let _ = api_gateway.delete_service("rpc-signalling-server").await;
        let _ = api_gateway.delete_service("rpc-signalling-shared-server").await;

        let pinned_service = GatewayServiceBuilder::new()
            .url(self.config.pinned_upstream_url(public_host, port))
            .enable_cors(true)
            .name(format!("rpc-signalling-{}-server", self.config.region_code))
            .routes(vec![build_gateway_route(
                format!("devlog-rpc-signalling-{}", self.config.region_code),
                &self.config.signalling_route,
                20
            )])
            .build();

        api_gateway.register(pinned_service).await?;

        log::info!("Registered signalling route {} to gateway", self.config.signalling_route);

        Ok(())
    }
}

fn build_gateway_route(name: impl Into<String>, route: &str, priority: u32) -> devlog_sdk::api_gateway::service::GatewayRoute {
    use devlog_sdk::api_gateway::service::{GatewayRouteBuilder, GatewayRouteExpression};

    GatewayRouteBuilder::new()
        .path(GatewayRouteExpression::exact_or_subpath(&format!("/{route}")))
        .http()
        .methods(vec![
            "POST".to_owned(),
            "GET".to_owned(),
            "OPTIONS".to_owned(),
        ])
        .strip_path(true)
        .public(true)
        .preserve_host(false)
        .priority(priority)
        .name(name)
        .build()
}

async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    key: web::Path<String>,
    state: web::Data<ServerState>
) -> Result<HttpResponse, actix_web::Error> {
    let key = key.into_inner();
    let peer_addr = req.connection_info().realip_remote_addr().unwrap_or("0.0.0.0").to_string();

    let _ip_address = extract_ip_from_request(&req, &peer_addr);

    let (response, session, msg_stream) = actix_ws::handle(&req, stream)?;

    let client = Arc::new(Client::new(key.clone(), session));

    state.client_manager.register(key.clone(), &client).await;

    let client_manager = Arc::clone(&state.client_manager);
    let turn_manager = Arc::clone(&state.turn_manager);
    let client_for_spawn = Arc::clone(&client);
    let key_clone = key.clone();

    tokio::task::spawn_local(async move {
        <Arc<Client> as Clone>::clone(&client_for_spawn).run(msg_stream).await;
        client_manager.unregister(&key_clone).await;
        turn_manager.unregister_client(&key_clone).await;
    });

    Ok(response)
}

async fn offer_handler(key: web::Path<String>, body: Bytes, state: web::Data<ServerState>) -> HttpResponse {
    let key = key.into_inner();
    let client = match state.client_manager.get(&key).await {
        Some(client) => client,
        None => return HttpResponse::ServiceUnavailable().body("client not connected")
    };

    let offer_request = match OfferRequest::decode(&body[..]) {
        Ok(message) => message,
        Err(error) => {
            return HttpResponse::BadRequest().body(format!("failed to decode offer request: {error}"));
        }
    };

    let mut message = Message {
        offer: Some(schema::devlog::rpc_signalling::server::OfferMessage {
            sdp: offer_request.offer.sdp,
            peer: offer_request.peer
        }),
        session_id: offer_request.session_id,
        ..Default::default()
    };

    message.ice_config = state.turn_manager.get_relay_config(&key).await;

    match client.request(message).await {
        Ok(response) => {
            let answer = match response.answer {
                Some(answer) => answer,
                None => return HttpResponse::InternalServerError().body("no answer in response")
            };

            let peer = match answer.peer.clone() {
                Some(peer) => peer,
                None => return HttpResponse::InternalServerError().body("no peer info in response")
            };

            let offer_response = OfferResponse {
                answer: AnswerMessage {
                    sdp: answer.sdp,
                    peer: Some(peer.clone())
                },
                peer
            };

            encode_binary_response(&offer_response)
        }
        Err(crate::client::ClientError::Timeout(_)) => HttpResponse::GatewayTimeout().body("request timed out"),
        Err(crate::client::ClientError::Disconnected) => HttpResponse::ServiceUnavailable().body("client disconnected"),
        Err(error) => HttpResponse::InternalServerError().body(format!("internal error: {error}"))
    }
}

async fn relay_handler(key: web::Path<String>, state: web::Data<ServerState>) -> HttpResponse {
    log::info!("Received requests");
    let key = key.into_inner();
    let _ = state.client_manager.get(&key).await;

    let relay_config = match state.turn_manager.get_relay_config(&key).await {
        Some(config) => config,
        None => return HttpResponse::ServiceUnavailable().body("client not connected")
    };

    log::info!("Resolved request");
    encode_binary_response(&relay_config)
}

async fn relay_proxy_handler(key: web::Path<String>, body: Bytes, state: web::Data<ServerState>) -> HttpResponse {
    use schema::devlog::bitbridge::relay_service_client::RelayServiceClient;
    use tonic::metadata::MetadataValue;

    let key = key.into_inner();

    if state.client_manager.get(&key).await.is_none() {
        return HttpResponse::ServiceUnavailable().body("client not connected");
    }

    let connect_req = match schema::devlog::bitbridge::ConnectRequest::decode(&body[..]) {
        Ok(message) => message,
        Err(error) => {
            return HttpResponse::BadRequest().body(format!("failed to decode ConnectRequest: {error}"));
        }
    };

    let relay = match state.turn_manager.get_assigned_relay(&key).await {
        Some(relay) => relay,
        None => return HttpResponse::ServiceUnavailable().body("no relay config assigned to client")
    };
    let relay_host = if relay.relay_host.contains(':') {
        format!("[{}]", relay.relay_host)
    } else {
        relay.relay_host
    };
    let url = format!("http://{relay_host}:{}", relay.relay_port);
    log::info!("Proxying relay request for client {} to {}", key, url);

    let channel = match tonic::transport::Channel::from_shared(url) {
        Ok(endpoint) => match endpoint.connect().await {
            Ok(channel) => channel,
            Err(error) => {
                return HttpResponse::InternalServerError().body(format!("failed to connect to relay server channel: {error}"));
            }
        },
        Err(error) => return HttpResponse::InternalServerError().body(format!("invalid relay server url: {error}"))
    };

    let mut client = RelayServiceClient::new(channel);

    let secret = std::env::var("RELAY_SERVER_SECRET").unwrap_or_else(|_| "supersecret".to_string());
    let header_value = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("user:{secret}"))
    );

    let mut request = tonic::Request::new(connect_req);
    if let Ok(meta_value) = MetadataValue::try_from(header_value) {
        request.metadata_mut().insert("authorization", meta_value);
    }

    match client.connect(request).await {
        Ok(response) => encode_binary_response(&response.into_inner()),
        Err(status) => HttpResponse::InternalServerError().body(format!("relay server gRPC error: {status}"))
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

fn parse_ipv4(value: &str) -> Option<Ipv4Addr> {
    let value = value.trim();

    if let Ok(ip) = value.parse::<Ipv4Addr>() {
        return Some(ip);
    }

    if let Ok(ip) = value.parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(ipv4) => Some(ipv4),
            IpAddr::V6(ipv6) => ipv6.to_ipv4()
        };
    }

    if let Ok(socket_addr) = value.parse::<std::net::SocketAddr>() {
        return match socket_addr.ip() {
            IpAddr::V4(ipv4) => Some(ipv4),
            IpAddr::V6(ipv6) => ipv6.to_ipv4()
        };
    }

    None
}

fn extract_public_ipv4_from_request(req: &HttpRequest, peer_addr: &str) -> Option<Ipv4Addr> {
    parse_ipv4(&extract_ip_from_request(req, peer_addr))
}

#[derive(serde::Deserialize)]
struct RegisterRelayRequest {
    stun_port: u16,
    relay_port: u16,
    relay_host: Option<String>,
    public_ip: Option<String>
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RegisterRelayResponse {
    ip_address: String
}

async fn register_relay_handler(
    req: HttpRequest,
    body: web::Json<RegisterRelayRequest>,
    state: web::Data<ServerState>
) -> HttpResponse {
    let auth_header = match req.headers().get("authorization").and_then(|header| header.to_str().ok()) {
        Some(header) => header,
        None => return HttpResponse::Unauthorized().body("missing authorization header")
    };

    if !auth_header.starts_with("Basic ") {
        return HttpResponse::Unauthorized().body("invalid authorization format");
    }

    let decoded = match base64::engine::general_purpose::STANDARD.decode(&auth_header[6..]) {
        Ok(decoded) => decoded,
        Err(_) => return HttpResponse::Unauthorized().body("failed to decode base64 auth")
    };

    let auth_str = String::from_utf8_lossy(&decoded);
    let mut parts = auth_str.split(':');
    let _user = parts.next().unwrap_or("");
    let secret = parts.next().unwrap_or("");

    let expected_secret = std::env::var("RELAY_SERVER_SECRET").unwrap_or_else(|_| "supersecret".to_string());
    if secret != expected_secret {
        return HttpResponse::Forbidden().body("invalid secret");
    }

    let peer_addr = req.connection_info().realip_remote_addr().unwrap_or("0.0.0.0").to_string();

    let ip_address = match body.public_ip.as_deref().and_then(parse_ipv4) {
        Some(ip) => ip,
        None => match extract_public_ipv4_from_request(&req, &peer_addr) {
            Some(ip) => ip,
            None => return HttpResponse::BadRequest().body("relay registration requires a public IPv4 address")
        }
    };

    let relay_host = match body.relay_host.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        Some(host) => host.to_string(),
        None => ip_address.to_string()
    };

    state
        .turn_manager
        .register_relay(ip_address.to_string(), relay_host, body.stun_port, body.relay_port)
        .await;

    HttpResponse::Ok().json(RegisterRelayResponse {
        ip_address: ip_address.to_string()
    })
}

fn encode_binary_response<T: ProstMessage>(message: &T) -> HttpResponse {
    let mut buffer = Vec::new();
    match message.encode(&mut buffer) {
        Ok(()) => HttpResponse::Ok().content_type("application/octet-stream").body(buffer),
        Err(error) => HttpResponse::InternalServerError().body(format!("failed to encode response: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_public_ipv4_from_request, register_relay_handler, RegisterRelayRequest, ServerState};
    use crate::client_manager::ClientManager;
    use crate::turn_manager::TurnManager;
    use actix_web::body::to_bytes;
    use actix_web::http::{header, StatusCode};
    use actix_web::{test as actix_test, web};
    use base64::Engine;
    use std::sync::Arc;

    fn basic_auth(secret: &str) -> String {
        let credentials = format!("user:{secret}");
        format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(credentials))
    }

    fn server_state(turn_manager: Arc<TurnManager>) -> web::Data<ServerState> {
        web::Data::new(ServerState {
            client_manager: ClientManager::new(),
            turn_manager
        })
    }

    #[test]
    fn extract_public_ipv4_accepts_ipv4_mapped_ipv6() {
        let req = actix_test::TestRequest::default()
            .insert_header(("X-Real-IP", "::ffff:198.51.100.8"))
            .to_http_request();

        assert_eq!(
            extract_public_ipv4_from_request(&req, "127.0.0.1:9000"),
            Some(std::net::Ipv4Addr::new(198, 51, 100, 8))
        );
    }

    #[test]
    fn extract_public_ipv4_rejects_pure_ipv6() {
        let req = actix_test::TestRequest::default().insert_header(("X-Real-IP", "2001:db8::1")).to_http_request();

        assert_eq!(extract_public_ipv4_from_request(&req, "127.0.0.1:9000"), None);
    }

    #[actix_web::test]
    async fn register_relay_handler_returns_ipv4_json() {
        let turn_manager = Arc::new(TurnManager::new().await);
        let state = server_state(turn_manager.clone());
        let req = actix_test::TestRequest::post()
            .insert_header((header::AUTHORIZATION, basic_auth("supersecret")))
            .insert_header(("X-Forwarded-For", "198.51.100.7"))
            .to_http_request();
        let body = web::Json(RegisterRelayRequest {
            stun_port: 3478,
            relay_port: 9101,
            relay_host: Some("127.0.0.1".to_string()),
            public_ip: None
        });

        let response = register_relay_handler(req, body, state).await;

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body()).await.unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "{\"ip_address\":\"198.51.100.7\"}");

        let relay = turn_manager.get_relay_config("client-1").await.unwrap();
        assert_eq!(relay.urls, vec!["stun:198.51.100.7:3478".to_string()]);
    }

    #[actix_web::test]
    async fn register_relay_handler_rejects_ipv6() {
        let state = server_state(Arc::new(TurnManager::new().await));
        let req = actix_test::TestRequest::post()
            .insert_header((header::AUTHORIZATION, basic_auth("supersecret")))
            .insert_header(("X-Real-IP", "2001:db8::1"))
            .to_http_request();
        let body = web::Json(RegisterRelayRequest {
            stun_port: 3478,
            relay_port: 9101,
            relay_host: Some("127.0.0.1".to_string()),
            public_ip: None
        });

        let response = register_relay_handler(req, body, state).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
