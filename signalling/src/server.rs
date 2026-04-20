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
    let key = key.into_inner();
    let _ = state.client_manager.get(&key).await;

    let relay_config = match state.turn_manager.get_relay_config(&key).await {
        Some(config) => config,
        None => return HttpResponse::ServiceUnavailable().body("client not connected")
    };

    encode_binary_response(&relay_config)
}

fn parse_submitted_ipv4(value: &str) -> Option<std::net::Ipv4Addr> {
    let value = value.trim();

    if let Ok(ip) = value.parse::<std::net::Ipv4Addr>() {
        return Some(ip);
    }

    if let Ok(ip) = value.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(ipv4) => Some(ipv4),
            std::net::IpAddr::V6(ipv6) => ipv6.to_ipv4()
        };
    }

    if let Ok(socket_addr) = value.parse::<std::net::SocketAddr>() {
        return match socket_addr.ip() {
            std::net::IpAddr::V4(ipv4) => Some(ipv4),
            std::net::IpAddr::V6(ipv6) => ipv6.to_ipv4()
        };
    }

    None
}

fn parse_submitted_ipv6(value: &str) -> Option<std::net::Ipv6Addr> {
    let value = value.trim().trim_start_matches('[').trim_end_matches(']');

    if let Ok(ip) = value.parse::<std::net::Ipv6Addr>() {
        return Some(ip);
    }

    if let Ok(ip) = value.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V6(ipv6) => Some(ipv6),
            std::net::IpAddr::V4(_) => None
        };
    }

    if let Ok(socket_addr) = value.parse::<std::net::SocketAddrV6>() {
        return Some(*socket_addr.ip());
    }

    None
}

#[derive(serde::Deserialize)]
struct RegisterRelayRequest {
    stun_port: u16,
    relay_port: u16,
    public_ipv4: Option<String>,
    public_ipv6: Option<String>,
    #[serde(default)]
    turn_port: u16,
    #[serde(default)]
    turn_username: Option<String>,
    #[serde(default)]
    turn_password: Option<String>,
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

    let public_ipv4 = body.public_ipv4.as_deref().and_then(parse_submitted_ipv4).map(|ip| ip.to_string());
    let public_ipv6 = body.public_ipv6.as_deref().and_then(parse_submitted_ipv6).map(|ip| ip.to_string());

    let response_ip = match public_ipv4.clone().or_else(|| public_ipv6.clone()) {
        Some(ip) => ip,
        None => return HttpResponse::BadRequest().body("relay registration requires at least one public IP address")
    };

    state.turn_manager.register_relay(public_ipv4, public_ipv6, body.stun_port, body.relay_port, body.turn_port, body.turn_username.clone(), body.turn_password.clone()).await;

    HttpResponse::Ok().json(RegisterRelayResponse { ip_address: response_ip })
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
    use super::{register_relay_handler, RegisterRelayRequest, ServerState};
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

    #[actix_web::test]
    async fn register_relay_handler_returns_dual_stack_config() {
        let turn_manager = Arc::new(TurnManager::new().await);
        let state = server_state(turn_manager.clone());
        let req = actix_test::TestRequest::post()
            .insert_header((header::AUTHORIZATION, basic_auth("supersecret")))
            .to_http_request();
        let body = web::Json(RegisterRelayRequest {
            stun_port: 3478,
            relay_port: 9101,
            public_ipv4: Some("198.51.100.7".to_string()),
            public_ipv6: Some("2001:db8::7".to_string()),
            turn_port: 19101
        });

        let response = register_relay_handler(req, body, state).await;

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body()).await.unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "{\"ip_address\":\"198.51.100.7\"}");

        let relay = turn_manager.get_relay_config("client-1").await.unwrap();
        assert_eq!(
            relay.urls,
            vec![
                "stun:198.51.100.7:3478".to_string(),
                "stun:[2001:db8::7]:3478".to_string()
            ]
        );
        let assigned = turn_manager.get_assigned_relay("client-1").await.unwrap();
        assert_eq!(assigned.relay_host, "198.51.100.7");
    }

    #[actix_web::test]
    async fn register_relay_handler_accepts_ipv6_only() {
        let turn_manager = Arc::new(TurnManager::new().await);
        let state = server_state(turn_manager.clone());
        let req = actix_test::TestRequest::post()
            .insert_header((header::AUTHORIZATION, basic_auth("supersecret")))
            .to_http_request();
        let body = web::Json(RegisterRelayRequest {
            stun_port: 3478,
            relay_port: 9101,
            public_ipv4: None,
            public_ipv6: Some("2001:db8::8".to_string()),
            turn_port: 19101
        });

        let response = register_relay_handler(req, body, state).await;

        assert_eq!(response.status(), StatusCode::OK);
        let relay = turn_manager.get_relay_config("client-1").await.unwrap();
        assert_eq!(relay.urls, vec!["stun:[2001:db8::8]:3478".to_string()]);
        let assigned = turn_manager.get_assigned_relay("client-1").await.unwrap();
        assert_eq!(assigned.relay_host, "2001:db8::8");
    }

    #[actix_web::test]
    async fn register_relay_handler_rejects_missing_public_addresses() {
        let state = server_state(Arc::new(TurnManager::new().await));
        let req = actix_test::TestRequest::post()
            .insert_header((header::AUTHORIZATION, basic_auth("supersecret")))
            .to_http_request();
        let body = web::Json(RegisterRelayRequest {
            stun_port: 3478,
            relay_port: 9101,
            public_ipv4: None,
            public_ipv6: None,
            turn_port: 19101
        });

        let response = register_relay_handler(req, body, state).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
