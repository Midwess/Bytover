use std::collections::HashMap;
use devlog_sdk::api_gateway::client::ApiGatewayClient;
use devlog_sdk::api_gateway::kong::client::KongGatewayAdminClient;
use devlog_sdk::api_gateway::service::{GatewayRouteBuilder, GatewayRouteExpression, GatewayServiceBuilder};
use devlog_sdk::tcp::listener::{TcpConnection, find_tcp_listener};
use futures_util::stream::SplitSink;
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use std::sync::{Arc, Weak, OnceLock};
use tokio::sync::{Mutex, OnceCell, mpsc, watch};
use tokio_tungstenite::tungstenite::{self, Message as WsMessage};
use tokio_tungstenite::tungstenite::handshake::server::Request;
use uuid::Uuid;
use devlog_sdk::distributed_id::gen_id;
use schema::devlog::rpc_signalling::server::{Message, ScopeState as ProtoScopeState, ScopeStateChanged};
use maxminddb;
use crate::scope_key::ScopeKey;
use crate::signaller::{ScopeRequest, ScopeRequestInner, ScopeState, SignallingMessage};
use crate::turn_manager::Continent;

pub struct Client {
    socket_id: String,
    join_id: OnceCell<String>,
    _ip_address: String,
    _continent: Continent,
    ws_sender: Mutex<SplitSink<tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, WsMessage>>,
    sent_message_order_id: Mutex<HashMap<String, u64>>,
    turn_manager: Arc<crate::turn_manager::TurnManager>,
    pub(crate) scopes: Arc<Mutex<Vec<ScopeKey>>>,
    scope_state_receivers: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
    state_change_tx: mpsc::Sender<(String, ScopeState)>,
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.socket_id == other.socket_id
    }
}

