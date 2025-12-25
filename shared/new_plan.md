# New P2P Architecture Plan

## Current Approach (To Remove)
- Direct P2P transfer via `TransferRequestMessage`
- Sender initiates full session transfer to receiver
- Receiver accepts entire session at once

## New Approach

### Overview
1. **Session Discovery**: When peers connect, sender broadcasts lightweight session overviews
2. **Authentication**: Receiver sees sessions, authenticates with password if needed
3. **Selective Download**: Receiver requests individual resources, not entire session

### Flow

```
Sender                          Receiver
  │                                │
  │  1. Peer Connected             │
  ├───────────────────────────────>│
  │   IntroduceRequest/Response    │
  │   (peer info only)             │
  │                                │
  │  2. Sessions Notification      │
  ├───────────────────────────────>│
  │   P2PSessionOverview[] (list)  │
  │                                │
  │                        3. Display sessions in UI
  │                        4. User clicks session
  │                                │
  │  5. ViewSessionDetailRequest   │
  │<───────────────────────────────┤
  │     (with password if needed)  │
  │                                │
  │  6. Validate password          │
  │     Send full session or error │
  ├───────────────────────────────>│
  │   ViewSessionDetailResponse    │
  │                                │
  │                        7. Display resources
  │                        8. User clicks download
  │                        9. Prepare IOWriter
  │                                │
  │  10. DownloadResourceRequest   │
  │<───────────────────────────────┤
  │                                │
  │  11. Stream resource data      │
  │      immediately (no response) │
  ├═══════════════════════════════>│
  │   (existing transfer protocol) │
  │                                │
  │                        12. Write to IOWriter
```

---

## Changes Required

### 1. Proto Schema (`libs/schema/proto/devlog/bitbridge/`)

#### `session.proto`
```protobuf
message P2PSessionOverviewMessage {
  required uint64 order_id = 1;
  required bool password_protected = 2;
}

message P2PTransferSessionMessage {
  required uint64 order_id = 1;
  repeated ResourceMessage resources = 2;
}
```

#### `request.proto`
**Add:**
```protobuf
message SessionsNotificationMessage {
  repeated P2PSessionOverviewMessage sessions = 1;
}

message ViewSessionDetailRequest {
  required uint64 order_id = 1;
  optional string password = 2;
}

message ViewSessionDetailResponse {
  oneof result {
    P2PTransferSessionMessage session = 1;
    PeerErrorsMessage error = 2;
  }
}

message DownloadResourceRequest {
  required uint64 session_order_id = 1;
  required uint64 resource_order_id = 2;
}
```

**Update PeerMessageBody:**
```protobuf
oneof request {
  IntroduceRequestMessage introduce_request = 2;
  CancelTransferSessionRequest cancel_request = 4;
  KeepAliveRequestMessage keep_alive = 5;
  ResourceThumbnailMessage resource_thumbnail_fullfill = 6;
  FecFeedback fec_feedback = 7;
  SessionsNotificationMessage sessions_notification = 8;
  ViewSessionDetailRequest view_session_request = 9;
  DownloadResourceRequest download_resource_request = 10;
}

oneof response {
  IntroduceResponseMessage introduce_response = 30;
  VoidResponseMessage void_response = 32;
  PeerErrorsMessage errors = 33;
  ViewSessionDetailResponse view_session_response = 34;
}
```

**IntroduceResponseMessage unchanged:**
```protobuf
message IntroduceResponseMessage {
  required PeerMessage peer = 1;
}
```

**Remove:**
- `TransferRequestMessage`
- `TransferResponseMessage`

**Extend errors:**
```protobuf
enum PeerErrorsMessage {
  InvalidRequest = 1;
  NoResponse = 2;
  InvalidPassword = 3;
  SessionNotFound = 4;
  SessionExpired = 5;
  PermissionDenied = 6;
  ResourceNotFound = 7;
  TransferInitFailed = 8;
}
```

---

### 2. Rust Entities (`shared/src/entities/`)

#### Update `target.rs` - Add password fields to P2P:
```rust
pub enum TransferTarget {
    P2P {
        from_peer: Peer,
        password: Option<String>,
        is_required_password: bool,
    },
    Internet {
        password: Option<String>,
        access_url: Option<String>,
        from_user: User,
        to_emails: Vec<String>,
        is_required_password: bool
    }
}
```

