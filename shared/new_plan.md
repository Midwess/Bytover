# Implementation Plan: Password-Protected Nearby Sessions on Receive Board

## Overview
Add password input UI and authentication flow for nearby (P2P) sessions on the receive board, similar to how cloud sessions currently work.

## Current State Analysis

### What Already Exists ✅

1. **View Model Fields** (`shared/src/app/view_models/receive_session.rs`):
   - `password_required: bool` (line 33) - indicates if session needs password
   - `is_authenticated: bool` (line 34) - indicates if user has entered correct password
   - `has_details: bool` (line 35) - indicates if session details (resources) are loaded

2. **View Model Population** (`shared/src/app/transfer/module.rs:587-593`):
   - `password_required` is set from `TransferTarget::P2P { is_required_password, .. }`
   - `is_authenticated = has_details` (authenticated when resources are loaded)

3. **Cloud Session Password UI** (`web-next/app/transfer/receive_board.tsx:182-208`):
   - Working password input component for cloud sessions
   - Can be used as a template for nearby sessions

4. **Backend Events**:
   - `RequestSessionDetail` event exists (module.rs:96-100)
   - Handler in `commands.rs:287-321` that calls `P2POperation::ViewSessionDetail`
   - Password validation in sender's `handle_view_session_request` (commands.rs:248-285)

### What's Missing ⚠️

1. **No password UI for nearby sessions** - currently only cloud sessions show password input
2. **No automatic session detail request** - nearby sessions don't auto-load like cloud sessions do
3. **Session click doesn't handle nearby sessions properly** - fires wrong event type
4. **No differentiation between cloud and nearby** in ContentBoard component

## Implementation Plan

### Phase 1: Backend - Ensure View Model Correctness

**File**: `shared/src/app/transfer/module.rs:587-593`

**Current code:**
```rust
let password_required = match &it.target {
    TransferTarget::P2P { is_required_password, .. } => *is_required_password,
    _ => false
};

let has_details = !it.resources.is_empty();
let is_authenticated = has_details;
```

**Analysis:**
- ✅ `password_required` correctly reads from `is_required_password` flag
- ✅ `is_authenticated` correctly checks if details are loaded
- ⚠️  Need to verify this works with password validation on sender side

**Action**: No changes needed, but verify during testing

---

### Phase 2: Frontend - Update ContentBoard Component

**File**: `web-next/app/transfer/receive_board.tsx:117-228`

#### Step 2.1: Add State for Nearby Session Password

**Location**: Inside `ContentBoard` function (after line 127)

**Add:**
```typescript
const [nearbyPassword, setNearbyPassword] = useState<string>('')
```

**Reasoning**: Separate state from `enteredPassword` (which is for cloud sessions) to avoid conflicts

---

#### Step 2.2: Add Auto-Load Logic for Nearby Sessions

**Location**: After the cloud session useEffect (after line 168)

**Add:**
```typescript
useEffect(() => {
    if (selectedSession && selectedSession instanceof ReceiveSessionViewModel) {
        const nearby = selectedSession as ReceiveSessionViewModel

        // Auto-load if no password required
        if (!nearby.password_required && !nearby.is_authenticated) {
            core.update(new AppEventVariantTransfer(
                new TransferEventVariantRequestSessionDetail(
                    nearby.peer_id,  // Need to add peer_id to ReceiveSessionViewModel
                    BigInt(nearby.id),
                    null  // no password
                )
            ))
        }
    }
}, [selectedSession?.id])
```

**Issue Identified**: ⚠️ `ReceiveSessionViewModel` doesn't have `peer_id` field!

**Fix Required**: Add `peer_id` to `ReceiveSessionViewModel` in Rust

---

#### Step 2.3: Add Password Input UI for Nearby Sessions

**Location**: After cloud session password check (after line 209), before the `if (!selectedSession)` check

