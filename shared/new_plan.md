# P2P Session Implementation Plan

## Overview
Implement P2P session creation and discovery via backend RPC, enabling direct peer-to-peer transfers with signaling scope coordination.

## Architecture
- **Backend**: Create/Find P2P sessions, generate signaling keys, manage session lifecycle
- **Sender**: Create session on backend, add scope to WebRTC, display session info
- **Receiver**: Find session by alias, extract scope, connect via WebRTC, view/download resources

## Connection Flow Sequence

```
SENDER SIDE:
1. User selects resources + clicks "Start P2P Transfer"
2. Call backend RPC: create_device_session(password_protected)
3. Backend returns: P2PSession { alias, owner_signalling_key, scope, ... }
4. Create local TransferSession { target: P2P { from_peer: None, scope, signalling_key: owner_key, ... } }
5. Emit event to Nearby module: AddFindingScope(FindingScope::Local(scope))
6. Nearby module adds scope to model.nearby.finding_scopes
7. Nearby module calls P2POperation::update_finding_scopes() → WebRTC RPC
8. Display session with alias/URL (same UI as public session, with "Stop Transfer" button)
9. When receiver connects, they request session details via P2P protocol (sender doesn't track who)
10. When "Stop Transfer" pressed → Emit RemoveFindingScope → Nearby removes scope → Updates WebRTC

RECEIVER SIDE:
1. User enters alias/keywords (e.g., "brave-dolphin-42")
2. Trigger existing FindPublicSession event (same event for both P2P and public)
3. RPC searches BOTH P2P (by alias) AND public (by keywords) sessions
4. If P2P found: Create TransferSession { target: P2P { from_peer: None, scope, signalling_key: member_key, ... } }
   - Emit AddFindingScope to Nearby module → WebRTC scopes updated
5. If public found: Create TransferSession { target: Internet { ... } } (existing flow)
6. Display found session (UI shows "Connecting..." for P2P with no peer)
7. FOR P2P: WebRTC event: PeerConnected(peer) → Update from_peer field
8. User clicks to view session details
9. Route based on target type:
   - P2P with peer: Call request_session_detail(peer_id, order_id, password)
   - Public: Call view_public_session(session, password) (existing)
10. Display resources → User can download

KEY INSIGHTS:
- Sender doesn't track individual receivers - just broadcasts availability via scope
- Nearby module owns scope management and WebRTC coordination
- Transfer module emits events to Nearby module to add/remove scopes
- Receiver uses SAME FindPublicSession event for both P2P and public sessions
- RPC layer searches both P2P (by alias) and public (by keywords) simultaneously
- Only "view session details" routing differs between P2P and public
```

---

## Implementation Steps

### 0. Nearby Module - Add Scope Management Events
**Files**: `shared/src/app/nearby/module.rs`

**Tasks**:
1. **Add new events to `NearbyEvent`**:
   ```rust
   pub enum NearbyEvent {
       Launch { auto_launch: bool },
       UpdateMe { new_peer: Peer },
       UpdateNearbyPeers { new_peer: Vec<Peer>, removed: Vec<Peer> },
       ClearNearbyPeers,

       // NEW: Scope management
       AddFindingScope(FindingScope),
       RemoveFindingScope(FindingScope),
   }
   ```

2. **Handle AddFindingScope event**:
   ```rust
   NearbyEvent::AddFindingScope(scope) => {
       if !model.nearby.finding_scopes.contains(&scope) {
           model.nearby.finding_scopes.push(scope);

           // Update WebRTC
           let scopes = model.nearby.finding_scopes.clone();
           return Command::handle_result(|it| async move {
               it.run(P2POperation::update_finding_scopes(scopes)).await
           });
       }
       Command::none()
   }
   ```

3. **Handle RemoveFindingScope event**:
   ```rust
   NearbyEvent::RemoveFindingScope(scope) => {
       model.nearby.finding_scopes.retain(|s| s != &scope);

       // Update WebRTC
       let scopes = model.nearby.finding_scopes.clone();
       return Command::handle_result(|it| async move {
           it.run(P2POperation::update_finding_scopes(scopes)).await
       });
   }
   ```

**Design Note**: Nearby module owns all WebRTC scope coordination. Transfer module just emits add/remove events.

---

