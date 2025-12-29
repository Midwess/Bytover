# Implementation Plan: P2P Session Online/Offline State Indicator

## Overview
Display online/offline status for P2P transfer sessions using a visual indicator (green/gray dot) based on RPC signalling events.

## Architecture Flow
1. RPC Signalling Server sends `ScopeStateChanged` events (already defined in proto)
2. Core services receive and process these events
3. Events propagate to `NearbyModule` to update scope states
4. `FindingScope` entities track online/offline state
5. `TransferSession` references `FindingScope` to access state
6. View models expose state for UI rendering

## Implementation Steps

### 1. Update FindingScope Entity
**File:** `shared/src/entities/finding_scope.rs`

**Add new fields to existing `FindingScope` struct:**

```rust
pub struct FindingScope {
    scope_id: String,
    is_direct: bool,
    is_owner: bool,
    is_watcher: bool,  // NEW: indicates passive observation mode
    state: ScopeState,  // NEW: Online/Offline state from RPC signalling
}
```

**Changes needed:**
- Add `is_watcher: bool` field - distinguishes watcher vs member participation
- Add `state: ScopeState` field - tracks online/offline status
- Add getter methods:
  - `is_online() -> bool` - returns `state == ScopeState::Online`
  - `is_watcher() -> bool` - returns the watcher flag
  - `set_watcher(&mut self, is_watcher: bool)` - setter for watcher mode
  - `update_state(&mut self, state: ScopeState)` - setter for state
- Update `new()` constructor:
  - Parse "watcher" from scope string (e.g., "direct://scope_id;watcher")
  - Initialize `state: ScopeState::Offline` by default
- Update `as_string()` to include watcher role when serializing
- Ensure serialization/deserialization works with new fields

**Scope Participation Modes:**
- **Watcher** (lightweight): Receives only state updates (online/offline)
- **Member** (full): Active participant in data transfer and detailed updates
- **Owner**: Session creator, can have both watcher and member roles

### 2. Update TransferTarget Enum
**File:** `shared/src/entities/target.rs:14-24`

Current P2P variant:
```rust
P2P {
    from_peer: Option<Peer>,
    signalling_key: String,
    scope: String,
    connection_state: P2PConnectionState,
}
```

Changes needed:
- Rename `scope: String` → `scope_name: String` (for backward compatibility)
- Add `scope: Option<FindingScope>` (full scope instance with state and watcher flag)
- This allows:
  - Checking online/offline via `scope.as_ref().map(|s| s.is_online())`
  - Checking participation mode via `scope.as_ref().map(|s| s.is_watcher())`

**Note:** Use `Option<FindingScope>` because scope is initially `None` when session is created, then populated when scope state updates arrive

### 3. Update TransferSession
**File:** `shared/src/entities/transfer_session.rs:210-236`

In `TransferSession::p2p()` method:
- Update P2P variant initialization with new field names
- Pass both `scope_name` (from signalling_key) and `scope: None`
- Initially `scope: None`, will be populated when scope state updates arrive

**Also update all pattern matches:**
- Search for all `TransferTarget::P2P { scope, .. }` patterns
- Update to use `scope_name` where string is expected
- Update to use `scope.as_ref()` where `FindingScope` instance is needed

### 4. Add Scope State Update Event Handler
**File:** `shared/src/app/nearby/module.rs`

Add new event variant to `NearbyEvent` enum (line ~31-41):
```rust
ScopeStateUpdated { scope_id: String, state: ScopeState }
```

Implement handler in `update()` method (add after line 91):
```rust
NearbyEvent::ScopeStateUpdated { scope_id, state } => {
    // Update state in nearby finding_scopes
    if let Some(scope) = model.nearby.finding_scopes.iter_mut()
        .find(|s| s.scope_id() == scope_id) {
        scope.update_state(state);
    }

    // Propagate to all transfer sessions with matching scope
    for session in model.transfer.sessions.iter_mut() {
        if let TransferTarget::P2P { scope_name, scope, .. } = &mut session.target {
            if scope_name == &scope_id {
                // Update or create scope instance with new state
                if let Some(existing_scope) = scope {
                    existing_scope.update_state(state);
                } else {
                    *scope = model.nearby.finding_scopes.iter()
                        .find(|s| s.scope_id() == scope_id)
                        .cloned();
                }
            }
        }
    }

    Command::render()
}
```

### 5. Handle ScopeStateChanged in Signalling Layer
**File:** `shared/src/protocol/webrtc/signalling.rs`

In `TryFrom<SignallingPeerResponse> for PeerEvent` implementation (line 224-261):

Add handling for `scope_state_changed` message:
```rust
impl TryFrom<SignallingPeerResponse> for PeerEvent {
    fn try_from(value: SignallingPeerResponse) -> Result<Self, Self::Error> {
        let value = value.0;

        // Add this new condition
        if let Some(scope_state_msg) = value.scope_state_changed {
            // This will need a new PeerEvent variant or custom handling
            // Return scope state change event to be propagated
        }

        // ... existing conditions (join, ice_candidate, offer, answer, left)
    }
}
```

**Note:** May need to create a custom event type since `PeerEvent` is from matchbox library

### 6. Add P2POperationOutput for Scope State
**Location:** Find where `P2POperationOutput` enum is defined (likely in `libs/core-services` or `shared/src/app/operations/p2p.rs`)

Add new variant:
```rust
pub enum P2POperationOutput {
    // ... existing variants
    ScopeStateChanged { scope_id: String, state: ScopeState },
}
```

This output will be streamed from the P2P operation to the nearby command handler.

### 7. Handle Scope State in Nearby Command
**File:** `shared/src/app/nearby/command.rs`

