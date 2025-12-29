# Updated P2P Transfer Session Password-Protected Flow

## Overview
Update the P2P transfer session flow to support password-protected sessions with a two-phase authentication approach. The receiver initially requests session metadata without a password, and if password-protected, prompts for password before receiving full session details and resources.

## Current Implementation Analysis

### Current Flow
1. **Receiver** sends `ViewSessionDetailRequest` with `order_id` + optional `password`
2. **Sender** validates password in `handle_view_session_request` (shared/src/app/transfer/commands.rs:248)
3. **Sender** responds with `ViewSessionDetailResponse` containing `P2pTransferSessionMessage`
4. **Sender** then streams all resources via `ResourceNotificationRequest`
5. **Receiver** updates session using `UpdateAction` pattern (shared/src/entities/transfer_session.rs:553)

### Current Schema (libs/schema/proto/devlog/bitbridge/)

**request.proto:**
```protobuf
message ViewSessionDetailRequest {
    required uint64 order_id = 1;
    optional string password = 2;
}

message ViewSessionDetailResponse {
    oneof result {
        P2PTransferSessionMessage session = 1;
        ResourceMessage resource_updated = 2;
        PeerErrorsMessage error = 3;
    }
}
```

**session.proto:**
```protobuf
message P2PTransferSessionMessage {
  required uint64 order_id = 1;
  optional string description = 2;
}
```

**Existing UpdateAction:**
```rust
impl UpdateAction<TransferSession> for schema::devlog::bitbridge::P2pTransferSessionMessage {
    fn update(self, data: &mut TransferSession) {
        data.description = self.description;
    }
}
```

---

## Implementation Plan

### Phase 1: Schema Updates

#### File: `libs/schema/proto/devlog/bitbridge/session.proto`
**Update `P2PTransferSessionMessage` to include password protection flag:**
```protobuf
message P2PTransferSessionMessage {
  required uint64 order_id = 1;
  optional string description = 2;
  required bool password_protected = 3;  // NEW: Indicates if session requires password
}
```

---

### Phase 2: Sender-Side Implementation

#### File: `shared/src/app/transfer/commands.rs`

**Update `handle_view_session_request` (line 248-285):**

Current logic:
- Validates password if `session.is_required_password` is true
- Returns early if invalid
- Sends full session detail with all resources

New logic:
```rust
pub async fn handle_view_session_request(
    &self,
    peer_id: String,
    request_id: String,
    password: Option<String>,
    session: Option<TransferSession>,
    device_info: Option<DeviceInfo>
) -> Result<(), CoreError> {
    let Some(mut session) = session else {
        log::warn!("Failed to load session detail: session not found");
        // Send error response
        self.run(P2POperation::send_session_detail_error(
            peer_id,
            request_id,
            CoreError::PeerRequestError(PeerErrorsMessage::SessionNotFound)
        )).await?;
        return Ok(());
    };

    session.description = device_info.map(|it| format!("{} - {}", it.name, it.platform.as_str_name()));

    // NEW: Two-phase authentication logic
    if session.is_required_password {
        match (&session.password, &password) {
            (Some(expected), Some(provided)) if expected == provided => {
                // Password correct: Send full session with resources
                let proto_session = P2pTransferSessionMessage {
                    order_id: session.order_id,
                    description: session.description.clone(),
                    password_protected: true,
                };

                self.run(P2POperation::send_session_detail(
                    peer_id,
                    request_id,
                    Some(proto_session),
                    Some(session.resources)  // Send resources
                )).await?;
            }
            (Some(_), None) => {
                // No password provided: Send metadata only (password_protected=true, no resources)
                let proto_session = P2pTransferSessionMessage {
                    order_id: session.order_id,
                    description: session.description.clone(),
                    password_protected: true,
                };

                self.run(P2POperation::send_session_detail(
                    peer_id,
                    request_id,
                    Some(proto_session),
                    None  // No resources
                )).await?;
            }
            (Some(_), Some(_)) => {
                // Password incorrect
                log::warn!("Invalid password for session {}", session.order_id);
                self.run(P2POperation::send_session_detail_error(
                    peer_id,
                    request_id,
                    CoreError::PeerRequestError(PeerErrorsMessage::InvalidPassword)
                )).await?;
            }
            (None, _) => {
                // No password required but flag is set (shouldn't happen)
                let proto_session = P2pTransferSessionMessage {
                    order_id: session.order_id,
                    description: session.description.clone(),
                    password_protected: false,
                };

                self.run(P2POperation::send_session_detail(
                    peer_id,
                    request_id,
                    Some(proto_session),
                    Some(session.resources)
                )).await?;
            }
        }
    } else {
        // Not password protected: Send full session
        let proto_session = P2pTransferSessionMessage {
            order_id: session.order_id,
            description: session.description.clone(),
            password_protected: false,
        };

        self.run(P2POperation::send_session_detail(
            peer_id,
            request_id,
            Some(proto_session),
            Some(session.resources)
        )).await?;
    }

    Ok(())
}
```

