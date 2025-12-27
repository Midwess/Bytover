# Implementation Plan: Connect to Transfer Session with Stop Control

## Overview
Enable proper peer-to-peer connection flow when viewing transfer session details, with UI controls for stopping active transfers.

**Note**: Peer detail request/response protocol is ALREADY IMPLEMENTED in `peer.rs` and `commands.rs`. No new message types needed.

---

## 1. Add Scope When Viewing Session (If Peer Not Connected)

### Implementation
**File**: `shared/src/app/transfer/module.rs`

Modify existing `ViewSession` handler to add scope for peer discovery when peer is not yet connected.

### Logic
- When user views a P2P receive session
- If `from_peer.is_none()` → Add scope to finding_scopes
- If `from_peer.is_some()` AND `resources.is_empty()` → Request details
- If resources already exist → Skip request (no duplicate fetches)

---

## 2. Auto-Request Details When Peer Connects

### Implementation
**File**: `shared/src/app/transfer/module.rs`

Modify `PeerUpdated` handler to automatically request session details when a peer is first assigned.

### Logic
- When peer connects and is assigned to session (`from_peer` changes from None to Some)
- Automatically trigger `RequestSessionDetail` event
- Uses existing detail request infrastructure (already implemented)

---

## 3. Handle Peer Disconnect Cleanup (Receiver Side)

### Implementation
**Files**:
- `shared/src/app/transfer/module.rs` - Add `PeerDisconnected` event
- `shared/src/app/nearby/command.rs` - Notify on disconnect