In `start_nearby_server()` function (line 76-117), add new match arm:

```rust
while let Some(output) = start_p2p_server_stream.next().await {
    match output {
        // ... existing cases (PeerConnected, Error, etc.)

        CoreOperationOutput::P2P(P2POperationOutput::ScopeStateChanged { scope_id, state }) => {
            log::info!(target: "nearby", "Scope state changed: {} -> {:?}", scope_id, state);

            // Fire event to NearbyModule to update scope states
            self.notify_event(NearbyEvent::ScopeStateUpdated { scope_id, state });
        }
    }
}
```

**Also in `handle_peer_connection()`** (line 136-186), add similar handling:
```rust
CoreOperationOutput::P2P(P2POperationOutput::ScopeStateChanged { scope_id, state }) => {
    self.notify_event(NearbyEvent::ScopeStateUpdated { scope_id, state });
}
```

This ensures scope state changes are propagated regardless of which stream receives them.

### 8. Add Scope Watcher Management in Transfer Commands
**File:** `shared/src/app/transfer/commands.rs`

**When sessions are loaded/added (Watcher mode):**

Add scope as **watcher** in these locations:

1. **`load_transfer_sessions()`** (line 27-36):
   ```rust
   for session in receive_sessions {
       // Add scope as watcher for each P2P session
       if let TransferTarget::P2P { ref signalling_key, .. } = session.target {
           let mut scope = FindingScope::new(&signalling_key);
           scope.set_watcher(true);  // Join as watcher
           self.update_model(NearbyEvent::AddFindingScope(scope));
       }
   }
   ```

2. **`find_transfer_session()`** (line 160-179):
   - After adding session to model, add scope as watcher
   - Similar logic to above

**When user views session (Member mode):**

3. **`request_session_detail()`** (line 342-380):
   ```rust
   // Before requesting details, upgrade from watcher to member
   if let TransferTarget::P2P { ref signalling_key, .. } = session.target {
       let mut scope = FindingScope::new(&signalling_key);
       scope.set_watcher(false);  // Upgrade to member
       self.update_model(NearbyEvent::AddFindingScope(scope));
   }
   ```

This ensures:
- ✅ All receive sessions get online/offline updates (as watchers)
- ✅ Only actively viewed sessions participate as members
- ✅ Reduced overhead for inactive sessions

### 9. Update View Models
**File:** `shared/src/app/view_models/receive_session.rs:30-54`

Add to `ReceiveSessionViewModel`:
- `is_scope_online: bool` - indicates if P2P sender scope is online
- Derive from `TransferSession.target.scope.as_ref().map(|s| s.is_online()).unwrap_or(false)`
- UI will use this to show green (online) or gray (offline) dot

### 10. Update Repository Layer (if needed)
**File:** `shared/src/repository/transfer_session.rs`

- Ensure scope state persists across app restarts (if required)
- Update database schema if storing `FindingScope` state
- Handle migration for existing sessions

## Verification Steps

### Build Validation
```bash
# 1. Check shared package compiles
cargo check -p shared

# 2. Build WASM bindings for TypeScript
cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features typescript

# 3. Run tests (if available)
cargo test -p shared
```

### Runtime Testing
1. Start two devices on same network
2. Initiate P2P transfer session
3. Verify green dot appears when sender is online
4. Disconnect sender device/network
5. Verify dot turns gray when sender goes offline
6. Reconnect and verify dot returns to green

## Files to Modify
1. `shared/src/entities/finding_scope.rs` - Add state and watcher fields
2. `shared/src/entities/target.rs` - Update P2P variant structure (scope → scope_name + scope)
3. `shared/src/entities/transfer_session.rs` - Update P2P constructor and all pattern matches
4. `shared/src/app/nearby/module.rs` - Add ScopeStateUpdated event handler
5. `shared/src/protocol/webrtc/signalling.rs` - Handle scope_state_changed messages from server
6. `shared/src/app/operations/p2p.rs` - Add ScopeStateChanged to P2POperationOutput enum
7. `shared/src/app/nearby/command.rs` - Handle ScopeStateChanged events in start_nearby_server and handle_peer_connection
8. `shared/src/app/transfer/commands.rs` - Add scope as watcher on load, upgrade to member on view
9. `shared/src/app/view_models/receive_session.rs` - Expose is_scope_online state
10. Proto schema already updated: `libs/schema/proto/devlog/rpc-signalling/server.proto:74-82`

## Dependencies
- `schema` crate (for ScopeState protobuf enum)
- Ensure proto generation is up-to-date
- UI layer must implement visual indicator (out of scope for this plan)

## Notes
- The RPC signalling proto already defines `ScopeState` enum and `ScopeStateChanged` message
- This is a non-breaking change if we use `Option<FindingScope>` for backward compatibility
- Consider default state handling for scopes without recent updates (assume offline after timeout?)

## Watcher vs Member Architecture

### Purpose
Optimize resource usage by having two participation levels:

| Mode | When | Receives | Use Case |
|------|------|----------|----------|
| **Watcher** | Session loaded/added | Only state updates (online/offline) | Monitor availability without full connection |
| **Member** | User clicks to view | Full data transfer + state | Active participation in P2P transfer |

### Flow
1. App loads receive sessions → Join each P2P scope as **watcher**
2. User sees list with online/offline dots (green/gray)
3. User clicks session → Upgrade to **member** for that scope
4. Session closes → Downgrade back to **watcher** (or remove scope)

### Benefits
- ✅ Reduced connection overhead for inactive sessions
- ✅ Real-time status visibility without full handshake
- ✅ Scalable for many concurrent sessions
- ✅ Clear separation between monitoring and active participation