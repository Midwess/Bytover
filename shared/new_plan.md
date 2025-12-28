# P2P Session Connection State Management - Implementation Plan

## Overview
This plan implements robust connection state management for P2P transfer sessions, ensuring users have clear visibility into connection status and proper timeout handling for failed connections.

## Architecture Changes

### 1. Add Connection State to P2P Target
**File:** `shared/src/entities/target.rs`

**Changes:**
- Add new `P2PConnectionState` enum with variants:
  - `NotConnected` - Initial state, no peer connected
  - `Connecting` - Actively requesting session details
  - `Connected` - Peer connected and session details received
  - `Failed(String)` - Connection failed with error message

- Update `TransferTarget::P2P` variant to include:
  ```rust
  P2P {
      from_peer: Option<Peer>,
      signalling_key: String,
      scope: String,
      connection_state: P2PConnectionState,  // NEW FIELD
  }
  ```

- Add helper methods:
  - `is_connected() -> bool`
  - `is_connecting() -> bool`
  - `is_failed() -> bool`
  - `connection_state() -> &P2PConnectionState`
  - `set_connection_state(state: P2PConnectionState)`

**Impact:** This provides clear separation between peer presence and connection state.

---

### 2. Update TransferSession to Handle Connection States
**File:** `shared/src/entities/transfer_session.rs`

**Changes:**
- Update `TransferSession::p2p()` constructor to initialize connection state as `NotConnected`
- Update `owner_connected()` method to:
  ```rust
  pub fn owner_connected(&mut self, peer: Peer) {
      if let TransferTarget::P2P { from_peer, connection_state, .. } = &mut self.target {
          from_peer.replace(peer);
          *connection_state = P2PConnectionState::Connected;
      }
  }
  ```

- Update `owner_disconnected()` method to:
  ```rust
  pub fn owner_disconnected(&mut self) {
      if let TransferTarget::P2P { from_peer, connection_state, .. } = &mut self.target {
          from_peer.take();
          *connection_state = P2PConnectionState::NotConnected;
      }
  }
  ```

- Add new methods:
  ```rust
  pub fn set_connecting(&mut self)  // Sets state to Connecting
  pub fn set_connection_failed(&mut self, error: String)  // Sets Failed state
  pub fn is_p2p_connected(&self) -> bool  // Checks if P2P is connected
  pub fn connection_message(&self) -> Option<String>  // Returns status message

  /// Add a resource to this session, checking peer ownership first
  pub fn add_resource_from_peer(&mut self, resource: LocalResource, peer: &Peer) -> bool {
      // Only add resource if this is a P2P session and peer is the owner
      if !peer.is_owned(self) {
          log::warn!("Peer {} is not owner of session {}, ignoring resource",
                     peer.id(), self.order_id);
          return false;
      }

      // Add the resource using existing logic
      self.add_resource(resource);
      true
  }
  ```

- Update `status()` method to check P2P connection state:
  ```rust
  pub fn status(&self) -> TransferSessionStatus {
      // For P2P sessions, check connection state first
      if let TransferTarget::P2P { connection_state, .. } = &self.target {
          match connection_state {
              P2PConnectionState::NotConnected => {
                  return TransferSessionStatus::Initializing;
              }
              P2PConnectionState::Connecting => {
                  return TransferSessionStatus::Initializing;
              }
              P2PConnectionState::Failed(msg) => {
                  return TransferSessionStatus::Failed(msg.clone());
              }
              P2PConnectionState::Connected => {
                  // Continue with normal status logic
              }
          }
      }

      // ... rest of existing status logic
  }
  ```

**Impact:** Session status now properly reflects P2P connection state, providing accurate feedback to users.

---

### 3. Update Transfer Module Peer Event Handling
**File:** `shared/src/app/transfer/module.rs`

**Changes:**
- Refactor `TransferEvent::PeerUpdated` handler to use new helper methods:
  ```rust
  TransferEvent::PeerUpdated { peer } => {
      for session in model.transfer.sessions.iter_mut() {
          if session.transfer_type != TransferType::Receive {
              continue;
          }

          // Use new helper methods from transfer_session.rs
          if peer.is_owned(session) {
              let was_disconnected = session.peer().is_none();
              session.owner_connected(peer.clone());

              if was_disconnected {
                  // Trigger session detail request
                  return Command::event(TransferEvent::RequestSessionDetail {
                      peer_id: peer.id,
                      order_id: session.order_id,
                      password: None
                  });
              }
          } else if peer.is_member(session) {
              // Handle member case if needed
          }
      }

      Command::render()
  }
  ```