#### File: `shared/src/app/operations/p2p.rs`

**Update `P2POperation::SendSessionDetail` variant (line 28-33):**

Current:
```rust
SendSessionDetail {
    peer_id: String,
    request_id: String,
    session: Option<TransferSession>,
    error: Option<CoreError>
}
```

New:
```rust
SendSessionDetail {
    peer_id: String,
    request_id: String,
    session_message: Option<P2pTransferSessionMessage>,  // NEW: Direct proto message
    resources: Option<Vec<LocalResource>>,                // NEW: Optional resources
    error: Option<CoreError>
}
```

**Update helper methods:**
```rust
pub fn send_session_detail(
    peer_id: String,
    request_id: String,
    session_message: Option<P2pTransferSessionMessage>,
    resources: Option<Vec<LocalResource>>
) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
    Command::request_from_shell(CoreOperation::P2P(P2POperation::SendSessionDetail {
        peer_id,
        request_id,
        session_message,
        resources,
        error: None
    })).map(|it| it.result())
}

pub fn send_session_detail_error(
    peer_id: String,
    request_id: String,
    error: CoreError
) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
    Command::request_from_shell(CoreOperation::P2P(P2POperation::SendSessionDetail {
        peer_id,
        request_id,
        session_message: None,
        resources: None,
        error: Some(error)
    })).map(|it| it.result())
}
```

#### File: `shared/src/shell/executor/p2p.rs`

**Update handler for `SendSessionDetail` (line 48-50):**

Current:
```rust
P2POperation::SendSessionDetail { peer_id, request_id, session, error } => {
    self.web_rtc().send_session_detail(peer_id, request_id, session, error).await?;
    Ok(CoreOperationOutput::None)
}
```

New:
```rust
P2POperation::SendSessionDetail { peer_id, request_id, session_message, resources, error } => {
    self.web_rtc().send_session_detail(peer_id, request_id, session_message, resources, error).await?;
    Ok(CoreOperationOutput::None)
}
```

#### File: `shared/src/protocol/webrtc/webrtc.rs`

**Update `send_session_detail` signature (line 123-136):**

Current:
```rust
pub async fn send_session_detail(
    &self,
    peer_id: String,
    request_id: String,
    session: Option<TransferSession>,
    error: Option<CoreError>,
) -> Result<(), WebRtcErrors>
```

New:
```rust
pub async fn send_session_detail(
    &self,
    peer_id: String,
    request_id: String,
    session_message: Option<P2pTransferSessionMessage>,
    resources: Option<Vec<LocalResource>>,
    error: Option<CoreError>,
) -> Result<(), WebRtcErrors> {
    let peer_id = PeerId(peer_id.parse()?);
    if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
        peer.send_session_detail_response(request_id, session_message, resources, error).await
    } else {
        Err(WebRtcErrors::ConnectionNotFound(peer_id))
    }
}
```

#### File: `shared/src/protocol/webrtc/peer.rs`

**Update `send_session_detail_response` (line 508-560):**

Current signature:
```rust
pub async fn send_session_detail_response(
    &self,
    request_id: String,
    session: Option<TransferSession>,
    error: Option<CoreError>,
) -> Result<(), WebRtcErrors>
```

