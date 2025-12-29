# Implementation Plan: Receive Session Select Flow

## Overview
Implement explicit session selection tracking to control when session details are requested from peers, with enhanced loading states and retry functionality.

---

## Part 1: Backend - Session Selection Tracking

### 1.1 Add Selected Session to TransferModel and ViewModel
**File:** `shared/src/app/transfer/module.rs:32-37` (TransferModel)

**Changes:**
```rust
pub struct TransferModel {
    selected_method: TransferMethodSelection,
    pub sessions: Vec<TransferSession>,
    keywords: String,
    selected_receive_session_id: Option<u64>  // ADD THIS
}
```

**File:** `shared/src/app/transfer/module.rs:39-46` (TransferViewModel)

**Changes:**
```rust
pub struct TransferViewModel {
    transfer_method: TransferMethodSelection,
    received_sessions: Vec<ReceiveSessionViewModel>,
    received_cloud_sessions: Vec<ReceiveSessionViewModel>,
    cloud_session: Option<CloudSession>,
    p2p_sessions: Vec<CloudSession>,
    selected_session: Option<ReceiveSessionViewModel>  // ADD THIS
}
```

**Purpose:**
- TransferModel tracks the selected session ID (internal state)
- TransferViewModel exposes the full selected session (UI state)
- Single source of truth in backend, no separate frontend Observable needed

---

### 1.2 Track Selection on ViewSession Event
**File:** `shared/src/app/transfer/module.rs` (ViewSession handler, ~lines 348-393)

**Changes:**
```rust
TransferEvent::ViewSession { session_id, password, transfer_type } => {
    // ADD: Track selected session
    if transfer_type == TransferType::Receive {
        model.transfer.selected_receive_session_id = Some(session_id.order_id.unwrap_or(0));
    }

    // Existing logic continues...
    match session.target {
        TransferTarget::P2P { ... } => {
            // existing P2P handling
        }
        TransferTarget::Internet { ... } => {
            // existing Internet handling
        }
    }
}
```

**Purpose:** Mark session as selected when user explicitly clicks to view it.

---

### 1.3 Conditional Auto-Request in Nearby Module
**File:** `shared/src/app/nearby/module.rs:92-144` (PeerUpdated handler)

**Current Logic:**
```rust
// When peer connects, automatically request details
if from_peer.is_none() && is_peer_owned {
    session.owner_connected(peer.clone());
    return Command::event(AppEvent::Transfer(
        TransferEvent::RequestSessionDetail { ... }
    ))
}
```

**Modified Logic:**
```rust
if from_peer.is_none() && is_peer_owned {
    session.owner_connected(peer.clone());

    // ONLY request details if session is selected
    let is_selected = model.transfer.selected_receive_session_id
        == Some(session.order_id);

    if is_selected {
        return Command::event(AppEvent::Transfer(
            TransferEvent::RequestSessionDetail {
                peer_id: peer.id.clone(),
                order_id: session.order_id,
                password: session.password.clone()
            }
        ))
    }
}
```

**Purpose:** Only fetch session details when user has actively selected the session, not automatically for all discovered sessions.

---

## Part 2: Backend - Enhanced Loading States

### 2.1 Add Loading State Context to ViewModel
**File:** `shared/src/app/view_models/receive_session.rs:29-52`

**Changes:**
```rust
pub struct ReceiveSessionViewModel {
    // ... existing fields ...
    pub is_loading: bool,
    pub loading_status: Option<String>,  // ADD THIS - e.g. "Signalling", "Authorizing..."
    pub error_message: Option<String>,   // ADD THIS - for retry UI
    // ... rest of fields ...
}
```

**Purpose:** Provide contextual loading text and error state for UI.

---

### 2.2 Compute Loading Status and Selected Session in View Function
**File:** `shared/src/app/transfer/module.rs:471-609` (view() function)