#### Fix references in other files (Nearby → P2P):
Files that need updating:
- `entities/transfer_session.rs`: `Nearby(peer)` → `P2P { from_peer: peer, .. }`
- `app/nearby/command.rs`: `Nearby(peer)` → `P2P { from_peer: peer, .. }`
- `repository/transfer_session.rs`: `Nearby(_)` → `P2P { .. }`

#### TransferSession mapping:
- `P2PSessionOverviewMessage` → minimal `TransferSession` (order_id only, empty resources)
- `P2PTransferSessionMessage` → full `TransferSession` (with resources populated)

---

### 3. Operations (`shared/src/app/operations/`)

#### 3.1 `p2p.rs` - Current state:
```rust
pub enum P2POperation {
    StartNearbyServer(Peer),      // KEEP
    StopNearbyServer,             // KEEP
    UpdateFindingScopes(Vec<FindingScope>), // KEEP
    PeerEvents(String),           // KEEP
    IsRunning                     // KEEP
}

pub enum P2POperationOutput {
    PeerConnected(Peer),          // KEEP
    PeerDisconnected(),           // KEEP
    ReceivedSessionRequest { remote_session: TransferSessionMessage }, // DELETE
    CancelSessionRequest { session_id: u64 }, // KEEP
    NearbyServerStopped,          // KEEP
    AlreadyRunning                // KEEP
}
```

#### 3.2 `p2p.rs` - Add to `P2POperation`:
```rust
SendSessionsNotification {
    peer_id: String,
    sessions: Vec<TransferSession>,
}

ViewSessionDetail {
    peer_id: String,
    order_id: u64,
    password: Option<String>,
}

SendSessionDetail {
    peer_id: String,
    session: TransferSession,
}

SendSessionDetailError {
    peer_id: String,
    order_id: u64,
    error: String,
}

DownloadResource {
    peer_id: String,
    session_order_id: u64,
    resource_order_id: u64,
}

StreamResourceToPeer {
    peer_id: String,
    resource: LocalResource,
}

PrepareReceiveResource {
    peer_id: String,
    session_order_id: u64,
    resource_order_id: u64,
    save_path: String,
}
```

#### 3.3 `p2p.rs` - Add to `P2POperationOutput`:
```rust
ReceivedSessionOverview {
    peer_id: String,
    order_id: u64,
    password_protected: bool,
}

ReceivedViewSessionRequest {
    peer_id: String,
    order_id: u64,
    password: Option<String>,
}

SessionDetailReceived {
    session: TransferSession,
}

SessionDetailFailed {
    order_id: u64,
    error: String,
}

ReceivedDownloadRequest {
    peer_id: String,
    session_order_id: u64,
    resource_order_id: u64,
}
```

#### 3.4 `p2p.rs` - Delete from `P2POperationOutput`:
```rust
ReceivedSessionRequest { remote_session: TransferSessionMessage } // DELETE
```

#### 3.5 `transfer.rs` - Current state:
```rust
pub enum TransferOperation {
    CreateCloudSession(TransferSession),  // KEEP (public transfer)
    SendSession(TransferSession),         // KEEP (public transfer)
    AnswerSessionRequest {                // DELETE (old P2P flow)
        peer_id: String,
        session: Option<TransferSession>,
        session_id: u64
    },
    CancelSession(Option<String>, u64),   // KEEP
    FindPublicSession { alias: String },  // KEEP
    SubscribeToPublicSessionTransferProgress { // KEEP
        session_owner_user_id: u64,
        session_order_id: u64,
        password: Option<String>
    }
}
```

#### 3.6 `transfer.rs` - Delete:
```rust
AnswerSessionRequest { peer_id, session, session_id } // DELETE
```

---

### 4. Protocol Handlers