New implementation:
```rust
pub async fn send_session_detail_response(
    &self,
    request_id: String,
    session_message: Option<P2pTransferSessionMessage>,  // NEW: Direct proto message
    resources: Option<Vec<LocalResource>>,                // NEW: Optional resources
    error: Option<CoreError>,
) -> Result<(), WebRtcErrors> {
    log::info!("Sending session detail response");
    use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

    if let Some(error_msg) = error {
        log::error!("Failed to send session detail response: {:?}", error_msg);
        match error_msg {
            CoreError::PeerRequestError(e) => {
                self.msg_channel.send_response(
                    request_id,
                    Response::ViewSessionResponse(ViewSessionDetailResponse {
                        result: Some(ResponseResult::Error(e.into()))
                    })
                ).await?;
            }
            e => {
                log::error!("Failed to send session detail response: {:?}", e);
                self.msg_channel.send_response(
                    request_id,
                    Response::ViewSessionResponse(ViewSessionDetailResponse {
                        result: Some(ResponseResult::Error(PeerErrorsMessage::InvalidRequest.into()))
                    })
                ).await?;
            }
        }
        return Ok(())
    }

    let Some(proto_session) = session_message else {
        return Ok(())
    };

    log::info!(
        "Sending session detail: order_id={}, password_protected={}, has_resources={}",
        proto_session.order_id,
        proto_session.password_protected,
        resources.is_some()
    );

    // Send the proto message
    let response = ViewSessionDetailResponse {
        result: Some(ResponseResult::Session(proto_session))
    };

    self.msg_channel.send_response(request_id, Response::ViewSessionResponse(response)).await?;

    sleep(Duration::from_millis(100)).await;

    // NEW: Only send resources if provided (authenticated or not password-protected)
    if let Some(resources) = resources {
        if !resources.is_empty() {
            let session_order_id = proto_session.order_id;
            for resource in resources {
                self.send_resource_notification(session_order_id, resource).await?;
                sleep(Duration::from_millis(50)).await;
            }
        }
    } else {
        log::info!(
            "No resources to send for session {} (password-protected, awaiting auth)",
            proto_session.order_id
        );
    }

    Ok(())
}
```

---

### Phase 3: Receiver-Side Implementation

#### File: `shared/src/entities/transfer_session.rs`

**Update existing `UpdateAction` implementation (line 553-557):**

Current:
```rust
impl UpdateAction<TransferSession> for schema::devlog::bitbridge::P2pTransferSessionMessage {
    fn update(self, data: &mut TransferSession) {
        data.description = self.description;
    }
}
```

New:
```rust
impl UpdateAction<TransferSession> for schema::devlog::bitbridge::P2pTransferSessionMessage {
    fn update(self, data: &mut TransferSession) {
        data.description = self.description;
        data.is_required_password = self.password_protected;  // NEW: Update password flag

        log::info!(
            "Updated session {} with description={:?}, password_protected={}",
            data.order_id,
            self.description,
            self.password_protected
        );
    }
}
```

#### File: `shared/src/protocol/webrtc/peer.rs`

**Update `request_session_detail` (line 454-506):**

Current logic:
- Sends request with password
- Receives `P2pTransferSessionMessage`
- Emits `SessionDetailReceived` with proto message

Keep the same but ensure we're emitting the proto message correctly:
```rust
pub async fn request_session_detail(
    &self,
    core_request: CoreRequest,
    order_id: u64,
    password: Option<String>,
) -> Result<(), WebRtcErrors> {
    use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

    log::info!(
        "Requesting session detail for order_id {} (password: {})",
        order_id,
        if password.is_some() { "provided" } else { "not provided" }
    );

    let request = ViewSessionDetailRequest {
        order_id,
        password,
    };

    let timeout_token = CancellationToken::timeout(Duration::from_secs(6));

    let response_result = self.msg_channel.send(Request::ViewSessionRequest(request), None)
        .with_cancel(&timeout_token)
        .await
        .map_err(|_| {
            log::error!("Timeout waiting for session detail response");
            WebRtcErrors::Timeout
        })??;

    match response_result {
        Response::ViewSessionResponse(resp) => {
            match resp.result {
                Some(ResponseResult::Session(proto_session)) => {
                    // Send the proto message directly for UpdateAction
                    core_request.response(CoreOperationOutput::Transfer(
                        TransferOperationOutput::SessionDetailReceived(proto_session)
                    )).await;
                }
                Some(ResponseResult::Error(error_type)) => {
                    let error_msg = PeerErrorsMessage::try_from(error_type)
                        .unwrap_or(PeerErrorsMessage::InvalidRequest);
                    core_request.response(CoreOperationOutput::Error(
                        CoreError::PeerRequestError(error_msg)
                    )).await;
                    return Err(WebRtcErrors::PeerError(error_msg.to_string()));
                }
                _ => {
                    return Err(WebRtcErrors::InvalidResponse("Unexpected response".to_string()));
                }
            }
        }
        _ => {
            return Err(WebRtcErrors::InvalidResponse("Expected ViewSessionResponse".to_string()));
        }
    }

    Ok(())
}
```

