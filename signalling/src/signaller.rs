use std::collections::{HashMap, VecDeque};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};
use std::time::Duration;

use crate::scope_key::ScopeKey;
use crate::websocket::Client;
use futures_util::future::join_all;
use schema::devlog::rpc_signalling::server::{LeftMessage, Message};
use tokio::sync::{mpsc, watch};
use devlog_sdk::distributed_id::gen_id;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ScopeState {
    pub is_online: bool,
    pub owner_id: Option<String>,
}

impl ScopeState {
    pub fn online(owner_id: String) -> Self {
        Self { is_online: true, owner_id: Some(owner_id) }
    }

    pub fn offline() -> Self {
        Self { is_online: false, owner_id: None }
    }
}

#[derive(Clone, Debug)]
pub struct SignallingMessage {
    pub id: u64,
    pub message: Box<Message>,
}

impl SignallingMessage {
    pub async fn new(message: Box<Message>) -> Self {
        Self {
            id: gen_id().await,
            message
        }
    }

    pub fn broadcast(message: Box<Message>) -> Self {
        Self {
            id: 0,
            message
        }
    }
}

impl Deref for SignallingMessage {
    type Target = Message;
    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl DerefMut for SignallingMessage {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.message
    }
}

pub struct Scope {
    id: String,
    owner: Option<Weak<Client>>,
    clients: HashMap<String, Weak<Client>>,
    is_direct: bool,
    state_tx: watch::Sender<ScopeState>,
}