### 1. Backend Integration - RPC Client Setup
**Files**: `shared/src/protocol/rpc/app_server.rs` or equivalent RPC client module

**Tasks**:
1. **Add `P2pOrchestrationServiceClient`** to RPC client structure

2. **Implement sender method**:
   - `create_device_session(password_protected: bool) -> Result<P2PSession, CoreError>`
   - Use authenticated channel (User + Device extensions)

3. **Update existing `find_transfer_session` method** to search BOTH:
   ```rust
   pub async fn find_transfer_session(keywords: String) -> Result<Option<TransferSession>, CoreError> {
       // Try P2P first
       if let Ok(Some(p2p_session)) = self.find_p2p_session_by_alias(keywords.clone()).await {
           // Convert P2PSession proto to local TransferSession with P2P target
           return Ok(Some(create_p2p_transfer_session(p2p_session)));
       }

       // Fallback to existing public session search
       self.find_public_session_by_keywords(keywords).await
   }
   ```

4. **Add helper method**:
   - `find_p2p_session_by_alias(alias: String) -> Result<Option<P2PSession>, CoreError>`
   - Calls backend `find_session` RPC
   - Returns proto `P2PSession` object

**Details**:
- P2PSession proto contains: session_id, signalling_room_id, owner_user_id, password_protected, access_url, alias, signalling_scope
- Unified search means receiver doesn't need to know if it's P2P or public

---

### 2. Sender Flow - Create P2P Session
**Files**:
- `shared/src/app/transfer/module.rs` (StartP2PTransfer event handler)
- `shared/src/app/transfer/commands.rs` (add new command function)
- `shared/src/entities/transfer_session.rs` (update `p2p()` constructor)

**Current State**:
```rust
// transfer/module.rs:236
TransferEvent::StartP2PTransfer { nearby_available, password } => {
    let session = TransferSession::p2p(selected_resources, me, password);
    // Currently creates local TransferSession only
    // Doesn't call backend to create P2P session
}

// transfer_session.rs:192
pub fn p2p(mut resources: Vec<LocalResource>, password: Option<String>) -> Self {
    // Currently creates session WITHOUT signalling_key and scope
}
```

**Changes**:
1. **Before creating local session**, call backend RPC:
   ```rust
   let p2p_session = app_server.create_device_session(password.is_some()).await?;
   ```

2. **Extract session info**:
   - `session_id`: Backend-generated unique ID
   - `alias`: Human-readable identifier (e.g., "brave-dolphin-42")
   - `signalling_scope`: Scope string for WebRTC (e.g., "brave-dolphin-42")
   - `owner_signalling_key`: RTC signaling room ID for sender (e.g., "direct:brave-dolphin-42:12345;owner")
   - `access_url`: Share URL (e.g., "https://bitbridge.com/p2p?session=brave-dolphin-42")

3. **Update `TransferSession::p2p()` constructor**:
   - Current signature: `pub fn p2p(resources, password) -> Self`
   - New signature: `pub fn p2p(resources, password, signalling_key, scope) -> Self`
   - Create session with:
     ```rust
     TransferTarget::P2P {
         from_peer: None,  // No peer yet, they will connect to us
         password,
         is_required_password: password.is_some(),
         signalling_key,  // Owner key for sender
         scope,
     }
     ```

   - Call it with backend data:
     ```rust
     let session = TransferSession::p2p(
         selected_resources,
         password,
         p2p_session.owner_signalling_key,  // from backend
         p2p_session.signalling_scope,      // from backend
     );
     ```

4. **Emit event to Nearby module to add scope**:
   ```rust
   let scope = FindingScope::Local(p2p_session.signalling_scope.clone());
   // Emit to Nearby module (not direct RPC call)
   self.update_model(NearbyEvent::AddFindingScope(scope));
   ```
   - Nearby module receives this event
   - Adds scope to `model.nearby.finding_scopes`
   - Calls `P2POperation::update_finding_scopes(model.nearby.finding_scopes.clone())`
   - WebRTC now listens on this scope for any receiver connections

5. **Store alias and access_url for UI display**:
   - Option A: Add new fields to `TransferSession` struct:
     ```rust
     pub struct TransferSession {
         // ... existing fields
         pub alias: Option<String>,        // For P2P sessions
         pub access_url: Option<String>,   // For both P2P and Internet sessions
     }
     ```
   - Option B: Extract from `TransferTarget::P2P` fields when needed in view layer
   - Recommendation: Use Option A for cleaner view layer code