impl Client {
    pub async fn handle(
        websocket: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        scope_request_tx: mpsc::Sender<ScopeRequest>,
        ip_address: String,
        continent: Continent,
        turn_manager: Arc<crate::turn_manager::TurnManager>,
    ) {
        let (ws_sender, mut ws_receiver) = websocket.split();
        let (state_change_tx, mut state_change_rx) = mpsc::channel::<(String, ScopeState)>(64);

        let socket_id = Uuid::new_v4().to_string();
        log::info!(target: &format!("client-{}", socket_id), "Connected from IP: {} ({:?})", ip_address, continent);

        let this = Arc::new(Self {
            socket_id: socket_id.clone(),
            join_id: OnceCell::new(),
            _ip_address: ip_address.clone(),
            _continent: continent,
            ws_sender: Mutex::new(ws_sender),
            sent_message_order_id: Default::default(),
            turn_manager: Arc::clone(&turn_manager),
            scopes: Arc::new(Mutex::new(Vec::new())),
            scope_state_receivers: Mutex::new(HashMap::new()),
            state_change_tx,
        });

        let _this_drop_guard = this.clone();

        loop {
            tokio::select! {
                biased;

                Some((scope_id, state)) = state_change_rx.recv() => {
                    this.send_scope_state_change(&scope_id, &state).await;
                }

                ws_msg = ws_receiver.next() => {
                    let Some(Ok(message)) = ws_msg else {
                        break;
                    };

                    let tungstenite::Message::Binary(message) = message else {
                        continue;
                    };

                    if message.len() > 1024 * 32 {
                        log::info!(target: &format!("client-{}", this.id()), "Message too large {} bytes, ignoring", message.len());
                        continue;
                    }

                    let Ok(message) = Message::decode(&message[..]) else {
                        continue;
                    };

                    let message = SignallingMessage::new(Box::new(message)).await;
                    let mut request_scopes = message.scopes.iter().map(|it| ScopeKey::new(it)).collect::<Vec<_>>();
                    request_scopes.truncate(64);

                    if message.join.is_some() {
                        if !message.from_id.is_empty() {
                            let _ = this.join_id.set(message.from_id.clone());
                        }

                        this.turn_manager.register_client(message.from_id.clone(), continent).await;

                        let current_scopes = this.get_scopes().await;

                        for scope in current_scopes.iter() {
                            if !request_scopes.contains(scope) {
                                log::info!(
                                    target: &format!("client-{}", this.id()),
                                    "Unsubscribing from scope {} (is_owner: {})",
                                    scope.scope_id, scope.is_owner
                                );

                                this.remove_scope_receiver(&scope.scope_id).await;

                                if let Err(e) = scope_request_tx
                                    .send(ScopeRequest {
                                        scope_id: scope.scope_id.clone(),
                                        request: ScopeRequestInner::Unsubscribe(this.id().clone())
                                    })
                                    .await
                                {
                                    log::error!(target: &format!("client-{}", this.id()), "Failed to send unsubscribe request: {}", e);
                                }
                            }
                        }

                        for request_scope in request_scopes.iter() {
                            if !current_scopes.contains(&request_scope) {
                                log::info!(
                                    target: &format!("client-{}", this.id()),
                                    "Subscribing to scope {} (is_owner: {})",
                                    request_scope.scope_id, request_scope.is_owner
                                );

                                let request = ScopeRequestInner::Join {
                                    client: Arc::downgrade(&this),
                                    is_owner: request_scope.is_owner,
                                    is_direct: request_scope.is_direct
                                };

                                if let Err(e) = scope_request_tx
                                    .send(ScopeRequest {
                                        scope_id: request_scope.scope_id.clone(),
                                        request
                                    })
                                    .await
                                {
                                    log::error!(target: &format!("client-{}", this.id()), "Failed to send join request: {}", e);
                                }
                            }
                        }
                    }

                    for scope in request_scopes {
                        if !scope.should_broad_cast() {
                            continue;
                        }

                        if let Err(e) = scope_request_tx
                            .send(ScopeRequest {
                                scope_id: scope.scope_id.clone(),
                                request: ScopeRequestInner::Broadcast(message.clone())
                            })
                            .await
                        {
                            log::error!(target: &format!("client-{}", this.id()), "Failed to send broadcast request: {}", e);
                        }
                    }
                }
            }
        }

        if let Some(join_id) = this.join_id.get() {
            this.turn_manager.unregister_client(&join_id).await;
        }

        let scopes = this.get_scopes().await;
        for scope in scopes.iter() {
            log::info!(target: &format!("client-{}", this.id()), "Leaving scope: {}", scope.scope_id);
            if let Err(e) = scope_request_tx
                .send(ScopeRequest {
                    scope_id: scope.scope_id.clone(),
                    request: ScopeRequestInner::Leave(this.id(), gen_id().await)
                })
                .await
            {
                log::error!(target: &format!("client-{}", this.id()), "Failed to send leave request: {}", e);
            }
        }
    }

    async fn send_scope_state_change(&self, scope_id: &str, state: &ScopeState) {
        let proto_state = if state.is_online {
            ProtoScopeState::Online
        } else {
            ProtoScopeState::Offline
        };

        let state_msg = Box::new(Message {
            scope_state_changed: Some(ScopeStateChanged {
                scope_id: scope_id.to_string(),
                state: proto_state as i32,
                owner_id: state.owner_id.clone(),
            }),
            ..Default::default()
        });

        let msg = SignallingMessage::broadcast(state_msg);
        self.send_internal(msg).await;
    }

    pub async fn add_scope_receiver(&self, scope_id: String, mut receiver: watch::Receiver<ScopeState>) {
        let current_state = receiver.borrow().clone();
        if current_state.is_online {
            let _ = self.state_change_tx.send((scope_id.clone(), current_state)).await;
        }

        let tx = self.state_change_tx.clone();
        let scope_id_clone = scope_id.clone();
        let handle = tokio::spawn(async move {
            loop {
                if receiver.changed().await.is_err() {
                    break;
                }
                
                let state = receiver.borrow().clone();
                if tx.send((scope_id_clone.clone(), state)).await.is_err() {
                    break;
                }
            }
        });

        let mut receivers = self.scope_state_receivers.lock().await;
        receivers.insert(scope_id, handle);
    }

    pub async fn remove_scope_receiver(&self, scope_id: &str) {
        let mut receivers = self.scope_state_receivers.lock().await;
        if let Some(handle) = receivers.remove(scope_id) {
            handle.abort();
        }
    }