### Logic for Receiver Sessions
When peer disconnects:
1. Set `from_peer` to `None`
2. Clear `resources` and `progress` lists
3. Remove scope from `finding_scopes`
4. Keep session in list (don't delete) - user can retry when peer reconnects
5. Update UI to show disconnected state

---

## 4. Gray Stop Button UI

### Implementation
**Files**:
- `web-next/app/transfer/receive_board.tsx`
- `web-next/app/transfer/send_board.tsx`

### UI Spec
- **Color**: Gray/neutral (not red/destructive)
- **Icon**: Square stop icon
- **Position**: Next to CircleProgress during active transfer
- **Condition**: Only show when `session.is_in_progress === true`
- **Confirmation**: "Are you sure you want to stop this transfer?"
- **Action**: Calls existing `CancelTransfer` event

---

## 5. File Summary

### Rust Core Changes (3 files)
1. `shared/src/app/transfer/module.rs`:
   - Modify `ViewSession` handler - add scope if peer is None
   - Modify `PeerUpdated` handler - auto-request details when peer connects
   - Add `PeerDisconnected` event and handler

2. `shared/src/app/nearby/command.rs`:
   - Notify transfer module on peer disconnect

### Web UI Changes (2 files)
1. `web-next/app/transfer/receive_board.tsx` - Gray stop button
2. `web-next/app/transfer/send_board.tsx` - Gray stop button

### Schema
- No changes needed - all messages already defined in `request.proto`

---

## 6. Implementation Order

1. **Phase 1**: Modify ViewSession - add scope if peer is None
2. **Phase 2**: Modify PeerUpdated - auto-request details on connect
3. **Phase 3**: Add PeerDisconnected event and cleanup logic
4. **Phase 4**: Add gray stop button UI
5. **Phase 5**: Build and test (`cargo build` to regenerate TypeScript types)

---

## 7. CODE CHANGES

### 7.1 Phase 1: Add Scope When Viewing Session

**File**: `shared/src/app/transfer/module.rs`

**Location**: Line ~377, modify `TransferEvent::ViewSession` handler

**Change**: Add scope if peer is None, only request if resources empty

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
        TransferTarget::P2P { signalling_key, from_peer, .. } => {
            // ADD THIS BLOCK: Add scope for peer discovery when peer is not connected
            if from_peer.is_none() {
                let scope = FindingScope::Global(signalling_key.clone());
                if !model.nearby.shared_context.finding_scopes.contains(&scope) {
                    log::info!("Adding scope {} for session {} - peer not connected", signalling_key, session.order_id);
                    Command::update_model(NearbyEvent::AddFindingScope(scope));
                }
            }
            // END NEW BLOCK

            let Some(peer_id) = session.peer_id() else {
                return Command::new(|it| async move {
                    DialogOperation::toast("Waiting for connection...".to_string())
                        .into_future(it.clone()).await;
                });
            };

            // ADD THIS CHECK: Only request details if we don't have them yet
            if session.resources.is_empty() {
                Command::handle_result(move |it| async move {
                    it.app().request_session_detail(peer_id, session.order_id, password).await
                })
            } else {
                // Already have details, just render
                Command::done()
            }
            // END NEW CHECK
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

### 7.2 Phase 2: Auto-Request Details When Peer Connects

**File**: `shared/src/app/transfer/module.rs`

**Location**: Line ~265, modify `TransferEvent::PeerUpdated` handler

**Change**: Send detail request after assigning peer

```rust
TransferEvent::PeerUpdated { peer } => {
    let mut peer_just_connected = false;
    let mut session_order_id = 0;

    for session in model.transfer.sessions.iter_mut() {
        if session.transfer_type != TransferType::Receive {
            continue;
        }

        if let TransferTarget::P2P {
            ref mut from_peer,
            ref scope,
            ..
        } = session.target
        {
            if from_peer.is_none() && peer.scopes.contains(scope) {
                log::info!(
                    "Updating P2P session {} with peer {} (scope: {})",
                    session.order_id,
                    peer.id,
                    scope
                );

                *from_peer = Some(peer.clone());
                // ADD THESE LINES
                peer_just_connected = true;
                session_order_id = session.order_id;
                // END NEW LINES

                break;
            }
        }
    }

    // ADD THIS BLOCK: Send detail request when peer first connects
    if peer_just_connected {
        log::info!("Sending detail request for session {} to peer {}", session_order_id, peer.id);
        return Command::update_self(TransferEvent::RequestSessionDetail {
            peer_id: peer.id,
            order_id: session_order_id,
            password: None
        }).then_render();
    }
    // END NEW BLOCK

    Command::render()
}
```

---

### 7.3 Phase 3: Handle Peer Disconnect

**File**: `shared/src/app/transfer/module.rs`

**Location 1**: Line ~50 (in TransferEvent enum), add new event variant

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransferEvent {
    // ... existing variants ...
    PeerUpdated {
        peer: Peer
    },
    // ADD THIS:
    PeerDisconnected {
        peer_id: String
    },
    // END NEW

    #[serde(skip)]
    ModelEvent(TransferSessionModelEvent)
}
```

**Location 2**: After `PeerUpdated` handler (line ~293), add new handler

```rust
// ADD THIS ENTIRE HANDLER:
TransferEvent::PeerDisconnected { peer_id } => {
    log::info!("Handling peer disconnect for peer: {}", peer_id);

    let mut scope_to_remove: Option<FindingScope> = None;

    for session in model.transfer.sessions.iter_mut() {
        if session.transfer_type != TransferType::Receive {
            continue;
        }

        if let TransferTarget::P2P {
            ref mut from_peer,
            ref signalling_key,
            ..
        } = session.target
        {
            // Check if this session was connected to the disconnected peer
            if let Some(ref peer) = from_peer {
                if peer.id == peer_id {
                    log::info!("Cleaning up session {} after peer disconnect", session.order_id);

                    // 1. Mark peer to None
                    *from_peer = None;

                    // 2. Clean up resources and progress
                    session.resources.clear();
                    session.progress.clear();

                    // 3. Remember scope to remove
                    scope_to_remove = Some(FindingScope::Global(signalling_key.clone()));

                    break;
                }
            }
        }
    }

    // 4. Remove scope from finding_scopes
    if let Some(scope) = scope_to_remove {
        log::info!("Removing scope {:?} after peer disconnect", scope);
        return Command::update_model(NearbyEvent::RemoveFindingScope(scope)).then_render();
    }

    Command::render()
}
// END NEW HANDLER
```

**File**: `shared/src/app/nearby/command.rs`

**Location**: Line ~142, in `handle_peer_connection` method

**Change**: Notify transfer module about peer disconnect

```rust
pub async fn handle_peer_connection(&self, peer: Peer) {
    let request = P2POperation::PeerEvents(peer.id.clone());
    let mut stream = self.stream_from_shell(request.into());

    while let Some(output) = stream.next().await {
        match output {
            CoreOperationOutput::P2P(P2POperationOutput::PeerDisconnected()) => {
                log::info!("Peer disconnected: {}", peer.id);
                // ADD THIS LINE:
                self.notify_event(TransferEvent::PeerDisconnected { peer_id: peer.id.clone() });
                // END NEW LINE
                break;
            }
            // ... rest of match arms ...
        }
    }

    self.notify_event(NearbyEvent::UpdateNearbyPeers {
        new_peer: vec![],
        removed: vec![peer.clone()]
    });
}
```

---

### 7.4 Phase 4: Gray Stop Button UI

**File**: `web-next/app/transfer/receive_board.tsx`

**Location**: In TransferSession component, where CircleProgress is displayed

**Change**: Add gray stop button next to progress

```typescript
{session.is_in_progress && (
  <div className="flex items-center gap-2">
    <CircleProgress
      value={session.progress}
      onClick={() => {/* existing click handler */}}
    />
    {/* ADD THIS: */}
    <button
      onClick={async () => {
        const confirmed = confirm("Are you sure you want to stop this transfer?");
        if (confirmed) {
          core.update(new AppEventVariantTransfer(
            new TransferEventVariantCancelTransfer(
              BigInt(session.id),
              new TransferTypeVariantReceive()
            )
          ));
        }
      }}
      className="p-2 rounded-lg bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
      title="Stop transfer"
    >
      <Square className="w-4 h-4 text-gray-600 dark:text-gray-300" />
    </button>
    {/* END NEW */}
  </div>
)}
```

**File**: `web-next/app/transfer/send_board.tsx`

**Location**: In CloudSession component

**Change**: Add gray stop button for send sessions

```typescript
{cloudSession.is_in_progress && (
  <div className="flex items-center gap-2">
    <CircleProgress value={cloudSession.progress} />
    {/* ADD THIS: */}
    <button
      onClick={async () => {
        const confirmed = confirm("Are you sure you want to stop this transfer?");
        if (confirmed) {
          core.update(new AppEventVariantTransfer(
            new TransferEventVariantCancelTransfer(
              BigInt(cloudSession.session_id),
              new TransferTypeVariantSend()
            )
          ));
        }
      }}
      className="p-2 rounded-lg bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
      title="Stop transfer"
    >
      <Square className="w-4 h-4 text-gray-600 dark:text-gray-300" />
    </button>
    {/* END NEW */}
  </div>
)}
```

---

## 8. Testing Checklist

### Integration Tests
- [ ] View receive session with no peer → Scope added to finding_scopes
- [ ] Peer connects → Detail request sent automatically
- [ ] ViewSession with existing resources → No duplicate request sent
- [ ] Stop button click → Session removed, scope removed
- [ ] Peer disconnect (receiver) → Peer set to None, resources cleared, scope removed

### UI Tests
- [ ] Stop button only visible when `is_in_progress === true`
- [ ] Button shows confirmation dialog
- [ ] Session removed from list after stop
- [ ] Stop button is gray, not red

### Edge Cases
- [ ] ViewSession when peer already connected
- [ ] ViewSession with resources already loaded
- [ ] Stop transfer at 0%, 50%, 99% progress
- [ ] Peer disconnect at 0% → Resources cleared, can reconnect
- [ ] Peer disconnect at 50% → Resources cleared, session stays in list