#### 4.1 `webrtc.rs` - Add methods to `WebRtc` struct:
```rust
impl WebRtc {
    pub async fn send_sessions_notification(
        &self,
        peer_id: String,
        sessions: Vec<TransferSession>,
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.send_sessions_notification(sessions).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn view_session_detail(
        &self,
        peer_id: String,
        order_id: u64,
        password: Option<String>,
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.request_session_detail(order_id, password).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn send_session_detail(
        &self,
        peer_id: String,
        request_id: String,
        session: TransferSession,
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.send_session_detail_response(request_id, Some(&session), None).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn send_session_detail_error(
        &self,
        peer_id: String,
        request_id: String,
        error: String,
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.send_session_detail_response(request_id, None, Some(error.into())).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn download_resource(
        &self,
        peer_id: String,
        session_order_id: u64,
        resource_order_id: u64,
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.request_resource_download(session_order_id, resource_order_id).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }

    pub async fn stream_resource_to_peer(
        &self,
        peer_id: String,
        resource: LocalResource,
    ) -> Result<(), WebRtcErrors> {
        let peer_id = PeerId(peer_id.parse()?);
        if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
            peer.stream_resource(resource).await
        } else {
            Err(WebRtcErrors::ConnectionNotFound(peer_id))
        }
    }
}
```

#### 4.2 `peer.rs` - Update `process_message_packet`:
```rust
pub async fn process_message_packet(&self, request_id: String, msg: Request) {
    match msg {
        Request::CancelRequest(request) => {
            self.transfers_context.cancel_transfer(request.session_id as u64).await;
        }

        // DELETE: Request::TransferRequest handling

        // ADD:
        Request::SessionsNotification(notification) => {
            for overview in notification.sessions {
                let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionOverview {
                    peer_id: self.peer.id().to_string(),
                    order_id: overview.order_id,
                    password_protected: overview.password_protected,
                });
                if let Some(core_request) = self.core_request() {
                    core_request.response(response).await;
                }
            }
        }

        Request::ViewSessionRequest(req) => {
            let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest {
                peer_id: self.peer.id().to_string(),
                order_id: req.order_id,
                password: req.password,
            });
            // Store request_id for response
            if let Some(core_request) = self.core_request() {
                core_request.response(response).await;
            }
        }

        Request::DownloadResourceRequest(req) => {
            let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest {
                peer_id: self.peer.id().to_string(),
                session_order_id: req.session_order_id,
                resource_order_id: req.resource_order_id,
            });
            if let Some(core_request) = self.core_request() {
                core_request.response(response).await;
            }
        }

        Request::FecFeedback(feedback) => {
            if let Some(feedback) = feedback.feedback {
                log::info!("Received FEC feedback: {:?}", feedback);
                let _ = self.transfer_feedback_sender.unbounded_send(feedback);
            };
        }
        _ => {}
    }
}
```

#### 4.3 `peer.rs` - Add new methods to `WebRtcPeer`:
```rust
impl WebRtcPeer {
    pub async fn send_sessions_notification(
        &self,
        sessions: Vec<TransferSession>,
    ) -> Result<(), WebRtcErrors> {
        todo!("Send SessionsNotificationMessage to peer")
    }

    pub async fn request_session_detail(
        &self,
        order_id: u64,
        password: Option<String>,
    ) -> Result<(), WebRtcErrors> {
        todo!("Send ViewSessionDetailRequest to peer")
    }

    pub async fn send_session_detail_response(
        &self,
        request_id: String,
        session: Option<&TransferSession>,
        error: Option<PeerErrorsMessage>,
    ) -> Result<(), WebRtcErrors> {
        todo!("Send ViewSessionDetailResponse to peer")
    }

    pub async fn request_resource_download(
        &self,
        session_order_id: u64,
        resource_order_id: u64,
    ) -> Result<(), WebRtcErrors> {
        todo!("Send DownloadResourceRequest to peer")
    }

    pub async fn stream_resource(
        &self,
        resource: LocalResource,
    ) -> Result<(), WebRtcErrors> {
        todo!("Stream resource data to peer using existing transfer protocol")
    }
}
```

#### 4.4 `peer.rs` - Delete from `process_message_packet`:
```rust
// DELETE this block:
Request::TransferRequest(request) => {
    self.transfers_context.start_transfer(request.session.order_id, request_id).await;
    let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionRequest {
        remote_session: request.session,
    });
    if let Some(core_request) = self.core_request() {
        core_request.response(response).await;
    }
}
```

---

### 5. Shell Handler (`shared/src/shell/`)

The Shell needs to handle the new P2POperations and route them to the WebRTC peer.