**Add:**
```typescript
// Handle nearby session password requirement
if (selectedSession instanceof ReceiveSessionViewModel) {
    const nearby = selectedSession as ReceiveSessionViewModel

    if (nearby.password_required && !nearby.is_authenticated) {
        return (
            <div className="text-foreground w-full h-full flex flex-col justify-center items-center gap-2">
                <div className="w-[50%] flex flex-col gap-4">
                    <p className="text-muted-foreground flex flex-row items-center">
                        <Image
                            alt="lock"
                            width={10}
                            height={10}
                            className="w-7 text-white bg-muted p-1.5 rounded-lg mr-2 h-7"
                            src="/lock.svg"
                            color="white"
                        />
                        This session is password protected
                    </p>
                    <input type="password" name="fake-password" style={{ display: 'none' }} />
                    <Input
                        className="h-10"
                        placeholder="Enter password"
                        value={nearbyPassword}
                        onChange={(e) => setNearbyPassword(e.target.value)}
                        type="password"
                        onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                                requestNearbySessionDetail()
                            }
                        }}
                    />
                    <Button
                        onClick={requestNearbySessionDetail}
                        className="w-fit bg-foreground"
                    >
                        Continue
                    </Button>
                </div>
            </div>
        )
    }
}

// Helper function to request session detail
const requestNearbySessionDetail = () => {
    if (!(selectedSession instanceof ReceiveSessionViewModel)) return

    const nearby = selectedSession as ReceiveSessionViewModel
    core.update(new AppEventVariantTransfer(
        new TransferEventVariantRequestSessionDetail(
            nearby.peer_id,  // Need peer_id in view model
            BigInt(nearby.id),
            nearbyPassword || null
        )
    ))
}
```

---

### Phase 3: Backend - Add peer_id to ReceiveSessionViewModel

**File**: `shared/src/app/view_models/receive_session.rs:28-44`

**Current:**
```rust
pub struct ReceiveSessionViewModel {
    pub id: String,
    pub peer_avatar: AvatarViewModel,
    pub peer_name: String,
    pub peer_description: String,
    pub password_required: bool,
    pub is_authenticated: bool,
    pub has_details: bool,
    // ... other fields
}
```

**Add field:**
```rust
pub peer_id: String,  // Add this after line 29
```

**Then update the view model builder** (`shared/src/app/transfer/module.rs:655-675`):

**Add:**
```rust
Some(ReceiveSessionViewModel {
    id: it.order_id.to_string(),
    peer_id: peer.id.clone(),  // Add this line
    peer_avatar: AvatarViewModel::new(peer.avatar_url.clone()),
    // ... rest of fields
})
```

---

### Phase 4: Frontend - Update Session Click Handler

**File**: `web-next/app/transfer/receive_board.tsx:392-419`

**Current code (line 403-409):**
```typescript
<TransferSession
    onPress={() => {
        core.update(new AppEventVariantTransfer(new TransferEventVariantViewSession()))
        core.updateSelectedSession(item)
        if (isMobile) {
            setOpenMobile(false)
        }
    }}
    id={item.id}
    key={item.id}
/>
```

**Issue**: ⚠️ Fires empty `ViewSession` event which is wrong for both session types

**Fix:**
```typescript
<TransferSession
    onPress={() => {
        core.updateSelectedSession(item)
        if (isMobile) {
            setOpenMobile(false)
        }

        // Session detail loading is now handled by useEffect in ContentBoard
        // based on session type and password requirements
    }}
    id={item.id}
    key={item.id}
/>
```

**Reasoning**: Remove the event firing from here - let the ContentBoard component handle it based on session type

---

### Phase 5: Backend - Verify Password Validation

**File**: `shared/src/app/transfer/commands.rs:248-285`

**Current handler** (`handle_view_session_request`):
```rust
pub async fn handle_view_session_request(
    &self,
    peer_id: String,
    request_id: String,
    password: Option<String>,
    session: Option<TransferSession>
) -> Result<(), CoreError> {
    let Some(session) = session else {
       return Ok(());
    };

    let is_password_valid = match &session.target {
        TransferTarget::P2P { password: session_password, is_required_password, .. } => {
            if *is_required_password {
                match (session_password, &password) {
                    (Some(expected), Some(provided)) => expected == provided,
                    (Some(_), None) => false,
                    (None, _) => true
                }
            } else {
                true
            }
        }
        _ => false
    };

    if !is_password_valid {
        return Ok(());
    }

    self.run(P2POperation::send_session_detail(
        peer_id,
        request_id,
        session
    )).await?;

    Ok(())
}
```