    async fn send_internal(&self, message: SignallingMessage) {
        let mut buf = Vec::new();
        let encoded = message.encode(&mut buf);
        if encoded.is_err() {
            log::error!(target: &format!("client-{}", self.id()), "Failed to encode message: {}", encoded.err().unwrap());
            return;
        }

        let mut ws_sender = self.ws_sender.lock().await;
        if let Err(e) = ws_sender.send(WsMessage::Binary(buf.into())).await {
            log::error!(target: &format!("client-{}", self.id()), "Failed to send internal message: {}", e);
        }
    }

    #[inline]
    pub fn id(&self) -> String {
        self.socket_id.clone()
    }

    #[inline]
    pub fn client_id(&self) -> String {
        self.join_id.get().cloned().unwrap_or_default()
    }

    pub async fn add_scope(&self, scope: ScopeKey) {
        let mut scopes = self.scopes.lock().await;
        if !scopes.contains(&scope) {
            scopes.push(scope);
        }
    }

    pub async fn remove_scope(&self, scope: &ScopeKey) {
        let mut scopes = self.scopes.lock().await;
        scopes.retain(|s| s != scope);
    }

    pub async fn get_scopes(&self) -> Vec<ScopeKey> {
        let scopes = self.scopes.lock().await;
        scopes.clone()
    }

    #[inline]
    pub async fn send_weak(this: &Weak<Self>, message: SignallingMessage) {
        if let Some(client) = this.upgrade() {
            client.send(message).await;
        }
    }

    pub async fn send(self: &Arc<Self>, mut message: SignallingMessage) {
        if message.from_id.eq(&self.client_id()) {
            return;
        }

        if let Some(to_id) = message.to_id.as_ref() &&
            to_id.ne(&self.client_id())
        {
            return;
        }

        if message.join.is_some() || message.offer.is_some() {
            if let Some(ice_config) = self.turn_manager.get_turn_for_message(
                &self.client_id(),
                Some(&message.from_id)
            ).await {
                log::info!(
                    target: &format!("client-{}", self.id()),
                    "Selected ICE config for peer {} <-> {}: {:?}",
                    self.client_id(),
                    message.from_id,
                    ice_config
                );

                if let Some(ref mut join) = message.message.join {
                    join.ice_config = Some(ice_config.clone());
                }

                if let Some(ref mut offer) = message.message.offer {
                    offer.ice_config = Some(ice_config);
                }
            }
        }

        let from_id = message.from_id.clone();
        let mut queue_guard = self.sent_message_order_id.lock().await;
        if queue_guard.len() > 200 {
            // Keep only the most recent 100 entries to avoid cache thrashing
            let mut entries: Vec<_> = queue_guard
                .iter()
                .map(|(key, order)| (key.clone(), *order))
                .collect();
            entries.sort_by_key(|(_, order)| std::cmp::Reverse(*order));
            entries.truncate(100);

            queue_guard.clear();
            for (key, order) in entries {
                queue_guard.insert(key, order);
            }
        }

        let queue_order = queue_guard.entry(from_id).or_insert(0);
        if message.id != 0 {
            if message.id <= *queue_order {
                return;
            }

            *queue_order = message.id;
        }

        drop(queue_guard);

        let mut buf = Vec::new();
        let encoded = message.encode(&mut buf);
        if encoded.is_err() {
            log::error!(target: &format!("client-{}", self.id()), "Failed to encode message: {}", encoded.err().unwrap());
            return;
        }

        let mut ws_sender = self.ws_sender.lock().await;
        match ws_sender.send(WsMessage::Binary(buf.into())).await {
            Err(_) => {
                log::error!(target: &format!("client-{}", self.id()), "Failed to send message");
            },
            Ok(()) => {}
        }
    }
}