**Changes in session mapping (~line 520-560):**
```rust
// Helper function to compute loading state for a session
fn compute_session_loading_state(session: &TransferSession) -> (bool, Option<String>, Option<String>) {
    let (is_loading, loading_status) = match &session.target {
        TransferTarget::P2P { from_peer, connection_state, .. } => {
            match connection_state {
                P2PConnectionState::NotConnected | P2PConnectionState::Connecting
                    if from_peer.is_none() => {
                    (true, Some("Signalling".to_string()))
                },
                P2PConnectionState::Connected if session.resources.is_empty() => {
                    (true, Some("Authorizing...".to_string()))
                },
                P2PConnectionState::Failed(_) => {
                    (false, None)  // error_message handles this
                },
                _ => (false, None)
            }
        },
        TransferTarget::Internet { .. } if session.resources.is_empty() => {
            (true, Some("Loading...".to_string()))
        },
        _ => (false, None)
    };

    let error_message = match &session.target {
        TransferTarget::P2P { connection_state, .. } => {
            if let P2PConnectionState::Failed(msg) = connection_state {
                Some(msg.clone())
            } else {
                None
            }
        },
        _ => None
    };

    (is_loading, loading_status, error_message)
}

// In view() function, when building ReceiveSessionViewModel
let (is_loading, loading_status, error_message) = compute_session_loading_state(&session);

let session_vm = ReceiveSessionViewModel {
    // ... existing fields ...
    is_loading,
    loading_status,
    error_message,
    // ... rest of fields ...
};

// After building all session ViewModels, find the selected one
let selected_session = model.selected_receive_session_id.and_then(|selected_id| {
    received_sessions.iter()
        .chain(received_cloud_sessions.iter())
        .find(|s| s.id == selected_id.to_string())
        .cloned()
});

TransferViewModel {
    transfer_method: model.selected_method.clone(),
    received_sessions,
    received_cloud_sessions,
    cloud_session,
    p2p_sessions,
    selected_session  // ADD THIS
}
```

**Purpose:**
- Compute loading status and error messages for all sessions
- Find and expose the selected session in the ViewModel
- Frontend can simply read `transferState.selected_session`

---

### 2.3 Enhance TransferSessionStatus Display
**File:** `shared/src/entities/transfer_session.rs:21-47`

**Option A: Add LoadingContext variant (Recommended)**
```rust
pub enum TransferSessionStatus {
    Initializing { context: Option<String> },  // CHANGE THIS
    InProgress { bytes_per_second: u64, percentage: f64 },
    Success,
    Failed(String),
    Canceled
}

impl Display for TransferSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TransferSessionStatus::Initializing { context } => {
                if let Some(ctx) = context {
                    write!(f, "{}", ctx)
                } else {
                    write!(f, "Initializing...")
                }
            },
            // ... rest unchanged ...
        }
    }
}
```

**Option B: Keep existing, use loading_status field instead**
- Simpler: no need to update all Initializing usages
- Loading context lives only in ViewModel

**Recommendation:** Use Option B to minimize changes.

---

## Part 3: Frontend - Simplify State Management & Add Retry UI

### 3.0 Remove Frontend-Managed selectedSession Observable
**File:** `web-next/wasm/wasm_core.ts`

**Remove these (lines 97, 101-111, 171-173, 584-589):**
```typescript
// DELETE THIS:
selectedSession: Observable<ReceiveSessionViewModel> = new Observable()

public useSelectedSession() {
    // DELETE entire method - will be replaced
}

public updateSelectedSession(session: ReceiveSessionViewModel) {
    // DELETE entire method - no longer needed
}

// In updateView(), DELETE this selectedSession sync logic:
const selectedSession = this.selectedSession.get()
if (selectedSession) {
    const newSession = viewModel.transfer?.received_sessions.find(it => it.id === selectedSession.id) ||
        viewModel.transfer?.received_cloud_sessions.find(it => it.id === selectedSession.id)
    this.selectedSession.set(newSession)
}
```

**Replace with simpler hook:**
```typescript
public useSelectedSession() {
    const [selectedSession, setSelectedSession] = useState<ReceiveSessionViewModel | undefined>()

    useEffect(() => {
        return this.transferState.subscribe((transferState) => {
            if (!isEqual(selectedSession, transferState?.selected_session)) {
                setSelectedSession(transferState?.selected_session)
            }
        })
    }, [selectedSession])

    return selectedSession
}
```

**Purpose:**
- Remove duplicate state management in frontend
- Single source of truth: backend manages selection via selected_receive_session_id
- Frontend simply reads from transferState.selected_session

---

### 3.1 Update Session Selection Logic in receive_board.tsx
**File:** `web-next/app/transfer/receive_board.tsx`

**Find where sessions are clicked (likely in TransferSession component or session list):**

**Replace:**
```typescript
// OLD: Manual state update
onClick={() => {
    core.updateSelectedSession(item)
    // ...
}}
```

