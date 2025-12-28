# Transfer Session Connect and Disconnect Implementation Plan

## Overview
Implement dynamic transfer session state management based on peer scope ownership changes, allowing sessions to reconnect/disconnect as peers join/leave scopes.

## Components to Modify

### 1. Peer Scope Management (shared/src/app/nearby/command.rs)
**Current behavior**: Peers have static scopes set during connection

**New behavior**:
- Each peer maintains a base finding scope equal to their own peer ID
- When peer A connects to peer B:
  - Peer A fires `NearbyEvent::AddFindingScope(peer_B_id)`
  - Peer B fires `NearbyEvent::AddFindingScope(peer_A_id)`
- When peer disconnects:
  - Fire `NearbyEvent::RemoveFindingScope(peer_id)`

**Implementation**:
- In `handle_peer_connection()`: After successful connection, fire `NearbyEvent::AddFindingScope(FindingScope::new(&peer.id))`
- In peer disconnection handler (line 142-145): Before firing `PeerDisconnected`, fire `NearbyEvent::RemoveFindingScope(FindingScope::new(&peer.id))`
- The `NearbyModule` will handle these events automatically (already implemented in nearby/module.rs:68-86)

### 2. Scope Change Notification (shared/src/protocol/webrtc/signalling.rs)
**Current behavior**: Scopes are sent during initial connection only

**New behavior**:
- When finding scopes are updated via `update_finding_scopes()`, broadcast the change to all connected peers
- Peers receive scope updates and notify the core via `P2POperationOutput::PeerScopesUpdated`

**Implementation**:
- In `update_finding_scopes()`: After updating scopes, send scope update message to all connected peers
- Add new message type in protocol for scope updates (if not exists)
- Handle incoming scope update messages and emit `PeerScopesUpdated` event

### 3. Peer Event Handling (shared/src/app/nearby/command.rs)
**Current behavior**: `PeerScopesUpdated` event updates peer scopes and notifies transfer module

**New behavior**: (Already implemented correctly)
- On `PeerScopesUpdated`: Update peer scopes and emit `TransferEvent::PeerUpdated`

**Implementation**: ✓ Already implemented in line 147-151

### 4. Transfer Session State Management (shared/src/app/transfer/module.rs)
**Current behavior**:
- `PeerUpdated`: Connects sessions when peer becomes owner
- `PeerDisconnected`: Cleans up session when peer disconnects

**New behavior**:
- `PeerUpdated`:
  - If peer becomes owner of a session → connect the session (existing flow)
  - If peer is no longer owner of a connected session → disconnect and reset to loading state
- `PeerDisconnected`: (keep existing behavior)

**Implementation**:
- In `TransferEvent::PeerUpdated` handler:
  ```rust
  // Existing: Check for sessions waiting for this peer (peer becomes owner)
  // NEW: Check all connected sessions with this peer
  for session in sessions where from_peer == peer {
      if !peer.is_owned(session) {
          // Peer is no longer owner - disconnect
          session.from_peer = None;
          session.resources.clear();
          session.progress.clear();
          // Optionally: Remove finding scope if needed
      }
  }
  ```

### 5. Base Peer Scope Initialization (shared/src/app/nearby/command.rs)
**New behavior**:
- When nearby server starts, add a base finding scope equal to the peer's own ID
- This allows other peers to find this peer by its ID

**Implementation**:
- In `start_nearby_server()`: After creating the peer and before starting the server, fire `NearbyEvent::AddFindingScope(FindingScope::new(&peer.id))`
- This ensures every peer always advertises its own ID as a findable scope

## Implementation Steps

1. **Step 1**: Add base peer scope on server start (nearby/command.rs)
   - In `start_nearby_server()`: Fire `AddFindingScope` with peer's own ID after creating peer

2. **Step 2**: Add peer scope on connection (nearby/command.rs)
   - In `handle_peer_connection()` after peer connects: Fire `AddFindingScope` with connected peer's ID

3. **Step 3**: Remove peer scope on disconnection (nearby/command.rs)
   - In peer disconnection handler: Fire `RemoveFindingScope` with disconnected peer's ID before firing `PeerDisconnected`

4. **Step 4**: Implement scope change broadcasting in `signalling.rs`
   - In `update_finding_scopes()`: Send scope update message to all connected peers
   - Handle incoming scope update messages and emit `PeerScopesUpdated` event

5. **Step 5**: Implement session disconnect logic in `transfer/module.rs`
   - In `PeerUpdated` handler: Check if connected peer lost ownership and reset session

6. **Step 6**: Test scenarios
   - Peer A creates session, Peer B connects and views it
   - Peer B disconnects → Peer A's session should reset
   - Peer B reconnects → Peer A's session should reconnect
   - Multiple peers connecting/disconnecting to same session

## Key Data Flows

### Peer Connection Flow
```
1. Peer A connects to Peer B (via introduce)
2. Peer A adds scope = Peer B's ID
3. Peer A broadcasts scope update to signalling server
4. Signalling server notifies Peer B: "Peer A's scopes changed"
5. Peer B receives PeerScopesUpdated event
6. Transfer module checks if Peer B owns any sessions in Peer A's scopes
7. If yes, connect the session
```

### Peer Disconnection Flow
```
1. Peer B disconnects
2. Peer A removes scope = Peer B's ID
3. Peer A broadcasts scope update
4. Transfer module checks if any connected sessions have Peer B as owner
5. If yes, reset session to loading state
```

### Owner Loss Flow (without disconnect)
```
1. Peer B is owner of session X (scope = "session-123")
2. Peer B removes scope "session-123" (e.g., cancelled the session)
3. Peer B broadcasts scope update to Peer A
4. Peer A receives PeerScopesUpdated event
5. Transfer module checks: Peer B no longer owns session X
6. Reset session X to loading state on Peer A
```

## Files to Modify

1. `shared/src/app/nearby/command.rs` - Add/remove peer scopes on connection/disconnection
2. `shared/src/protocol/webrtc/signalling.rs` - Scope update broadcasting to peers
3. `shared/src/app/transfer/module.rs` - Session disconnect logic when peer loses ownership
4. `schema/` - Add scope update message types (if not exists)

## Questions to Resolve

1. ✅ ~~Should we automatically add peer ID to finding scopes?~~ Yes, use existing `AddFindingScope`/`RemoveFindingScope` events
2. Should we remove the finding scope when a peer disconnects, or keep it for potential reconnection?
3. How to handle the case where a session is in "loading" state for too long?
4. Should we show different UI states for "waiting for peer" vs "peer lost connection"?
5. Do we need to persist the session state when peer disconnects?