6. **Display session info with Stop button**:
   - Show `alias` and `access_url` to user (for sharing)
   - Display "Stop Transfer" button (similar to how cloud sessions work)
   - Sender doesn't track individual receivers - just broadcasts on scope
   - ANY receiver with the alias can connect and request details

---

### 3. Sender Flow - Handle Stop Transfer
**Files**:
- `shared/src/app/transfer/module.rs` (CancelTransfer event)
- `shared/src/app/nearby/module.rs` (add RemoveFindingScope event)

**Tasks**:
1. **When user clicks "Stop Transfer"**:
   - Already handled by existing `CancelTransfer` event
   - Need to add scope cleanup logic

2. **Remove scope from Nearby module**:
   ```rust
   // In CancelTransfer event handler
   if let TransferTarget::P2P { scope, .. } = &session.target {
       let finding_scope = FindingScope::Local(scope.clone());
       self.update_model(NearbyEvent::RemoveFindingScope(finding_scope));
   }
   ```

3. **Nearby module handles RemoveFindingScope**:
   - Remove scope from `model.nearby.finding_scopes`
   - Call `P2POperation::update_finding_scopes(model.nearby.finding_scopes.clone())`
   - WebRTC stops listening on that scope

---

### 4. Receiver Flow - Find Session (Unified P2P + Public)
**Files**:
- `shared/src/protocol/rpc/app_server.rs` (update `find_transfer_session`)
- `shared/src/app/transfer/commands.rs` (update `find_transfer_session`)
- `shared/src/app/transfer/module.rs` (FindPublicSession event - NO CHANGES)

**Current Flow**:
```rust
// transfer/module.rs:324
TransferEvent::FindPublicSession { mut keywords } => {
    // Parse URL if needed
    // Call find_transfer_session(keywords)
    // Add session to model
}
```

**Changes**:
1. **Update RPC `find_transfer_session`** (see Step 1 above):
   - Try P2P search by alias first
   - If not found, try public session search
   - Return unified `TransferSession` regardless of type

2. **Handle P2P session result** in `find_transfer_session` command:
   ```rust
   // In commands.rs
   pub async fn find_transfer_session(&self, keywords: String) -> Result<(), CoreError> {
       let session_overview = self.run(TransferOperation::find_transfer_session(keywords)).await?;

       let Some(session) = session_overview else {
           // Show "not found" message
           return Ok(());
       };

       // If P2P session, add scope to Nearby module
       if let TransferTarget::P2P { ref scope, .. } = session.target {
           let finding_scope = FindingScope::Local(scope.clone());
           self.update_model(NearbyEvent::AddFindingScope(finding_scope));
       }

       // Save and add to model (existing logic)
       self.run(TransferSessionPersistentOperation::save(session.clone())).await?;
       self.update_model(TransferSessionModelEvent::Add(session));
       Ok(())
   }
   ```

3. **No changes to FindPublicSession event handler** - it already calls `find_transfer_session`

**Important State for P2P**: Session created with `from_peer: None`, so:
- UI shows "Connecting..." or session card with limited info
- When `PeerConnected` event fires, `from_peer` gets populated
- Then user can view session details

---

### 5. Receiver Flow - Peer Connection and Session Details
**Files**:
- `shared/src/app/transfer/module.rs` (handle P2POperationOutput::PeerConnected)
- `shared/src/app/transfer/commands.rs`

**Phase A: Handle Peer Connection**

When WebRTC establishes connection, `PeerConnected(peer)` event fires:

1. **Update session with peer info** (in Transfer or Nearby module):
   ```rust
   // When handling P2POperationOutput::PeerConnected(peer)
   // Find receiver-side session waiting for this peer
   if let Some(session) = model.transfer.sessions.iter_mut().find(|s| {
       matches!(s.target, TransferTarget::P2P {
           from_peer: None,
           ref scope,
           ..
       } if scope == &peer.scope)  // Match by scope
   }) {
       // Update from_peer field
       session.target = TransferTarget::P2P {
           from_peer: Some(peer.clone()),
           // ... keep other fields (scope, signalling_key, password, is_required_password)
       };
   }
   ```

2. **Now session has peer_id**:
   - `peer_id()` returns `Some(peer_id)`
   - Can proceed to request session details

