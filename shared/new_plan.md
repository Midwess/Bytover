# Implementation Plan: P2P Transfer Session with Password Protection

## Overview
Add UI controls to the People tab in the send board to allow users to start a password-protected P2P transfer session with nearby peers.

## Architecture Review

### Current Flow
- **Frontend**: React (web-next) uses WASM core bindings to communicate with Rust core
- **Core**: Rust shared library handles business logic, compiles to WASM
- **Events**: Transfer events defined in `shared/src/app/transfer/module.rs`
- **Commands**: Transfer commands in `shared/src/app/transfer/commands.rs`
- **P2P**: Nearby peer operations in `shared/src/app/nearby/command.rs` and `shared/src/app/operations/p2p.rs`

### Key Files
- `web-next/app/transfer/send_board.tsx` - SendBoard UI (lines 797-884: NearbySend component)
- `shared/src/app/transfer/module.rs` - Transfer events and module logic (line 62-65: StartP2PTransfer event already exists!)
- `shared/src/app/transfer/commands.rs` - Transfer command handlers
- `shared/src/app/nearby/command.rs` - Nearby peer session handling (lines 82-86: TODO comment about notifying peers)
- `shared/src/entities/transfer_session.rs` - Session entity (lines 192-209: p2p session constructor)
- `shared/src/app/operations/p2p.rs` - P2P operations (lines 23-26: SendSessionsNotification)

## Implementation Steps

### 1. **UI Updates - People Tab (web-next/app/transfer/send_board.tsx)**
   **File**: `web-next/app/transfer/send_board.tsx:797-884` (NearbySend function)

   **Changes needed**:
   - Add state for password input: `const [password, setPassword] = useState('')`
   - Add state for "require password" toggle: `const [requirePassword, setRequirePassword] = useState(false)`
   - Add password input field (similar to PublicSend component at line 686-688)
   - Add "Start Transfer" button
   - Button should trigger `TransferEventVariantStartP2PTransfer` with password
   - Disable controls when transfer is in progress
   - Show session status/progress after starting

   **UI Components to add**:
   ```tsx
   - Label for password section
   - Switch/Checkbox for "Require Password"
   - Input field (type="password") - only show if requirePassword is true
   - Button "Start Transfer" - triggers the transfer
   - Progress indicator when session is active
   - Cancel button when session is in progress
   ```

### 2. **Event Handling - Check if Already Implemented**
   **File**: `shared/src/app/transfer/module.rs:223-232`

   **Current Implementation** (StartP2PTransfer event):
   - ✅ Event already exists at line 62-65
   - ✅ Handler implemented at lines 223-232
   - ✅ Creates P2P session with `TransferSession::p2p()`
   - ✅ Adds session to model
   - ⚠️  **MISSING**: No authentication check (unlike StartPublicTransfer which checks user login at line 205-211)
   - ⚠️  **MISSING**: No session broadcasting to nearby peers
   - ⚠️  **MISSING**: No validation for selected resources

   **Changes needed**:
   - Add resource validation (check if `selected_resources.is_empty()`)
   - Add authentication check if password is set (optional, depends on requirements)
   - Trigger session broadcast after creating session
   - Add error handling and user feedback

### 3. **Session Broadcasting**
   **File**: `shared/src/app/nearby/command.rs:82-86`

   **Current State**:
   - There's a TODO comment at lines 82-86: `// TODO: Notify connected peer about current transfer P2P sessions`
   - Function `notify_peer_sessions` exists in `transfer/commands.rs:239-246`

   **Changes needed**:
   - Implement the TODO: After peer connects, notify them of active P2P send sessions
   - After creating new P2P session, broadcast to all connected peers
   - Use `P2POperation::send_sessions_notification` (from `app/operations/p2p.rs:23-26`)

   **Implementation approach**:
   ```rust
   // In transfer/module.rs StartP2PTransfer handler, after creating session:
   // 1. Get all connected peers from model.nearby.peers
   // 2. For each peer, call notify_peer_sessions with the new session
   // 3. Or create a new command that broadcasts to all peers
   ```

### 4. **Authentication Flow**
   **File**: `shared/src/app/authentication/command.rs:19-34`

   **Consideration**:
   - Public transfers require authentication (transfer/module.rs:205-211)
   - Should P2P transfers also require authentication when password protected?
   - Current implementation doesn't check authentication for P2P

   **Decision needed** (ask user or document):
   - If password is required, should user be authenticated?
   - Or is P2P transfer available to anyone nearby without account?

   **Suggested approach**:
   - P2P transfers work without authentication (current behavior)
   - Password is just for securing the specific transfer session
   - Keep it simple - no auth check for P2P

### 5. **Session State Management**
   **File**: `shared/src/app/transfer/module.rs:223-232`

   **Changes needed**:
   - Update `TransferSession::p2p()` to properly set `is_required_password` flag
   - Currently sets `is_required_password: false` hardcoded (line 205)
   - Should be: `is_required_password: password.is_some()`

   **File**: `shared/src/entities/transfer_session.rs:192-209`
   ```rust
   // Line 205 needs to change from:
   is_required_password: false
   // To:
   is_required_password: password.is_some()
   ```

### 6. **UI State Display - Show Active Session**
   **File**: `web-next/app/transfer/send_board.tsx:797-884`

   **Changes needed**:
   - Use `core.useTransferState()` to get nearby sessions (similar to cloud_session at line 649)
   - Filter for TransferType.Send sessions with target P2P
   - Display session info when active:
     - Session ID
     - Connected peers count
     - Transfer progress
     - Cancel button
   - Hide password input and start button when session is active
   - Show session list when there are active nearby send sessions