**Analysis:**
- ✅ Correctly validates password for P2P sessions
- ✅ Handles all cases: required with match, required without, not required
- ⚠️  Returns `Ok(())` silently on invalid password - receiver won't get error feedback

**Enhancement (Optional)**:
Consider sending error response to peer on invalid password:
```rust
if !is_password_valid {
    self.run(P2POperation::SendSessionDetail {
        peer_id,
        request_id,
        session: None,
        error: Some(CoreError::Unauthorized("Invalid password".to_string()))
    }).await?;
    return Ok(());
}
```

---

## Complete Implementation Checklist

### Phase 1: Rust Core Updates ✅
- [x] **Verify** view model population logic (module.rs:587-593) - No changes needed
- [ ] **Add** `peer_id: String` field to `ReceiveSessionViewModel` (view_models/receive_session.rs:29)
- [ ] **Update** view model builder to populate `peer_id` (module.rs:655+)
- [ ] **Optional**: Add error response for invalid password (commands.rs:275)

### Phase 2: TypeScript Type Generation
- [ ] Run `cargo check -p shared` - ensure compilation
- [ ] Build to WASM and generate TypeScript types
- [ ] Verify `ReceiveSessionViewModel` has `peer_id` field in generated types

### Phase 3: Frontend Updates
- [ ] **Add** `nearbyPassword` state variable in ContentBoard
- [ ] **Add** `requestNearbySessionDetail` helper function
- [ ] **Add** useEffect for auto-loading nearby sessions without password (after line 168)
- [ ] **Add** password UI for nearby sessions (after line 209)
- [ ] **Update** session click handler to remove event firing (line 403-409)
- [ ] **Verify** imports include `TransferEventVariantRequestSessionDetail`

### Phase 4: Testing
- [ ] **Test**: Click nearby session without password - should auto-load
- [ ] **Test**: Click nearby session with password - should show password input
- [ ] **Test**: Enter correct password - should load session details
- [ ] **Test**: Enter wrong password - should show error or stay on password screen
- [ ] **Test**: Cloud sessions still work correctly (regression test)
- [ ] **Test**: Mobile view works correctly
- [ ] **Test**: Session switching between nearby and cloud sessions
- [ ] **Test**: Multiple nearby sessions with different password states

---

## File Modification Summary

### Rust Files (shared/)
1. **src/app/view_models/receive_session.rs**
   - Add `peer_id: String` field (after line 29)

2. **src/app/transfer/module.rs**
   - Add `peer_id: peer.id.clone()` to view model builder (around line 656)

3. **src/app/transfer/commands.rs** (Optional)
   - Add error response for invalid password (line 275)

### TypeScript Files (web-next/)
1. **app/transfer/receive_board.tsx**
   - Add `nearbyPassword` state (line 127+)
   - Add auto-load useEffect for nearby sessions (line 168+)
   - Add password UI for nearby sessions (line 209+)
   - Add `requestNearbySessionDetail` helper function
   - Update session click handler (line 403-409)

---

## Event Flow Diagrams

### Flow 1: Nearby Session WITHOUT Password
```
User clicks nearby session in list
  ↓
SessionItemsList.onPress → updateSelectedSession(item)
  ↓
ContentBoard useEffect detects: ReceiveSessionViewModel + !password_required
  ↓
Auto-fire: TransferEventVariantRequestSessionDetail(peer_id, session_id, null)
  ↓
Core: RequestSessionDetail handler (module.rs:410-419)
  ↓
Core: request_session_detail command (commands.rs:287-321)
  ↓
P2P: Send ViewSessionDetail request to peer
  ↓
Peer: handle_view_session_request validates (no password needed)
  ↓
Peer: Sends session details back
  ↓
Receiver: Session details populate (resources, progress)
  ↓
View model: is_authenticated = true, has_details = true
  ↓
UI: Shows session resources (images, videos, files)
```