#### Update Shell executor to handle:
```rust
P2POperation::SendSessionsNotification { peer_id, sessions } => {
    let peer = find_peer(&peer_id);
    let overviews: Vec<P2PSessionOverviewMessage> = sessions
        .iter()
        .map(|s| P2PSessionOverviewMessage {
            order_id: s.order_id,
            password_protected: s.target.is_required_password(),
        })
        .collect();
    peer.send_sessions_notification(overviews).await
}

P2POperation::ViewSessionDetail { peer_id, order_id, password } => {
    let peer = find_peer(&peer_id);
    peer.request_session_detail(order_id, password).await
}

P2POperation::SendSessionDetail { peer_id, session } => {
    let peer = find_peer(&peer_id);
    let proto_session: P2PTransferSessionMessage = session.into();
    peer.send_session_detail_response(request_id, Some(proto_session), None).await
}

P2POperation::SendSessionDetailError { peer_id, order_id, error } => {
    let peer = find_peer(&peer_id);
    peer.send_session_detail_response(request_id, None, Some(error.into())).await
}

P2POperation::DownloadResource { peer_id, session_order_id, resource_order_id } => {
    let peer = find_peer(&peer_id);
    peer.request_resource_download(session_order_id, resource_order_id).await
}

P2POperation::StreamResourceToPeer { peer_id, resource } => {
    let peer = find_peer(&peer_id);
    // Use existing transfer protocol to stream resource data
    peer.stream_resource(resource).await
}

P2POperation::PrepareReceiveResource { peer_id, session_order_id, resource_order_id, save_path } => {
    // Create IOWriter at save_path
    // Register to receive data for this resource
    // Data will flow through existing transfer protocol
    let writer = IOWriter::create(save_path)?;
    register_resource_receiver(peer_id, resource_order_id, writer).await
}
```

#### Proto ↔ Rust Conversions:
```rust
impl From<&TransferSession> for P2PSessionOverviewMessage {
    fn from(session: &TransferSession) -> Self {
        let is_required_password = match &session.target {
            TransferTarget::P2P { is_required_password, .. } => *is_required_password,
            _ => false,
        };
        Self {
            order_id: session.order_id,
            password_protected: is_required_password,
        }
    }
}

impl From<&TransferSession> for P2PTransferSessionMessage {
    fn from(session: &TransferSession) -> Self {
        Self {
            order_id: session.order_id,
            resources: session.resources.iter().map(Into::into).collect(),
        }
    }
}

impl TransferSession {
    pub fn from_p2p_message(msg: P2PTransferSessionMessage, peer: Peer) -> Self {
        Self {
            order_id: msg.order_id,
            resources: msg.resources.into_iter().map(Into::into).collect(),
            progress: vec![],
            transfer_type: TransferType::Receive,
            target: TransferTarget::P2P {
                from_peer: peer,
                password: None,
                is_required_password: false,
            },
            cancellation_token: CancellationToken::new(),
        }
    }
}
```

---

## Key Implementation Details

### Password Validation
- ALWAYS validate on sender side (Core)
- Use constant-time comparison
- Rate limit attempts per peer

### Session States (Receiver)
Store session state separately in Core or use TransferSession.status:
- Overview received: Create TransferSession with empty resources
- Password required: Wait for user input
- Authenticated: Populate resources in TransferSession
- Failed: Update TransferSession.status

### Resource Transfer
- Each download creates independent `TransferSession`
- Uses existing transfer protocol (FEC, chunking)
- Multiple resources can download in parallel

### On Peer Connection (Sender)
1. Complete peer introduction handshake
2. After connection established, get all active transfer sessions (TransferSession with target = P2P)
3. Create P2PSessionOverviewMessage list from TransferSession (order_id + check if password_protected)
4. Send SessionsNotificationMessage with list of sessions

### On Session View Request (Sender)
1. Receive ViewSessionDetailRequest
2. Find TransferSession by order_id
3. If password protected, validate password
4. If valid: send P2PTransferSessionMessage with TransferSession resources
5. If invalid: send PeerErrorsMessage::InvalidPassword

### On Download Request (Sender)
1. Receive DownloadResourceRequest (session_order_id, resource_order_id)
2. Find session and resource
3. Core asks Shell to immediately stream resource data to peer
4. Shell uses existing transfer protocol (FEC, chunking)

### On Download Request (Receiver)
1. User clicks download on resource
2. Send DownloadResourceRequest
3. Immediately prepare IOWriter cursor at save_path
4. Wait for incoming data stream
5. Receive data and write to IOWriter

---

## Benefits