fn extract_ip_from_request(req: &Request, peer_addr: &str) -> String {
    if let Some(cf_ip) = req.headers().get("CF-Connecting-IP") {
        if let Ok(ip) = cf_ip.to_str() {
            return ip.split(',').next().unwrap().trim().to_string();
        }
    }

    if let Some(xff) = req.headers().get("X-Forwarded-For") {
        if let Ok(list) = xff.to_str() {
            return list.split(',').next().unwrap().trim().to_string();
        }
    }

    if let Some(xri) = req.headers().get("X-Real-IP") {
        if let Ok(ip) = xri.to_str() {
            return ip.to_string();
        }
    }

    peer_addr.to_string()
}

pub struct SignallingServer {
    scope_request_tx: mpsc::Sender<ScopeRequest>,
    geoip_reader: Option<Arc<maxminddb::Reader<Vec<u8>>>>,
    turn_manager: Arc<crate::turn_manager::TurnManager>,
}

impl SignallingServer {
    pub fn new(scope_request_tx: mpsc::Sender<ScopeRequest>, turn_manager: Arc<crate::turn_manager::TurnManager>) -> Self {
        let geoip_data = include_bytes!("../GeoLite2-Country.mmdb");
        let geoip_reader = maxminddb::Reader::from_source(geoip_data.to_vec())
            .ok()
            .map(Arc::new);

        if geoip_reader.is_none() {
            log::warn!(target: "websocket", "GeoIP database not found or invalid");
        }

        Self {
            scope_request_tx,
            geoip_reader,
            turn_manager,
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let tcp_listener = find_tcp_listener(Some(3003)).await?;

        self.setup_gateway(&tcp_listener).await?;

        let std_listener = tcp_listener.listener;
        std_listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(std_listener)?;

        log::info!(
            target: "websocket",
            "RPC Signalling Server listening on: {} port: {}",
            tcp_listener.public_host,
            tcp_listener.port
        );

        let geoip_reader = self.geoip_reader.clone();
        let turn_manager = Arc::clone(&self.turn_manager);

        while let Ok((stream, addr)) = listener.accept().await {
            let scope_request_tx = self.scope_request_tx.clone();
            let peer_addr = addr.ip().to_string();
            let geoip_reader_clone = geoip_reader.clone();
            let turn_manager_clone = Arc::clone(&turn_manager);

            tokio::spawn(async move {
                let ip_address = Arc::new(OnceLock::new());
                let ip_address_clone = Arc::clone(&ip_address);

                let callback = |req: &Request, resp| {
                    let ip = extract_ip_from_request(req, &peer_addr);
                    let _ = ip_address_clone.set(ip);
                    Ok(resp)
                };

                match tokio_tungstenite::accept_hdr_async(stream, callback).await {
                    Ok(ws_stream) => {
                        let final_ip = ip_address.get().cloned().unwrap_or_else(|| peer_addr.clone());
                        let continent = crate::turn_manager::detect_continent(
                            &final_ip,
                            geoip_reader_clone.as_ref().map(|r| r.as_ref())
                        );

                        tokio::spawn(async move {
                            Client::handle(ws_stream, scope_request_tx.clone(), final_ip, continent, turn_manager_clone).await;
                        });
                    }
                    Err(e) => {
                        log::error!(target: "websocket", "WebSocket handshake error: {}", e);
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn setup_gateway(&self, connection: &TcpConnection) -> Result<(), Box<dyn std::error::Error>> {
        let api_gateway = KongGatewayAdminClient {
            url: devlog_sdk::config::CONFIGS.kong.admin_url.clone()
        };

        let service = GatewayServiceBuilder::new()
            .http(connection.public_host.clone(), connection.port)
            .enable_cors(true)
            .name("rpc-signalling-websocket-server")
            .routes(vec![
                GatewayRouteBuilder::new()
                    .path(GatewayRouteExpression::start_with("/rpc-signalling"))
                    .http()
                    .methods(vec!["GET".to_owned(), "OPTIONS".to_owned()])
                    .strip_path(true)
                    .public(true)
                    .preserve_host(false)
                    .priority(10)
                    .name("devlog-rpc-signalling-websocket-server-path")
                    .build(),
            ])
            .build();

        api_gateway.register(service).await?;
        log::info!(target: "websocket", "Registered http service to gateway");

        Ok(())
    }
}