impl Scope {
    pub fn new(id: String, is_direct: bool) -> Self {
        let (state_tx, _) = watch::channel(ScopeState::offline());
        Self {
            id,
            clients: HashMap::new(),
            is_direct,
            owner: None,
            state_tx,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<ScopeState> {
        self.state_tx.subscribe()
    }

    fn update_state(&self, state: ScopeState) {
        let _ = self.state_tx.send_replace(state);
    }

    pub async fn add_client(&mut self, client: Weak<Client>, is_owner: bool) {
        if let Some(client) = client.upgrade() {
            if self.owner_client_id().map(|it| it.eq(&client.client_id())).unwrap_or(false) {
                return;
            }

            if is_owner && !self.owner_present() {
                log::info!(target: &format!("scope-{}", self.id), "Owned by {}", client.client_id());
                self.owner = Some(Arc::downgrade(&client));
                client.add_scope(ScopeKey::from_parts(
                    self.id.clone(),
                    self.is_direct,
                    true
                )).await;


                self.update_state(ScopeState::online(client.client_id()));
                log::info!(target: &format!("scope-{}", self.id), "Online = {}", self.subscribe().borrow().is_online);

                if !self.is_direct {
                    self.clients.insert(client.id(), Arc::downgrade(&client));
                }
            }
            else {
                if self.clients.contains_key(&client.id()) {
                    return;
                }

                self.clients.insert(client.id(), Arc::downgrade(&client));
            }

            let scope_info = ScopeKey::from_parts(
                self.id.clone(),
                self.is_direct,
                is_owner
            );

            client.add_scope(scope_info).await;
        }
        else {
            log::warn!(target: &format!("scope-{}", self.id), "Client disconnected immediately after join");
        }
    }

    #[inline]
    pub fn owner_client_id(&self) -> Option<String> {
        let Some(owner) = self.owner.as_ref().and_then(|owner| owner.upgrade()) else {
            return None;
        };

        Some(owner.client_id())
    }

    #[inline]
    pub fn owner_present(&self) -> bool {
        self.owner.as_ref().and_then(|owner| owner.upgrade()).is_some()
    }

    pub async fn cleanup(&mut self) -> bool {
        let clients_before = self.clients.len();
        self.clients.retain(|_, weak_client| weak_client.upgrade().is_some());
        let clients_removed = clients_before - self.clients.len();

        if clients_removed > 0 {
            log::info!(
                target: &format!("scope-{}", self.id),
                "Cleanup: removed {} clients. Remaining: {} clients",
                clients_removed, self.clients.len()
            );
        }

        if let Some(owner_ref) = self.owner.as_ref() {
            if owner_ref.upgrade().is_some() {
                return false
            }
            else {
                self.owner.take();
                log::info!(target: &format!("scope-{}", self.id), "Owner disconnected, sending OFFLINE state to {} clients",
                    self.clients.len());
                self.update_state(ScopeState::offline());
            }
        };

        self.clients.is_empty() && self.owner.is_none()
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub async fn broadcast(&self, mut message: SignallingMessage, max_concurrent_requests: usize) {
        message.from_scope = Some(self.id.clone());

        let recipients: Vec<&Weak<Client>> = if self.is_direct {
            let Some(owner_client_id) = self.owner_client_id() else {
                return;
            };

            let is_from_owner = message.from_id.eq(&owner_client_id);

            if is_from_owner {
                self.clients.iter().map(|(_, client)| client).collect()
            }
            else {
                message.to_id = self.owner_client_id();
                vec![self.owner.as_ref().unwrap()]
            }
        }
        else {
            self.clients.values().collect()
        };

        let mut client_iter = recipients.into_iter();
        loop {
            let mut futures = Vec::with_capacity(max_concurrent_requests);
            let mut count = 0;

            while count < max_concurrent_requests {
                if let Some(client) = client_iter.next() {
                    futures.push(Client::send_weak(client, message.clone()));
                    count += 1;
                } else {
                    break;
                }
            }

            if futures.is_empty() {
                break;
            }

            join_all(futures).await;
        }
    }

    pub async fn remove_client(&mut self, id: String) {
        let is_owner = self.owner.as_ref()
            .and_then(|o| o.upgrade())
            .map(|o| o.id() == id)
            .unwrap_or(false);

        let scope_to_remove = ScopeKey::from_parts(
            self.id.clone(),
            self.is_direct,
            is_owner
        );

        if let Some(client_weak) = self.clients.get(&id) {
            if let Some(client) = client_weak.upgrade() {
                client.remove_scope(&scope_to_remove).await;
            }
        }

        if let Some(owner) = self.owner.as_ref().and_then(|owner| owner.upgrade()) {
            if owner.id().eq(&id) {
                owner.remove_scope(&scope_to_remove).await;
                self.owner = None;
                log::info!(target: &format!("scope-{}", self.id), "Owner disconnected");
                self.update_state(ScopeState::offline());
           }
        }

        self.clients.remove(&id);
    }
}

#[derive(Clone)]
pub struct ScopeRequest {
    pub scope_id: String,
    pub request: ScopeRequestInner
}

#[derive(Clone)]
pub enum ScopeRequestInner {
    Join {
        client: Weak<Client>,
        is_owner: bool,
        is_direct: bool
    },
    Broadcast(SignallingMessage),
    Leave(String, u64),
    Unsubscribe(String)
}

pub struct Signaller {
    scopes: HashMap<String, Scope>,
    scope_request_tx: mpsc::Sender<ScopeRequest>,
    scope_request_rx: mpsc::Receiver<ScopeRequest>,
}

impl Default for Signaller {
    fn default() -> Self {
        Self::new()
    }
}

impl Signaller {
    pub fn new() -> Self {
        let (scope_request_tx, scope_request_rx) = mpsc::channel(2048);
        Self {
            scopes: HashMap::new(),
            scope_request_tx,
            scope_request_rx,
        }
    }

    pub fn request_tx(&self) -> mpsc::Sender<ScopeRequest> {
        self.scope_request_tx.clone()
    }

    pub async fn run(mut self) {
        let mut broadcast_requests: VecDeque<(String, SignallingMessage)> = VecDeque::with_capacity(512);
        let _max_concurrent_requests = 512usize;
        let mut broadcast_ticker = tokio::time::interval(Duration::from_millis(8));
        let mut cleanup_ticker = tokio::time::interval(Duration::from_secs(15));
        let mut logger_ticker = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                Some(request) = self.scope_request_rx.recv() => {
                    match request.request {
                        ScopeRequestInner::Broadcast(message) => {
                            broadcast_requests.push_back((request.scope_id.clone(), message));
                        }
                        ScopeRequestInner::Join { client, is_owner, is_direct } => {
                            let scope = if let Some(scope) = self.scopes.get_mut(&request.scope_id) {
                                scope.add_client(client.clone(), is_owner).await;
                                scope
                            }
                            else {
                                if is_owner && !is_direct {
                                    log::warn!(target: &format!("scope-{}", request.scope_id), "Owner cannot join a non-direct scope");
                                    continue;
                                }

                                let mut scope = Scope::new(request.scope_id.clone(), is_direct);
                                scope.add_client(client.clone(), is_owner).await;
                                self.scopes.insert(request.scope_id.clone(), scope);
                                self.scopes.get(&request.scope_id).unwrap()
                            };

                            if let Some(client_arc) = client.upgrade() {
                                let receiver = scope.subscribe();
                                client_arc.add_scope_receiver(request.scope_id.clone(), receiver).await;
                            }
                        }
                        ScopeRequestInner::Unsubscribe(id) => {
                            if let Some(scope) = self.scopes.get_mut(&request.scope_id) {
                                let is_owner = scope.owner.as_ref()
                                    .and_then(|o| o.upgrade())
                                    .map(|o| o.id() == id)
                                    .unwrap_or(false);
                                log::info!(
                                    target: &format!("scope-{}", request.scope_id),
                                    "Client {} unsubscribing (is_owner: {})",
                                    id, is_owner
                                );
                                scope.remove_client(id).await;
                            }
                        }
                        ScopeRequestInner::Leave(id, msg_id) => {
                            let msg = Box::new(Message {
                                from_id: id.clone(),
                                left_message: Some(LeftMessage {
                                    id: id.clone()
                                }),
                                ..Default::default()
                            });

                            if let Some(scope) = self.scopes.get_mut(&request.scope_id) {
                                scope.remove_client(id).await;
                                let message = SignallingMessage { message: msg, id: msg_id };
                                broadcast_requests.push_back((request.scope_id.clone(), message));
                            }
                        }
                    }
                }
                _ = broadcast_ticker.tick() => {
                    if broadcast_requests.is_empty() {
                        continue;
                    }

                    let mut scope_messages: HashMap<String, Vec<SignallingMessage>> = HashMap::with_capacity(broadcast_requests.len());
                    for (scope_id, message) in std::mem::take(&mut broadcast_requests) {
                        scope_messages.entry(scope_id).or_insert_with(Vec::new).push(message);
                    }

                    let mut scope_futures = Vec::new();
                    for (scope_id, messages) in scope_messages {
                        if let Some(scope) = self.scopes.get(&scope_id) {
                            scope_futures.push(async {
                                for message in messages {
                                    scope.broadcast(message, 64).await;
                                }
                            });
                        }
                    }

                    // Wait for all scopes to finish processing their messages
                    join_all(scope_futures).await;
                }
                _ = cleanup_ticker.tick() => {
                    let mut removed_scopes = Vec::new();
                    for (scope_id, scope) in self.scopes.iter_mut() {
                        if scope.cleanup().await {
                            removed_scopes.push(scope_id.clone());
                        }
                    }

                    for scope_id in removed_scopes {
                        self.scopes.remove(&scope_id);
                    }
                }
                _ = logger_ticker.tick() => {
                    if !self.scopes.is_empty() {
                        log::info!(target: "signaller", "Total scopes: {:?}", self.scopes.len());
                        let total_clients = self.scopes.values().map(|scope| scope.len()).sum::<usize>();
                        let average_clients = total_clients / self.scopes.len();
                        log::info!(target: "signaller", "Total members: {:?}, Average members per scope: {:?}", total_clients, average_clients);
                    }
                }
            }
        }
    }
}