✅ **Bandwidth Efficient**: Send lightweight overviews, full details only when needed
✅ **Better UX**: See all sessions immediately, authenticate later
✅ **Selective Downloads**: Download individual files, not entire session
✅ **Security**: Password validated server-side before revealing resources
✅ **Scalable**: Add metadata to overviews without protocol changes
✅ **Resumable**: Each resource is independent transfer

---

## 6. App Events & Commands Updates

### 6.1 TransferEvent (in `app/transfer/module.rs`)

**Add new events:**
```rust
pub enum TransferEvent {
    NotifyPeerSessions {
        peer_id: String,
    },

    ReceivedSessionOverview {
        peer_id: String,
        order_id: u64,
        password_protected: bool,
    },

    ReceivedViewSessionRequest {
        peer_id: String,
        order_id: u64,
        password: Option<String>,
    },

    RequestSessionDetail {
        peer_id: String,
        order_id: u64,
        password: Option<String>,
    },

    SessionDetailReceived {
        session: TransferSession,
    },

    SessionDetailFailed {
        order_id: u64,
        error: String,
    },

    ReceivedDownloadRequest {
        peer_id: String,
        session_order_id: u64,
        resource_order_id: u64,
    },

    RequestDownloadResource {
        peer_id: String,
        session_order_id: u64,
        resource_order_id: u64,
        save_path: String,
    },
}
```

**Update existing:**
```rust
TransferRequest { ... }  // DEPRECATE - keep for migration, remove later
```

### 6.2 nearby/command.rs - `handle_peer_connection`

**Update to forward P2P events to Transfer module:**
```rust
pub async fn handle_peer_connection(&self, peer: Peer) {
    let request = P2POperation::PeerEvents(peer.id.clone());
    let mut stream = self.stream_from_shell(request.into());

    while let Some(output) = stream.next().await {
        match output {
            CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionOverview {
                peer_id,
                order_id,
                password_protected
            }) => {
                self.notify_event(TransferEvent::ReceivedSessionOverview {
                    peer_id,
                    order_id,
                    password_protected,
                });
            }

            CoreOperationOutput::P2P(P2POperationOutput::ReceivedViewSessionRequest {
                peer_id,
                order_id,
                password
            }) => {
                self.notify_event(TransferEvent::ReceivedViewSessionRequest {
                    peer_id,
                    order_id,
                    password,
                });
            }

            CoreOperationOutput::P2P(P2POperationOutput::SessionDetailReceived { session }) => {
                self.notify_event(TransferEvent::SessionDetailReceived { session });
            }

            CoreOperationOutput::P2P(P2POperationOutput::SessionDetailFailed { order_id, error }) => {
                self.notify_event(TransferEvent::SessionDetailFailed { order_id, error });
            }

            CoreOperationOutput::P2P(P2POperationOutput::ReceivedDownloadRequest {
                peer_id,
                session_order_id,
                resource_order_id,
            }) => {
                self.notify_event(TransferEvent::ReceivedDownloadRequest {
                    peer_id,
                    session_order_id,
                    resource_order_id,
                });
            }

            CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected()) => {
                log::info!("Peer disconnected: {}", peer.id);
                break;
            }

            _ => {}
        }
    }
}
```

**Call notify after peer connected in `start_nearby_server`:**
```rust
CoreOperationOutput::P2P(P2POperationOutput::PeerConnected(peer)) => {
    log::info!(target: "nearby", "New peer connected: {}", peer.id);

    self.notify_event(NearbyEvent::UpdateNearbyPeers {
        new_peer: vec![peer.clone()],
        removed: vec![]
    });

    self.notify_event(TransferEvent::UpdateTransferTargets {
        added: vec![TransferTarget::P2P { from_peer: peer.clone(), .. }],
        removed: vec![]
    });

    // NEW: Notify peer about our sessions
    self.notify_event(TransferEvent::NotifyPeerSessions {
        peer_id: peer.id.clone()
    });

    self.spawn(|it| async move {
        it.app().handle_peer_connection(peer).await;
    });
}
```

### 6.3 transfer/module.rs - Event Handlers