**Note**: Sender-side sessions don't update `from_peer` - they respond to any peer that requests details

**Phase B: Request Session Details**

**Current State**:
- `view_public_session` handles public (cloud) sessions
- `request_session_detail` handles P2P session detail requests (lines 279-331)

**Tasks**:
1. **Auto-request details after peer connects** (optional):
   - After updating `from_peer`, automatically call `request_session_detail`
   - OR wait for user to explicitly "view" the session

2. **Reuse existing `request_session_detail`**:
   - Already streams session details from peer
   - Already handles resources and progress updates
   - Requires `peer_id` (now available after connection)

3. **Update `ViewSession` event handler** in module.rs (line 342):
   ```rust
   TransferEvent::ViewSession { password, session_id, transfer_type } => {
       let session_id = TransferSessionId {
           order_id: Some(session_id.to_string()),
           transfer_type: Some(TransferType::Receive)
       };

       let Some(session) = model.transfer.sessions.lookup(&session_id).cloned() else {
           return Command::done();
       };

       // Route based on target type
       match &session.target {
           TransferTarget::P2P { from_peer, .. } => {
               // Check if peer is connected
               let Some(peer_id) = session.peer_id() else {
                   return Command::new(|it| async move {
                       DialogOperation::toast("Waiting for connection...".to_string())
                           .into_future(it.clone()).await;
                   });
               };

               // Request details from peer
               Command::handle_result(move |it| async move {
                   it.app().request_session_detail(peer_id, session.order_id, password).await
               })
           }
           TransferTarget::Internet { .. } => {
               // Existing public session flow
               Command::handle_result(|it| async move {
                   it.app().view_public_session(session, password).await
               })
           }
       }
   }
   ```

4. **Handle password protection**:
   - P2P sessions may require password
   - Validate on sender side via `handle_view_session_request` (already implemented, lines 240-277)

---

### 6. Resource Download Flow
**Files**: Already implemented in commands.rs

**Current Implementation**:
- `request_download_resource` (lines 349-385): Receiver requests resource from peer
- `handle_download_request` (lines 333-347): Sender handles download request
- Both functions already work with P2P sessions

**No Changes Needed**:
- Existing P2P download flow is complete
- Uses peer_id and session_id for coordination
- Streams resource data via WebRTC

---

### 7. WebRTC Signaling Integration
**Files**:
- `shared/src/protocol/webrtc/webrtc.rs`
- WebRTC peer connection setup

**Tasks**:
1. **Verify scope handling**:
   - Sender adds `owner_signalling_key` scope to WebRTC
   - Receiver adds `member_signalling_key` scope to WebRTC
   - Scopes must match pattern: `direct:{alias}:{session_id};{role}`

2. **Signaling coordination**:
   - Both peers listen on their respective signaling rooms
   - Backend RPC service doesn't participate in signaling (P2P direct)
   - Scopes enable WebRTC peer discovery without global broadcast

**Validation**:
- Test that sender and receiver can discover each other via scopes
- Verify signaling keys don't collide with other sessions

---

### 8. UI/UX Updates
**Files**:
- `web-next/app/transfer/receive_board.tsx` (already modified)
- Frontend components for P2P session display

**Tasks**:
1. **Sender UI**:
   - Display P2P session similar to public session
   - Show: alias, access_url, QR code (optional)
   - Allow cancellation

2. **Receiver UI**:
   - Use existing input field (same UI for P2P and public sessions)
   - Existing URL parsing logic works: `/p2p?session=brave-dolphin-42` → `brave-dolphin-42`
   - **Session card states** (for P2P sessions):
     - Show session with limited info when `from_peer: None`
     - Can display "Connecting..." indicator
     - After `PeerConnected`, enable "View Details" button
   - Clicking "View Details" triggers ViewSession event
   - If peer not connected yet, show "Waiting for connection..." toast
   - Show password prompt if required (before requesting details)

---

### 9. Error Handling & Edge Cases

**Scenarios**:
1. **Session not found**: Show friendly error, don't crash
2. **Incorrect password**: Sender rejects request, receiver gets error message
3. **Sender offline**: Receiver sees session but can't connect (timeout)
4. **Session expiry**: Backend could implement TTL, but not in initial scope
5. **Multiple receivers**: Each receiver gets unique signaling connection
6. **Network errors**: Retry logic in RPC client, fallback messaging