**With:**
```typescript
// NEW: Trigger ViewSession event (backend handles selection)
onClick={() => {
    core.update(new TransferEventVariantViewSession(
        item.password ?? undefined,
        new TransferSessionId(
            new TransferType(item.is_cloud ? "Internet" : "P2P"),
            item.id
        )
    ))
    // No need to manually update selected session - backend will do it
}}
```

**Purpose:** Selection is now handled by the backend when ViewSession event fires.

---

### 3.2 Display Loading Status
**File:** `web-next/app/transfer/receive_board.tsx` (ContentBoard component, ~lines 210-221)

**Current:**
```typescript
if (isLoading) {
  return <div className="...">
    <div className="spinner" />
    {loadMessage.message && <p>{loadMessage.message}</p>}
  </div>
}
```

**Enhanced:**
```typescript
if (isLoading || selectedSession?.is_loading) {
  return <div className="...">
    <div className="spinner" />

    {/* Show contextual status */}
    {selectedSession?.loading_status && (
      <p className="text-muted-foreground">
        {selectedSession.loading_status}
      </p>
    )}

    {/* Show error with retry */}
    {selectedSession?.error_message && (
      <div className="flex flex-col gap-2">
        <p className="text-red-500">{selectedSession.error_message}</p>
        <Button onClick={handleRetry}>Retry</Button>
      </div>
    )}

    {loadMessage.message && <p>{loadMessage.message}</p>}
  </div>
}
```

---

### 3.3 Implement Retry Handler
**File:** `web-next/app/transfer/receive_board.tsx`

**Add handler:**
```typescript
const handleRetry = () => {
  if (!selectedSession) return;

  // Re-trigger ViewSession event (same as clicking the session)
  core.update(new TransferEventVariantViewSession(
    selectedSession.password ?? undefined,
    new TransferSessionId(
      new TransferType(selectedSession.is_cloud ? "Internet" : "P2P"),
      selectedSession.id
    )
  ));
};
```

**Purpose:** Allow user to retry failed session loads without refreshing page.

---

### 3.4 Update TypeScript Types (Auto-generated)
**Ensure new fields appear in generated types:**
- `ReceiveSessionViewModel`: `loading_status?: string`, `error_message?: string`
- `TransferViewModel`: `selected_session?: ReceiveSessionViewModel`

Will be generated in step 5.2 below.

---

## Part 4: Data Flow Summary

```
USER CLICKS SESSION
  ↓
TransferEvent::ViewSession
  ↓
selected_receive_session_id = Some(order_id)
  ↓
┌─────────────────────────────────────┐
│ P2P PATH                            │
├─────────────────────────────────────┤
│ 1. Add FindingScope (if not found)  │
│ 2. Wait for peer...                 │
│    loading_status = "Signalling"    │
│                                     │
│ PEER CONNECTS                       │
│   ↓                                 │
│ NearbyEvent::PeerUpdated            │
│   ↓                                 │
│ Check: is_selected?                 │
│   YES → RequestSessionDetail        │
│   NO  → Just set owner, no request  │
│                                     │
│ REQUESTING DETAILS                  │
│   loading_status = "Authorizing..." │
│                                     │
│ DETAILS RECEIVED                    │
│   is_loading = false                │
│   Show resources                    │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ INTERNET PATH                       │
├─────────────────────────────────────┤
│ 1. Call view_public_session()       │
│    loading_status = "Loading..."    │
│                                     │
│ 2. Subscribe to session stream      │
│                                     │
│ 3. Resources arrive                 │
│    is_loading = false               │
│    Show resources                   │
└─────────────────────────────────────┘

ERROR HANDLING:
  connection_state = Failed(msg)
    ↓
  error_message = Some(msg)
  is_loading = false
    ↓
  UI shows error + Retry button
    ↓
  User clicks Retry
    ↓
  Re-trigger ViewSession event
```

---

## Part 5: Testing & Build

### 5.1 Verify Compilation
```bash
cargo check -p shared
```

**Expected:** No errors, all new fields compile correctly.

---

### 5.2 Generate TypeScript Types
```bash
cargo build -p shared_types \
  --target wasm32-unknown-unknown \
  --no-default-features \
  --features typescript
```

**Expected:**
- New fields `loading_status` and `error_message` appear in `ReceiveSessionViewModel` TS interface
- File location: `web-next/wasm/types/` (or similar)

---

### 5.3 Manual Test Cases