**Add handlers in `TransferModule::update()`:**
```rust
match event {
    TransferEvent::NotifyPeerSessions { peer_id } => {
        Command::handle_result(|it| async move {
            it.app().notify_peer_sessions(peer_id).await
        })
    }

    TransferEvent::ReceivedSessionOverview { peer_id, order_id, password_protected } => {
        Command::handle_result(|it| async move {
            it.app().handle_session_overview(peer_id, order_id, password_protected).await
        })
    }

    TransferEvent::ReceivedViewSessionRequest { peer_id, order_id, password } => {
        Command::handle_result(|it| async move {
            it.app().handle_view_session_request(peer_id, order_id, password).await
        })
    }

    TransferEvent::RequestSessionDetail { peer_id, order_id, password } => {
        Command::handle_result(|it| async move {
            it.app().request_session_detail(peer_id, order_id, password).await
        })
    }

    TransferEvent::SessionDetailReceived { session } => {
        Command::handle_result(|it| async move {
            it.app().handle_session_detail_received(session).await
        })
    }

    TransferEvent::SessionDetailFailed { order_id, error } => {
        Command::new(|it| async move {
            DialogOperation::toast(format!("Failed to load session: {}", error))
                .into_future(it.clone()).await;
        })
    }

    TransferEvent::ReceivedDownloadRequest { peer_id, session_order_id, resource_order_id } => {
        Command::handle_result(|it| async move {
            it.app().handle_download_request(peer_id, session_order_id, resource_order_id).await
        })
    }

    TransferEvent::RequestDownloadResource { peer_id, session_order_id, resource_order_id, save_path } => {
        Command::handle_result(|it| async move {
            it.app().request_download_resource(peer_id, session_order_id, resource_order_id, save_path).await
        })
    }
}
```

### 6.4 transfer/commands.rs - New Commands

**Add (with access to model via TransferEvent):**
```rust
impl AppCommand {
    pub async fn notify_peer_sessions(&self, peer_id: String) -> Result<(), CoreError> {
        // Get all active P2P send sessions from model
        let sessions: Vec<TransferSession> = /* access model.transfer.sessions */
            .iter()
            .filter(|it| it.transfer_type == TransferType::Send)
            .filter(|it| matches!(it.target, TransferTarget::P2P { .. }))
            .cloned()
            .collect();

        // Send sessions notification via P2P
        self.run(P2POperation::SendSessionsNotification {
            peer_id,
            sessions,
        }).await
    }

    pub async fn handle_session_overview(
        &self,
        peer_id: String,
        order_id: u64,
        password_protected: bool,
    ) -> Result<(), CoreError> {
        // Find peer from model
        let peer = /* get peer from model.nearby.peers by peer_id */;

        // Create stub TransferSession with no resources
        let stub_session = TransferSession {
            order_id,
            resources: vec![],
            progress: vec![],
            transfer_type: TransferType::Receive,
            target: TransferTarget::P2P {
                from_peer: peer,
                url: String::new(),
            },
            cancellation_token: CancellationToken::new(),
        };

        // Add to model
        self.update_model(TransferSessionModelEvent::Add(stub_session.clone()));

        // Save to persistence
        let _ = self.run(TransferSessionPersistentOperation::save(stub_session)).await;

        Ok(())
    }

    pub async fn handle_view_session_request(
        &self,
        peer_id: String,
        order_id: u64,
        password: Option<String>,
    ) -> Result<(), CoreError> {
        // Sender side: find session from model
        let session = /* find in model.transfer.sessions by order_id */;

        // Validate password if needed
        let is_password_valid = match &session.target {
            TransferTarget::P2P { .. } => {
                // Check if session has password
                // Validate password
                true
            }
            _ => false,
        };

        if !is_password_valid {
            // Send error response
            self.run(P2POperation::SendSessionDetailError {
                peer_id,
                order_id,
                error: "Invalid password".to_string(),
            }).await?;
            return Ok(());
        }

        // Send full session via P2P
        self.run(P2POperation::SendSessionDetail {
            peer_id,
            session,
        }).await
    }

    pub async fn request_session_detail(
        &self,
        peer_id: String,
        order_id: u64,
        password: Option<String>,
    ) -> Result<(), CoreError> {
        // Receiver side: send request via P2P
        self.run(P2POperation::ViewSessionDetail {
            peer_id,
            order_id,
            password,
        }).await
    }

    pub async fn handle_session_detail_received(
        &self,
        session: TransferSession,
    ) -> Result<(), CoreError> {
        // Update existing stub session with full details
        self.update_model(TransferSessionModelEvent::Update(
            session.id(),
            session.clone().into(),
        ));

        // Save to persistence
        let _ = self.run(TransferSessionPersistentOperation::save(session)).await;

        Ok(())
    }

    pub async fn handle_download_request(
        &self,
        peer_id: String,
        session_order_id: u64,
        resource_order_id: u64,
    ) -> Result<(), CoreError> {
        // Sender side: find session and resource from model
        let session = /* find in model.transfer.sessions by session_order_id */;
        let resource = /* find in session.resources by resource_order_id */;

        // Immediately ask Shell to stream resource data to peer
        // No response needed - data starts flowing immediately
        self.run(P2POperation::StreamResourceToPeer {
            peer_id,
            resource: resource.clone(),
        }).await
    }

    pub async fn request_download_resource(
        &self,
        peer_id: String,
        session_order_id: u64,
        resource_order_id: u64,
        save_path: String,
    ) -> Result<(), CoreError> {
        // Receiver side:
        // 1. Send download request via P2P
        self.run(P2POperation::DownloadResource {
            peer_id: peer_id.clone(),
            session_order_id,
            resource_order_id,
        }).await?;

        // 2. Immediately prepare IOWriter and wait for data stream
        // 3. Data will flow through existing transfer protocol
        self.run(P2POperation::PrepareReceiveResource {
            peer_id,
            session_order_id,
            resource_order_id,
            save_path,
        }).await
    }
}
```