**Implementation**:
- Add proper error types to `CoreError`
- Display user-friendly messages via `DialogOperation`
- Log detailed errors for debugging

---

### 10. Testing Checklist

**Backend**:
- [ ] Create P2P session via gRPC (authenticated)
- [ ] Find P2P session by alias
- [ ] Update session password protection
- [ ] Handle duplicate device sessions (should update, not create)

**Sender Flow**:
- [ ] Create P2P session via backend RPC
- [ ] Emit AddFindingScope event to Nearby module
- [ ] Verify Nearby module adds scope and calls P2POperation::update_finding_scopes
- [ ] Display session alias and URL with "Stop Transfer" button
- [ ] Respond to session detail requests from any receiver (don't track who)
- [ ] Stream resources on download request
- [ ] Stop transfer removes scope via RemoveFindingScope event

**Receiver Flow**:
- [ ] Enter alias in existing FindPublicSession UI (same event for P2P and public)
- [ ] RPC searches BOTH P2P (by alias) and public (by keywords)
- [ ] If P2P found, session created with from_peer: None
- [ ] Verify AddFindingScope event emitted to Nearby module
- [ ] Verify WebRTC scopes updated
- [ ] Handle PeerConnected event → update session's from_peer field
- [ ] Verify peer_id() now returns Some(peer_id)
- [ ] ViewSession event routes based on target type:
  - [ ] P2P with peer → request_session_detail
  - [ ] P2P without peer → show "Waiting for connection"
  - [ ] Public → view_public_session (existing)
- [ ] Download resources successfully

**Error Cases**:
- [ ] Session not found (invalid alias)
- [ ] Wrong password provided
- [ ] Sender disconnects mid-transfer
- [ ] Network interruption handling

---

## Key Design Decisions

1. **Signaling Architecture**: Direct P2P via scopes, backend only stores session metadata
2. **Session Lifecycle**: Created on sender, discovered by receiver, no explicit deletion (stateless after creation)
3. **Backward Compatibility**: Fallback to public sessions ensures existing flows work
4. **Password Protection**: Optional, validated on sender when receiver requests details
5. **Resource Management**: Local on sender, streamed on-demand to receiver

---

## Files Summary

### Backend (Already Complete)
- ✅ `backend/src/entities/p2p_session.rs`
- ✅ `backend/src/grpc/p2p_service.rs`
- ✅ `backend/src/repositories/p2p_session.rs`
- ✅ `backend/src/infrastructure/postgres/p2p_session.rs`
- ✅ `backend/src/transfer/p2p_transfer_service.rs`
- ✅ `backend/migration/src/m20251227_000004_create_p2p_session_table.rs`

### Shared/Core (Needs Implementation)
- 🔧 `shared/src/app/nearby/module.rs` - **CRITICAL**: Add AddFindingScope and RemoveFindingScope events
- 🔧 `shared/src/protocol/rpc/app_server.rs` - Add P2P RPC client methods (create_device_session, find_p2p_session_by_alias)
- 🔧 `shared/src/protocol/rpc/app_server.rs` - Update `find_transfer_session` to search BOTH P2P and public
- 🔧 `shared/src/app/transfer/module.rs` - Update StartP2PTransfer to emit AddFindingScope event
- 🔧 `shared/src/app/transfer/module.rs` - Update CancelTransfer to emit RemoveFindingScope event
- 🔧 `shared/src/app/transfer/module.rs` - Update ViewSession to route based on target type (lines 342-353)
- 🔧 `shared/src/app/transfer/commands.rs` - Update `find_transfer_session` to emit AddFindingScope for P2P
- ✅ `shared/src/app/transfer/commands.rs` - NO changes to FindPublicSession event handler
- 🔧 `shared/src/entities/transfer_session.rs` - Update `p2p()` constructor to accept signalling_key and scope
- ✅ `shared/src/entities/target.rs` - Already has signalling_key and scope fields
- 📝 `shared/src/protocol/webrtc/webrtc.rs` - Verify scope handling
- 🔧 Handle `PeerConnected` event to update `from_peer` field (receiver-side sessions only)

### Frontend (Partial)
- 🔧 `web-next/app/transfer/receive_board.tsx` - Update receiver UI