### 7. **Type Generation & Validation**
   **Commands to run**:
   ```bash
   # In shared directory:
   cargo check -p shared

   # Generate TypeScript types (if there's a build script):
   cargo build -p shared --target wasm32-unknown-unknown
   wasm-bindgen ... # (check existing build process)

   # In web-next directory:
   pnpm run type-check
   ```

## Detailed Implementation Checklist

### Phase 1: Rust Core Updates
- [ ] Fix `TransferSession::p2p()` to properly set `is_required_password` flag based on password presence
- [ ] Update `StartP2PTransfer` event handler in `transfer/module.rs`:
  - [ ] Add validation: check if `selected_resources.is_empty()`
  - [ ] Show toast message if no resources selected
  - [ ] Keep nearby_available check (line 223)
  - [ ] After adding session to model, trigger broadcast to nearby peers
- [ ] Implement peer notification in `nearby/command.rs`:
  - [ ] Complete TODO at line 82-86
  - [ ] Create helper function to broadcast session overview to all connected peers
- [ ] Add new command/event for broadcasting sessions to all peers (if needed)

### Phase 2: UI Updates (NearbySend Component)
- [ ] Add state variables:
  - [ ] `password` state
  - [ ] `requirePassword` state
  - [ ] Get nearby send sessions from `core.useTransferState()`
- [ ] Add UI controls:
  - [ ] "Require Password" switch/checkbox
  - [ ] Password input field (conditional - only if requirePassword is true)
  - [ ] "Start Transfer" button
  - [ ] Disable controls when session is active
- [ ] Add session display:
  - [ ] Show active session info if exists
  - [ ] Display transfer progress
  - [ ] Show connected peers
  - [ ] Add cancel button
- [ ] Wire up event:
  - [ ] On "Start Transfer" click, call `core.update(new AppEventVariantTransfer(new TransferEventVariantStartP2PTransfer(...)))`
  - [ ] Pass `nearby_available: true` (from checking if nearby server is running)
  - [ ] Pass password (or null if not required)

### Phase 3: Testing & Validation
- [ ] Run `cargo check -p shared` - ensure no errors
- [ ] Build shared to WASM and generate TypeScript types
- [ ] Run `pnpm run type-check` in web-next - ensure types are correct
- [ ] Manual testing:
  - [ ] Test starting session without password
  - [ ] Test starting session with password
  - [ ] Test that nearby peers see the session
  - [ ] Test password validation on receiver side
  - [ ] Test canceling session
  - [ ] Test with no resources selected (should show error)

### Phase 4: Edge Cases & Polish
- [ ] Handle case where nearby server is not running
- [ ] Handle case where no peers are connected (show appropriate message)
- [ ] Add loading states
- [ ] Add error handling for failed broadcasts
- [ ] Test on mobile view (the component already has mobile support)
- [ ] Ensure password field uses secure input (type="password")
- [ ] Add fake password input field to prevent autofill issues (see PublicSend line 192)

## Files to Modify

### Rust (shared/)
1. `src/entities/transfer_session.rs` - Fix is_required_password flag (line 205)
2. `src/app/transfer/module.rs` - Update StartP2PTransfer handler (lines 223-232)
3. `src/app/nearby/command.rs` - Implement peer notification TODO (lines 82-86)
4. `src/app/transfer/commands.rs` - May need to add broadcast helper function

### TypeScript (web-next/)
1. `app/transfer/send_board.tsx` - Update NearbySend component (lines 797-884)

## API/Event Flow

```
User clicks "Start Transfer" in People tab
  ↓
Frontend: TransferEventVariantStartP2PTransfer { nearby_available, password }
  ↓
Core: transfer/module.rs - StartP2PTransfer handler
  ↓ (validate resources exist)
  ↓ (create session with TransferSession::p2p())
  ↓ (add session to model)
  ↓
Core: Broadcast session overview to all connected peers
  ↓ (for each peer in model.nearby.peers)
  ↓ (P2POperation::send_sessions_notification)
  ↓
Peer receives session notification
  ↓
Peer UI updates to show new available session
  ↓
Session is ready for peer to view and download
```

## Questions to Resolve

1. **Authentication**: Should P2P transfers with password require user authentication?
   - **Recommendation**: No - keep P2P simple and local-only

2. **Multiple sessions**: Can user have multiple active P2P send sessions?
   - **Current**: Looks like only one cloud session is shown, but model supports multiple
   - **Recommendation**: Support multiple P2P sessions, show list

3. **Session persistence**: Should P2P send sessions be persisted to DB?
   - **Current**: Only receive sessions are persisted (transfer/commands.rs:165)
   - **Recommendation**: No persistence for send sessions (temporary, local only)

4. **Broadcast timing**: When to broadcast sessions?
   - On peer connect (TODO at nearby/command.rs:82-86)
   - On new session creation
   - Both?
   - **Recommendation**: Both - ensure peers always see current sessions

## Success Criteria

- [ ] User can see password input and "Start Transfer" button in People tab
- [ ] Clicking button creates a P2P session with selected resources
- [ ] Session is broadcast to all connected nearby peers
- [ ] Peers can see the session in their receive board
- [ ] Password protection works (if set)
- [ ] User can cancel the session
- [ ] No TypeScript or Rust compilation errors
- [ ] `cargo check -p shared` passes
- [ ] TypeScript types are generated correctly