### 6.5 View Models Updates

**ReceiveSessionViewModel - Add fields:**
```rust
pub struct ReceiveSessionViewModel {
    pub id: String,
    pub peer_avatar: AvatarViewModel,
    pub peer_name: String,
    pub peer_description: String,

    pub password_required: bool,
    pub is_authenticated: bool,
    pub has_details: bool,

    pub image_resources: Vec<ImageReceiveResourceViewModel>,
    pub video_resources: Vec<VideoReceiveResourceViewModel>,
    pub file_resources: Vec<FileReceiveResourceViewModel>,
    pub is_completed: bool,
    pub is_in_progress: bool,
    pub display_download_speed: String,
    pub progress: f64,
    pub display_datetime: String,
}
```

**Update TransferModule::view():**
```rust
received_sessions: model
    .transfer
    .sessions
    .iter()
    .filter(|it| it.transfer_type == TransferType::Receive)
    .filter_map(|it| {
        let Some(peer) = it.peer() else {
            return None;
        };

        let has_details = !it.resources.is_empty();
        let password_required = /* check if password protected */;
        let is_authenticated = has_details;

        Some(ReceiveSessionViewModel {
            id: it.order_id.to_string(),
            peer_avatar: AvatarViewModel::new(peer.avatar_url.clone()),
            peer_name: peer.name.clone().unwrap_or(peer.device.name.clone()),
            peer_description: "Nearby".to_owned(),

            password_required,
            is_authenticated,
            has_details,

            // ... rest of fields
        })
    })
    .collect()
```

---

## Migration Steps

### Phase 1: Proto & Bindings
1. Update proto schemas (session.proto, request.proto)
2. Generate Rust proto bindings

### Phase 2: Entities
3. Update `TransferTarget::P2P` to add password fields
4. Fix `Nearby` → `P2P` references in:
   - `entities/transfer_session.rs`
   - `app/nearby/command.rs`
   - `repository/transfer_session.rs`

### Phase 3: Operations
5. Add new `P2POperation` variants
6. Add new `P2POperationOutput` variants
7. Remove `ReceivedSessionRequest`

### Phase 4: Protocol & Shell
8. Update `protocol/webrtc/peer.rs` - add new message handlers
9. Update Shell executor to handle new P2POperations
10. Add proto ↔ Rust conversion functions

### Phase 5: App Layer
11. Add new `TransferEvent` variants
12. Update `nearby/command.rs`:
    - Add `NotifyPeerSessions` call in `start_nearby_server`
    - Update `handle_peer_connection` to forward new events
13. Add event handlers in `transfer/module.rs`
14. Add command functions in `transfer/commands.rs`

### Phase 6: View
15. Update `ReceiveSessionViewModel` with auth state fields
16. Update `TransferModule::view()` to populate new fields

### Phase 7: Cleanup
17. Remove old `TransferRequest` handling
18. Remove `TransferRequestMessage` / `TransferResponseMessage` from proto

### Phase 8: Testing
19. Test session discovery (overviews sent on connect)
20. Test password authentication flow
21. Test selective resource downloads
22. Test peer disconnect cleanup