#### File: `shared/src/app/operations/transfer.rs`

**Keep `SessionDetailReceived` using proto message:**
```rust
pub enum TransferOperationOutput {
    // ... existing variants ...
    SessionDetailReceived(schema::devlog::bitbridge::P2pTransferSessionMessage),
    // ... other variants ...
}
```

#### File: `shared/src/app/transfer/commands.rs`

**Update `request_session_detail` (line 287-313):**

Current logic:
- Streams responses
- Updates session with description when received

New logic using `UpdateAction`:
```rust
pub async fn request_session_detail(
    &self,
    peer_id: String,
    session_id: TransferSessionId,
    order_id: u64,
    password: Option<String>
) -> Result<(), CoreError> {
    let mut stream = self.stream_from_shell(P2POperation::ViewSessionDetail { peer_id, order_id, password }.into());

    while let Some(output) = stream.next().await {
        match output {
            CoreOperationOutput::Transfer(TransferOperationOutput::SessionDetailReceived(proto_session)) => {
                log::info!(
                    "Received session detail for order_id {}: description={:?}, password_protected={}",
                    proto_session.order_id,
                    proto_session.description,
                    proto_session.password_protected
                );

                // NEW: Use UpdateAction pattern directly with proto message
                self.update_model(TransferSessionModelEvent::Update(
                    session_id.clone(),
                    proto_session.into()  // proto_session implements UpdateAction
                ));
                break;
            }
            CoreOperationOutput::Error(e) => {
                log::error!("Error receiving session detail: {:?}", e);
                return Err(e);
            }
            _ => continue
        }
    }

    Ok(())
}
```

---

### Phase 4: UI/Module Integration

#### File: `shared/src/app/transfer/module.rs`

**Update `ViewSession` event handler (line 379-426):**

Current behavior:
- Checks if should_request based on connection state
- Requests session detail with password if provided

New behavior:
```rust
TransferEvent::ViewSession { password, session_id, .. } => {
    let session_id = TransferSessionId {
        order_id: Some(session_id.to_string()),
        transfer_type: Some(TransferType::Receive)
    };

    let Some(session) = model.transfer.sessions.lookup(&session_id).cloned() else {
        return Command::done()
    };

    match &session.target {
        TransferTarget::P2P { connection_state, signalling_key, from_peer, .. } => {
            // Determine if we should request session detail
            let should_request = match connection_state {
                P2PConnectionState::NotConnected |
                P2PConnectionState::Failed(_) => true,
                P2PConnectionState::Connected => {
                    session.resources.is_empty()
                },
                P2PConnectionState::Connecting => false,
            };

            if !should_request {
                return Command::done();
            }

            // Ensure we're in the finding scope
            if from_peer.is_none() {
                let scope = FindingScope::new(signalling_key);
                if !model.nearby.finding_scopes.contains(&scope) {
                    log::info!("Adding scope {} for session {}", signalling_key, session.order_id);
                    return Command::event(AppEvent::Nearby(NearbyEvent::AddFindingScope(scope)));
                }
                return Command::done();
            }

            let peer_id = from_peer.as_ref().unwrap().id().to_string();
            let session_id_clone = session_id.clone();

            Command::handle_result(move |it| async move {
                it.app().request_session_detail(peer_id, session_id_clone, session.order_id, password).await
            })
        }
        TransferTarget::Internet { .. } => {
            Command::handle_result(|it| async move {
                it.app().view_public_session(session, password).await
            })
        }
    }
}
```

---

## Testing Strategy

### Test Case 1: Non-Password-Protected Session
1. **Sender** creates P2P session without password
2. **Receiver** finds session and views it (no password)
3. **Expected**:
   - Receiver receives `P2pTransferSessionMessage` with `password_protected=false`
   - Resources are streamed immediately
   - Session updates via `UpdateAction`