**Test Case 1: P2P Session - Signalling**
1. Start sender with P2P session
2. Open receiver, see session in list (don't click yet)
3. Verify: No auto-request sent
4. Click session → should show "Signalling"
5. Sender goes online → should show "Authorizing..."
6. Resources load → shows files/media

**Test Case 2: P2P Session - Retry on Failure**
1. Click session while sender offline
2. Wait for connection timeout/failure
3. Verify: error_message displayed with Retry button
4. Click Retry → re-triggers ViewSession
5. If sender online → should succeed

**Test Case 3: Internet Session - Loading**
1. Click internet session
2. Should show "Loading..."
3. Resources arrive → display content

**Test Case 4: Multiple Sessions**
1. Have 3 P2P sessions discovered
2. Click session A → only session A requests details
3. Sessions B and C remain in discovered state (no auto-request)
4. Click session B → session B now requests details

---

## Implementation Checklist

### Backend (shared/)
- [ ] Add `selected_receive_session_id: Option<u64>` to TransferModel
- [ ] Add `selected_session: Option<ReceiveSessionViewModel>` to TransferViewModel
- [ ] Set selected_receive_session_id in ViewSession handler
- [ ] Add conditional check in nearby/module.rs PeerUpdated handler
- [ ] Add `loading_status: Option<String>` to ReceiveSessionViewModel
- [ ] Add `error_message: Option<String>` to ReceiveSessionViewModel
- [ ] Create helper function to compute loading state
- [ ] Compute selected_session in view() by finding session matching selected_receive_session_id
- [ ] Run `cargo check -p shared`

### Frontend (web-next/)
- [ ] Remove `selectedSession` Observable from wasm_core.ts
- [ ] Remove `updateSelectedSession()` method from wasm_core.ts
- [ ] Simplify `useSelectedSession()` to read from transferState.selected_session
- [ ] Remove selectedSession sync logic in updateView()
- [ ] Update session click handlers to trigger ViewSession event only
- [ ] Add loading_status display in ContentBoard loading UI
- [ ] Add error_message display with styling
- [ ] Implement Retry button in error state
- [ ] Create handleRetry function to re-trigger ViewSession
- [ ] Test all loading states (Signalling, Authorizing, Loading)
- [ ] Test retry functionality

### Build & Verification
- [ ] Run `cargo check -p shared` - ensure passes
- [ ] Run TypeScript build command to generate types
- [ ] Verify new fields in generated TS types
- [ ] Manual test P2P session selection flow
- [ ] Manual test retry on connection failure
- [ ] Manual test internet session loading
- [ ] Verify no auto-requests for non-selected sessions

---

## File Reference

| File | Lines | Changes |
|------|-------|---------|
| `shared/src/app/transfer/module.rs` | 32-37 | Add selected_receive_session_id to TransferModel |
| `shared/src/app/transfer/module.rs` | 39-46 | Add selected_session to TransferViewModel |
| `shared/src/app/transfer/module.rs` | ~348-393 | Track selection in ViewSession handler |
| `shared/src/app/transfer/module.rs` | ~520-560 | Compute loading states + selected_session |
| `shared/src/app/nearby/module.rs` | 92-144 | Conditional auto-request based on selection |
| `shared/src/app/view_models/receive_session.rs` | 29-52 | Add loading_status, error_message fields |
| `web-next/wasm/wasm_core.ts` | 97, 101-111, 171-173, 584-589 | Remove selectedSession Observable, simplify hook |
| `web-next/app/transfer/receive_board.tsx` | various | Update click handlers to use ViewSession event |
| `web-next/app/transfer/receive_board.tsx` | ~210-221 | Enhanced loading UI with status/error |
| `web-next/app/transfer/receive_board.tsx` | new | Add handleRetry function |

---

## Notes

- **Architecture Improvement:** Selected session now managed in backend, not frontend
  - Single source of truth: `TransferModel.selected_receive_session_id`
  - Exposed via `TransferViewModel.selected_session`
  - Frontend simply reads from transferState (no separate Observable)
- **Breaking Change:** Adding fields to TransferModel requires Default impl update
- **Backward Compatibility:** New Optional fields in ViewModel are non-breaking
- **Performance:** Conditional request reduces unnecessary network calls
- **UX Improvement:** Clear loading states improve user understanding of what's happening
- **Simplified Frontend:** No manual state synchronization, everything flows through normal view update cycle