### Flow 2: Nearby Session WITH Password
```
User clicks nearby session in list
  ↓
SessionItemsList.onPress → updateSelectedSession(item)
  ↓
ContentBoard detects: ReceiveSessionViewModel + password_required + !is_authenticated
  ↓
UI: Shows password input screen
  ↓
User enters password and clicks Continue
  ↓
requestNearbySessionDetail() → Fire TransferEventVariantRequestSessionDetail(peer_id, session_id, password)
  ↓
Core: RequestSessionDetail handler
  ↓
Core: request_session_detail command with password
  ↓
P2P: Send ViewSessionDetail request to peer with password
  ↓
Peer: handle_view_session_request validates password
  ↓ (if valid)
Peer: Sends session details back
  ↓
Receiver: Session details populate
  ↓
View model: is_authenticated = true, has_details = true
  ↓
UI: Shows session resources
  ↓ (if invalid)
Peer: Returns nothing (or error response if enhanced)
  ↓
Receiver: Session stays in password_required state
  ↓
UI: Stays on password input (no change, could add error message)
```

---

## Key Differences from Cloud Sessions

| Aspect | Cloud Sessions | Nearby Sessions |
|--------|---------------|----------------|
| Event Type | `ViewSession` | `RequestSessionDetail` |
| Password State | `password: Option<String>` in view model | Only `password_required: bool` |
| Auto-load | Lines 157-168 | Need to add (Phase 2.2) |
| Validation | Server-side | Peer-to-peer sender validates |
| Error Handling | Server returns error response | Currently silent failure |
| Session ID Type | String (from URL query) | String (order_id) |

---

## Edge Cases to Handle

1. **Peer goes offline during password entry**
   - Session should show "Peer unavailable" message
   - Need to detect peer disconnection

2. **Multiple password attempts**
   - Currently no rate limiting or attempt tracking
   - Consider adding attempt counter

3. **Session details change while viewing**
   - Resources could be added/removed by sender
   - Need to handle updates to existing session

4. **Wrong password entered**
   - Currently silent failure
   - Should show error message to user
   - Recommendation: Add error state and message

5. **Session already authenticated but password changed**
   - Edge case: sender changes password after receiver loaded session
   - Current behavior: receiver keeps access (until session refresh)
   - Acceptable behavior for P2P

---

## Recommended Enhancements (Post-MVP)

### 1. Add Error Feedback for Wrong Password
**Location**: ContentBoard component

**Add state:**
```typescript
const [passwordError, setPasswordError] = useState<string>('')
```

**Show error:**
```typescript
{passwordError && (
    <p className="text-destructive text-sm">{passwordError}</p>
)}
```

**Set error on failed attempt** (would need backend support)

### 2. Add Loading State During Authentication
```typescript
const [isAuthenticating, setIsAuthenticating] = useState(false)
```

Show spinner while waiting for session details response

### 3. Add "Forgot Password" or "Request Access" Feature
Allow receiver to send message to sender requesting password

### 4. Add Session Refresh Button
Allow user to manually request session details again

---

## Success Criteria

- [ ] Nearby sessions display password lock icon when `password_required = true`
- [ ] Clicking password-protected nearby session shows password input
- [ ] Entering correct password loads session details (resources visible)
- [ ] Entering wrong password keeps user on password screen (no crash)
- [ ] Nearby sessions without password auto-load on click
- [ ] Cloud sessions continue to work (no regression)
- [ ] Mobile view works correctly for both session types
- [ ] TypeScript compilation passes
- [ ] Rust `cargo check -p shared` passes
- [ ] All tests pass (if tests exist)

---

## Notes

- View model already has all necessary fields (`password_required`, `is_authenticated`, `has_details`)
- Backend password validation logic already exists and works correctly
- Main work is UI wiring and auto-load logic
- Similar pattern to cloud sessions makes implementation straightforward
- Only missing piece is `peer_id` in view model for firing the request