### Test Case 2: Password-Protected Session - Initial View
1. **Sender** creates P2P session with password "test123"
2. **Receiver** finds session and views it (no password provided initially)
3. **Expected**:
   - Receiver receives `P2pTransferSessionMessage` with `password_protected=true`
   - No resources sent
   - Session updates `is_required_password=true` via `UpdateAction`
   - UI shows password input field

### Test Case 3: Password-Protected Session - Correct Password
1. **Receiver** enters correct password "test123" and views session
2. **Expected**:
   - Receiver receives `P2pTransferSessionMessage` with `password_protected=true`
   - Resources are streamed via `ResourceNotificationRequest`
   - UI updates to show resources

### Test Case 4: Password-Protected Session - Incorrect Password
1. **Receiver** enters incorrect password "wrong" and views session
2. **Expected**:
   - Receiver gets `InvalidPassword` error response
   - UI shows error message
   - Session remains in metadata-only state

### Test Case 5: Re-authentication
1. **Receiver** has metadata for password-protected session
2. **Receiver** enters password (wrong first, then correct)
3. **Expected**:
   - First attempt fails with error
   - Second attempt succeeds and loads resources
   - UI updates progressively

---

## Key Implementation Details

### 1. **Single Function for Both Phases**
Both authentication phases use the same `handle_view_session_request` function. The logic creates appropriate `P2pTransferSessionMessage` based on:
- `session.is_required_password` flag
- Presence/absence of `password` parameter
- Password match validation

### 2. **Direct Proto Message**
No `clone_metadata_only` helper needed. Instead:
- Create `P2pTransferSessionMessage` directly with appropriate fields
- Pass `None` for resources when not authenticated
- Pass `Some(resources)` when authenticated or not password-protected

### 3. **UpdateAction Pattern**
Follows existing codebase pattern:
- `P2pTransferSessionMessage` implements `UpdateAction<TransferSession>`
- Updates both `description` and `is_required_password` fields
- Receiver uses `TransferSessionModelEvent::Update` with proto message

### 4. **Resource Streaming**
Resources are only streamed when:
- `resources` parameter is `Some(vec)` in `send_session_detail_response`
- This happens when session is NOT password-protected OR password is correct

### 5. **State Management**
Session state transitions via `UpdateAction`:
1. **Initial**: `resources=[]`, `is_required_password=false`
2. **Metadata Received**: `is_required_password=true` set by `UpdateAction`, `resources` still `[]`
3. **Authenticated**: Resources added via `ResourceNotificationRequest` events

---

## Files Modified Summary

### Schema Changes (Proto)
- `libs/schema/proto/devlog/bitbridge/session.proto` - Add `password_protected` to `P2PTransferSessionMessage`

### Rust Implementation
- `shared/src/entities/transfer_session.rs` - Update `UpdateAction` for `P2pTransferSessionMessage`
- `shared/src/app/transfer/commands.rs` - Update `handle_view_session_request` and `request_session_detail`
- `shared/src/app/operations/p2p.rs` - Update `SendSessionDetail` operation signature
- `shared/src/shell/executor/p2p.rs` - Update executor for new signature
- `shared/src/protocol/webrtc/webrtc.rs` - Update `send_session_detail` signature
- `shared/src/protocol/webrtc/peer.rs` - Update `send_session_detail_response` signature
- `shared/src/app/operations/transfer.rs` - Keep `SessionDetailReceived` with proto message
- `shared/src/app/transfer/module.rs` - Update `ViewSession` event handling

---

## Migration Notes

- **Backward Compatibility**: Existing sessions without password continue to work unchanged
- **Proto Compatibility**: New `password_protected` field is `required` in proto, needs careful rollout
- **Client Updates**: UI must handle `is_required_password` flag to show password input when needed

---

## Benefits of This Approach

1. ✅ **Single Function**: Both phases handled in `handle_view_session_request`
2. ✅ **Follows Existing Pattern**: Uses `UpdateAction` like other session updates
3. ✅ **No Helper Methods**: Direct proto message creation, cleaner code
4. ✅ **Type Safe**: Proto message enforces structure at compile time
5. ✅ **Secure**: Password validated server-side, never sent unnecessarily
6. ✅ **Clear UX**: Distinct states (metadata vs. full session) with clear UI feedback
7. ✅ **Efficient**: Avoids streaming resources unnecessarily before authentication
8. ✅ **Maintainable**: Leverages existing message types and flow patterns