- Update `TransferEvent::PeerDisconnected` to call `owner_disconnected()`:
  ```rust
  TransferEvent::PeerDisconnected { peer_id } => {
      for session in model.transfer.sessions.iter_mut() {
          if let Some(peer) = session.peer() {
              if peer.id == peer_id {
                  session.owner_disconnected();
                  break;
              }
          }
      }
      Command::render()
  }
  ```

**Impact:** Cleaner separation of concerns, peer connection logic centralized in entities.

---

### 4. Add View Detail Trigger Logic with State Check
**File:** `shared/src/app/transfer/module.rs`

**Changes:**
- Update `TransferEvent::ViewSession` handler:
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
          TransferTarget::P2P { connection_state, from_peer, signalling_key, .. } => {
              // Only trigger view detail if NOT connected or FAILED
              let should_request = match connection_state {
                  P2PConnectionState::NotConnected | P2PConnectionState::Failed(_) => true,
                  P2PConnectionState::Connected => session.resources.is_empty(),
                  P2PConnectionState::Connecting => false,  // Already in progress
              };

              if !should_request {
                  return Command::done();
              }

              // Ensure we're in the finding scope
              if from_peer.is_none() {
                  let scope = FindingScope::new(signalling_key);
                  if !model.nearby.finding_scopes.contains(&scope) {
                      return Command::event(NearbyEvent::AddFindingScope(scope));
                  }
                  // Return early - peer not connected yet
                  return Command::done();
              }

              let peer_id = from_peer.as_ref().unwrap().id().to_string();

              Command::handle_result(move |it| async move {
                  it.app().request_session_detail(peer_id, session.order_id, password).await
              })
          }
          // ... existing Internet handling
      }
  }
  ```

**Impact:** Prevents duplicate requests and provides better user experience.

---

### 5. Add Timeout Handling to WebRTC Peer
**File:** `shared/src/protocol/webrtc/peer.rs`

**Changes:**
- Update `request_session_detail()` to use notify pattern instead of streaming, with 6-second timeout only for session metadata:
  ```rust
  pub async fn request_session_detail(
      &self,
      core_request: CoreRequest,
      order_id: u64,
      password: Option<String>,
  ) -> Result<(), WebRtcErrors> {
      use core_services::utils::cancellation::FutureExtension;

      log::info!("Requesting session detail for order_id {}", order_id);

      let request = ViewSessionDetailRequest {
          order_id,
          password,
      };

      // Create timeout token (6 seconds) - only for session metadata
      let timeout_token = CancellationToken::timeout(Duration::from_secs(6));

      // Send request and wait for session response with timeout
      use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;
      let response = self.msg_channel.send(Request::ViewSessionRequest(request), None)
          .with_cancel(&timeout_token)
          .await
          .map_err(|_| {
              log::error!("Timeout waiting for session detail response");
              WebRtcErrors::Timeout
          })?;

      match response {
          Response::ViewSessionResponse(resp) => {
              match resp.result {
                  Some(ResponseResult::Session(proto_session)) => {
                      let session = TransferSession {
                          order_id: proto_session.order_id,
                          resources: vec![],
                          progress: vec![],
                          transfer_type: TransferType::Receive,
                          target: TransferTarget::P2P {
                              from_peer: Some(self.peer.clone()),
                              signalling_key: String::new(),
                              scope: String::new(),
                              connection_state: P2PConnectionState::Connected,
                          },
                          access_url: String::new(),
                          alias: proto_session.alias.clone().unwrap_or_default(),
                          from_user: User { id: 0, email: String::new(), name: String::new(), avatar: String::new() },
                          password: None,
                          is_required_password: false,
                          cancellation_token: CancellationToken::new(),
                      };

                      core_request.response(session).await;
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

- Add separate `send_resource_notification()` method for notifying about resources:
  ```rust
  pub async fn send_resource_notification(
      &self,
      session_order_id: u64,
      resource: LocalResource,
  ) -> Result<(), WebRtcErrors> {
      let resource_proto = resource.to_proto();

      // Load thumbnail if exists
      let mut resource_with_thumbnail = resource_proto;
      if let Some(thumbnail_path) = resource.thumbnail_path.as_ref() {
          if let Ok(mut thumbnail_cursor) = self.resource_repo.read(thumbnail_path.clone(), 64 * 1024, false).await {
              if let Ok(bytes) = thumbnail_cursor.read_all().await {
                  resource_with_thumbnail.thumbnail_png = Some(bytes.to_vec());
              }
          }
      }

      // Notify peer about new resource
      let notification = ResourceNotification {
          session_order_id,
          resource: Some(resource_with_thumbnail),
      };

      self.msg_channel.notify(Request::ResourceNotification(notification)).await?;
      Ok(())
  }
  ```

- Update `send_session_detail_response()` to only send session metadata, resources notified separately:
  ```rust
  pub async fn send_session_detail_response(
      &self,
      request_id: String,
      session: Option<TransferSession>,
      error: Option<CoreError>,
  ) -> Result<(), WebRtcErrors> {
      use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;

      if let Some(error_msg) = error {
          // ... existing error handling
      }

      let Some(session) = session else {
          return Ok(())
      };

      // Send only session metadata
      let proto_session = P2pTransferSessionMessage {
          order_id: session.order_id,
          resources: vec![],
          user_id: Some(session.from_user.id),
          alias: if !session.alias.is_empty() {
              Some(session.alias.clone())
          } else {
              None
          },
      };

      let response = ViewSessionDetailResponse {
          result: Some(ResponseResult::Session(proto_session))
      };

      self.msg_channel.send_response(request_id, Response::ViewSessionResponse(response)).await?;

      // Resources will be notified separately via send_resource_notification
      // when they are ready/updated

      Ok(())
  }
  ```

**Impact:**
- Timeout only applies to session metadata (6 seconds is sufficient)
- Resources are notified asynchronously as they become available
- Cleaner separation between session initialization and resource updates

---

### 6. Update Transfer Commands to Handle Connection State
**File:** `shared/src/app/transfer/commands.rs`

**Changes:**
- Update `request_session_detail()` to handle session metadata only, resources come via notifications:
  ```rust
  pub async fn request_session_detail(
      &self,
      peer_id: String,
      order_id: u64,
      password: Option<String>
  ) -> Result<(), CoreError> {
      let session_id = TransferSessionId {
          order_id: Some(order_id.to_string()),
          transfer_type: Some(TransferType::Receive)
      };

      // Set session to Connecting state
      self.update_model(TransferSessionModelEvent::Update(
          session_id.clone(),
          Box::new(|session: &mut TransferSession| {
              session.set_connecting();
          })
      ));

      // Request session detail (with 6-second timeout)
      let result = self.run(P2POperation::ViewSessionDetail {
          peer_id: peer_id.clone(),
          order_id,
          password
      }).await;

      match result {
          Ok(session) => {
              // Session detail received successfully - mark as connected
              self.update_model(TransferSessionModelEvent::Update(
                  session_id.clone(),
                  Box::new(|session: &mut TransferSession| {
                      if let TransferTarget::P2P { connection_state, .. } = &mut session.target {
                          *connection_state = P2PConnectionState::Connected;
                      }
                  })
              ));

              // Save session to persistent storage
              let _ = self.run(TransferSessionPersistentOperation::save(session)).await;
          }
          Err(err) => {
              log::error!("Failed to load session detail: {err:?}");

              // Update session to Failed state
              self.update_model(TransferSessionModelEvent::Update(
                  session_id.clone(),
                  Box::new(move |session: &mut TransferSession| {
                      session.set_connection_failed(err.to_string());
                  })
              ));
          }
      }

      Ok(())
  }
  ```

- Add handler for resource notifications in WebRTC peer message processing:
  ```rust
  // In WebRtcPeer::process_message_packet()
  Request::ResourceNotification(notification) => {
      let resource_proto = notification.resource;
      let session_order_id = notification.session_order_id;

      if let Some(resource_proto) = resource_proto {
          let mut resource = LocalResource {
              order_id: resource_proto.order_id,
              name: resource_proto.name,
              size: resource_proto.size as u64,
              path: LocalResourcePath::RelativePath {
                  path: format!("received/session_{}/resource_{}", session_order_id, resource_proto.order_id),
                  is_private: false,
              },
              thumbnail_path: None,
              r#type: (ResourceTypeMessage::try_from(resource_proto.r#type)
                  .unwrap_or_default())
                  .try_into()
                  .unwrap_or(ResourceType::File),
          };

          // Save thumbnail if present
          if let Some(thumbnail_bytes) = resource_proto.thumbnail_png {
              match self.resource_repo.save_thumbnail(thumbnail_bytes, resource.order_id).await {
                  Ok(thumbnail_path) => {
                      resource.thumbnail_path = Some(thumbnail_path);
                  }
                  Err(e) => {
                      log::warn!("Failed to save thumbnail: {:?}", e);
                  }
              }
          }

          // Send to core for session update
          if let Some(core_request) = self.core_request() {
              core_request.response(CoreOperationOutput::ResourceNotification {
                  session_order_id,
                  resource,
                  peer_id: self.peer.id().to_string(),
              }).await;
          }
      }
  }
  ```

- Add event handler in Transfer module to process resource notifications:
  ```rust
  // In TransferEvent enum
  ResourceNotification {
      session_order_id: u64,
      resource: LocalResource,
      peer_id: String,
  }

  // In TransferModule::update()
  TransferEvent::ResourceNotification { session_order_id, resource, peer_id } => {
      let session_id = TransferSessionId {
          order_id: Some(session_order_id.to_string()),
          transfer_type: Some(TransferType::Receive)
      };

      // Get session mutably
      let Some(session) = model.transfer.sessions.lookup_mut(&session_id) else {
          log::warn!("Session {} not found for resource notification", session_order_id);
          return Command::done();
      };

      // Get peer to verify ownership
      let Some(peer) = model.nearby.peers.iter().find(|p| p.id == peer_id) else {
          log::warn!("Peer {} not found, ignoring resource notification", peer_id);
          return Command::done();
      };

      // Add resource directly - ownership logic handled in entity
      if session.add_resource_from_peer(resource, peer) {
          log::info!("Added resource to session {} from peer {}", session_order_id, peer_id);
      }

      Command::render()
  }
  ```

**Impact:**
- Session connection completes quickly with just metadata
- Resources added asynchronously via notifications
- Clear state transitions: Connecting → Connected (on success) or Failed (on timeout/error)

---

### 7. Update Scope Creation to Use Direct Protocol
**File:** `shared/src/app/transfer/module.rs` (StartP2PTransfer event)

**Changes:**
- When creating finding scope, format it as direct with proper owner designation:
  ```rust
  TransferEvent::StartP2PTransfer { password, .. } => {
      // ... existing session creation code

      // Create scope with direct protocol and owner flag
      let scope_string = format!("direct://{}:owner", p2p_session.signalling_room_id);
      let scope = FindingScope::new(&scope_string);

      it.update_model(NearbyEvent::AddFindingScope(scope));

      Ok(())
  }
  ```

- When receiver joins scope (on peer discovery), format as member:
  ```rust
  // In nearby module or when discovering sessions
  let scope_string = format!("direct://{}:member", signalling_room_id);
  let scope = FindingScope::new(&scope_string);
  ```

**Impact:** Clear ownership semantics in P2P connections.

---

### 8. Reset Connection State When Saving P2P Sessions
**File:** Implementation depends on where persistence is handled (likely in executor/persistent.rs)

**Changes:**
- Update `TransferSessionPersistentOperation::Save` handler:
  ```rust
  // Before saving the session
  let mut session_to_save = session.clone();

  if let TransferTarget::P2P { connection_state, from_peer, .. } = &mut session_to_save.target {
      // Reset connection state to NotConnected
      *connection_state = P2PConnectionState::NotConnected;
      // Optionally clear peer if desired
      *from_peer = None;
  }

  // Save the modified session
  repository.save(session_to_save)?;
  ```

**Impact:** Sessions loaded from disk start in a clean state, requiring fresh connections.

---

## Error Handling
- Add `Timeout` variant to `WebRtcErrors` enum
- Add `Timeout(String)` variant to `CoreError` enum
- Proper error propagation through CoreRequest stream

## Testing Checklist
- [ ] P2P session starts in NotConnected state
- [ ] Connection state transitions: NotConnected → Connecting → Connected
- [ ] Connection state transitions: NotConnected → Connecting → Failed on timeout
- [ ] Session status reflects connection state properly
- [ ] Peer disconnect resets connection state
- [ ] Saved sessions have reset connection state
- [ ] Scope creation uses direct protocol with owner/member flags
- [ ] Timeout triggers after 6 seconds without response
- [ ] Multiple view detail requests are prevented when already connecting
- [ ] Failed connections can be retried by user

## Migration Notes
- Existing P2P sessions in database need migration to add `connection_state` field
- Default to `NotConnected` for backward compatibility