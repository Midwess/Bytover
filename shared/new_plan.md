# Implementation Plan: Display Nearby Session with User Information

## Overview
Simplify P2P session discovery by removing SessionsNotification. Instead, sessions will be discovered through direct peer communication and ViewSessionDetail requests, similar to how Public sessions work.

## Current Architecture Analysis

**Current P2P Session Flow (TO BE REMOVED):**
1. Sender creates a P2P session with `TransferSession::p2p()`
2. When a peer connects, sender calls `send_sessions_notification()` which sends `P2PSessionOverviewMessage`
3. Receiver gets `ReceivedSessionsOverview` event and creates stub sessions
4. UI displays sessions with peer info

**New Simplified P2P Flow:**
1. Sender creates a P2P session with user information
2. Peer connects via WebRTC
3. Receiver can request session details directly (ViewSessionDetail already exists)
4. Session information includes user details from backend

**Public Session Flow (UNCHANGED):**
1. Sender creates session with `TransferSession::public()` which includes `from_user` in `TransferTarget::Internet`
2. Receiver finds session by alias using `FindPublicSession { alias }`
3. Backend fetches full session with user information
4. UI displays rich sender info (name, avatar, alias)

**View Models:**
- `ReceiveSessionViewModel` - for P2P sessions (uses peer info)
- `ReceiveCloudSessionViewModel` - for Internet sessions (uses from_user info)
- **Goal:** Merge into single unified `ReceiveSessionViewModel`

---

## Implementation Steps

### **Step 1: Remove SessionsNotification from Protobuf**
**File:** `libs/schema/proto/devlog/bitbridge/request.proto:54-56`

Remove:
```protobuf
message SessionsNotificationMessage {
    repeated P2PSessionOverviewMessage sessions = 1;
}
```

**File:** `libs/schema/proto/devlog/bitbridge/request.proto:25`

Remove from PeerMessageBody:
```protobuf
SessionsNotificationMessage sessions_notification = 8;
```

**File:** `libs/schema/proto/devlog/bitbridge/session.proto:8-11`

Remove entire `P2PSessionOverviewMessage`:
```protobuf
message P2PSessionOverviewMessage {
  required uint64 order_id = 1;
  required bool password_protected = 2;
}
```

**Rebuild schema:**
```bash
cd libs/schema && cargo build
```

---

### **Step 2: Remove P2PSessionOverview from Operations**
**File:** `shared/src/app/operations/p2p.rs:84-88`

Remove entire struct:
```rust
pub struct P2PSessionOverview {
    pub order_id: u64,
    pub password_protected: bool
}
```

**File:** `shared/src/app/operations/p2p.rs:23-26`

Remove from P2POperation enum:
```rust
SendSessionsNotification {
    peer_id: String,
    sessions: Vec<TransferSession>
},
```

**File:** `shared/src/app/operations/p2p.rs:59-62`

Remove from P2POperationOutput enum:
```rust
ReceivedSessionsOverview {
    peer_id: String,
    sessions: Vec<P2PSessionOverview>
},
```

**File:** `shared/src/app/operations/p2p.rs:111-113`

Remove helper method:
```rust
pub fn send_sessions_notification(peer_id: String, sessions: Vec<TransferSession>) -> AppRequestBuilder<impl Future<Output = Result<(), CoreError>>> {
    Command::request_from_shell(CoreOperation::P2P(P2POperation::SendSessionsNotification { peer_id, sessions })).map(|it| it.result())
}
```

---

### **Step 3: Remove SessionsNotification Handler from Peer**
**File:** `shared/src/protocol/webrtc/peer.rs:338-352`

Remove from `process_message_packet`:
```rust
Request::SessionsNotification(notification) => {
    let sessions: Vec<P2PSessionOverview> = notification.sessions.iter().map(|s| {
        P2PSessionOverview {
            order_id: s.order_id,
            password_protected: s.password_protected,
        }
    }).collect();
    let response = CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionsOverview {
        peer_id: self.peer.id().to_string(),
        sessions,
    });
    if let Some(core_request) = self.core_request() {
        core_request.response(response).await;
    }
}
```

**File:** `shared/src/protocol/webrtc/peer.rs:540-559`

Remove entire `send_sessions_notification` method:
```rust
pub async fn send_sessions_notification(
    &self,
    sessions: Vec<TransferSession>,
) -> Result<(), WebRtcErrors> {
    // ... entire method
}
```

---

### **Step 4: Remove SessionsNotification from WebRTC**
**File:** `shared/src/protocol/webrtc/webrtc.rs:104-115`

Remove entire `send_sessions_notification` method:
```rust
pub async fn send_sessions_notification(
    &self,
    peer_id: String,
    sessions: Vec<TransferSession>,
) -> Result<(), WebRtcErrors> {
    // ... entire method
}
```

---

### **Step 5: Remove from Executor**
**File:** `shared/src/shell/executor/p2p.rs:44-46`

Remove case:
```rust
P2POperation::SendSessionsNotification { peer_id, sessions } => {
    self.web_rtc().send_sessions_notification(peer_id, sessions).await?;
    Ok(CoreOperationOutput::None)
}
```

---

### **Step 6: Remove ReceivedSessionsOverview Event**
**File:** `shared/src/app/transfer/module.rs:90-93`

Remove from TransferEvent enum:
```rust
ReceivedSessionsOverview {
    peer_id: String,
    sessions: Vec<P2PSessionOverview>
},
```

**File:** `shared/src/app/transfer/module.rs:405-452`

Remove entire event handler:
```rust
TransferEvent::ReceivedSessionsOverview { peer_id, sessions } => {
    // ... entire handler
}
```

---

### **Step 7: Remove from Nearby Command Handler**
**File:** `shared/src/app/nearby/command.rs:149-151`

Remove case:
```rust
CoreOperationOutput::P2P(P2POperationOutput::ReceivedSessionsOverview { peer_id, sessions }) => {
    self.notify_event(TransferEvent::ReceivedSessionsOverview { peer_id, sessions });
}
```

---

### **Step 8: Add RPC Operation to Get User by ID**
**File:** `shared/src/app/operations/rpc.rs:14-24`

Add new operation to fetch user by ID:
```rust
pub enum RpcOperation {
    GetAuthenticateUrl(DeviceInfo),
    GetMe(),
    GetUserById(u64),  // Add this
    Feedback {
        email: String,
        message: String,
    },
    RandomAvatar,
    CreateP2PSession { password_protected: bool },
}
```

Update output enum:
```rust
pub enum RpcOperationOutput {
    GetMe(User),
    GetUserById(User),  // Add this
}
```

Add helper method:
```rust
impl RpcOperation {
    pub fn get_user_by_id(user_id: u64) -> AppRequestBuilder<impl Future<Output = Result<User, CoreError>>> {
        Command::request_from_shell(CoreOperation::Rpc(RpcOperation::GetUserById(user_id))).map(|res| match res {
            CoreOperationOutput::Rpc(RpcOperationOutput::GetUserById(user)) => Ok(user),
            CoreOperationOutput::Error(error) => Err(error),
            e => panic!("Invalid output for RpcOperation::GetUserById got {e:?}")
        })
    }
}
```

---

### **Step 9: Update TransferTarget to Add User Fields**
**File:** `shared/src/entities/target.rs:6-22`

Change `TransferTarget::P2P` from:
```rust
P2P {
    from_peer: Option<Peer>,
    password: Option<String>,
    is_required_password: bool,
    signalling_key: String,
    scope: String
}
```

To:
```rust
P2P {
    from_peer: Option<Peer>,
    from_user: Option<User>,
    alias: Option<String>,
    password: Option<String>,
    is_required_password: bool,
    signalling_key: String,
    scope: String
}
```

**Impact:** Update all pattern matches on `TransferTarget::P2P` to include the new fields:
- `shared/src/entities/transfer_session.rs:203-209` - `TransferSession::p2p()` constructor
- `shared/src/entities/transfer_session.rs:280-284` - `peer_id()` method
- `shared/src/entities/transfer_session.rs:287-291` - `peer()` method
- `shared/src/app/transfer/module.rs:267-287` - `PeerUpdated` handler
- `shared/src/app/transfer/module.rs:637-645` - view function for P2P sessions

---

### **Step 10: Store User Info When Creating P2P Sessions**
**File:** `shared/src/app/transfer/module.rs:230-259`

Update `StartP2PTransfer` handler to check authentication and store user info:

```rust
TransferEvent::StartP2PTransfer { password, .. } => {
    let selected_resources = model.shelf.shelf.resources.clone();
    if selected_resources.is_empty() {
        return Command::new(|it| async move {
            let _ = DialogOperation::toast("No resources selected".to_string()).into_future(it.clone()).await;
        });
    }

    // Check if user is authenticated - if not, trigger sign-in flow
    let user = model.authentication.user.clone();
    if user.is_none() {
        log::info!("User is not logged in, opening login page");
        return Command::handle_result(|it| async move {
            it.app().authenticate().await;
            Ok(())
        });
    }

    let Some(_me) = model.nearby.me.clone() else {
        log::info!("Nearby service not available");
        return Command::done()
    };

    Command::handle_result(move |it| async move {
        let p2p_session = it.app().run(RpcOperation::create_p2p_session(password.is_some())).await?;

        let mut session = TransferSession::p2p(
            selected_resources,
            password,
            p2p_session.signalling_room_id.clone(),
            p2p_session.signalling_scope.clone(),
        );

        // Store user info and alias in session
        if let TransferTarget::P2P { from_user, alias, .. } = &mut session.target {
            *from_user = user;
            *alias = Some(p2p_session.alias.clone());
        }

        it.update_model(TransferSessionModelEvent::Add(session.clone()));

        let scope = FindingScope::Global(p2p_session.signalling_room_id);
        it.update_model(NearbyEvent::AddFindingScope(scope));

        Ok(())
    })
}
```

---

### **Step 11: Enhance ViewSessionDetail to Include User Info**
**File:** `shared/src/app/transfer/module.rs` (ReceivedViewSessionRequest handler)

When handling ViewSessionRequest on the sender side, include user information in the response. The existing `P2PTransferSessionMessage` should include user details.

Update the handler to include user info from authentication model:
```rust
TransferEvent::ReceivedViewSessionRequest { peer_id, request_id, order_id, password } => {
    let session_id = TransferSessionId {
        order_id: Some(order_id.to_string()),
        transfer_type: Some(TransferType::Send)
    };
    let session = model.transfer.sessions.lookup(&session_id).cloned();
    let current_user = model.authentication.user.clone();

    Command::handle_result(move |it| async move {
        if let Some(mut session) = session {
            // If session doesn't have user info but we're authenticated, add it
            if let TransferTarget::P2P { from_user, .. } = &mut session.target {
                if from_user.is_none() && current_user.is_some() {
                    *from_user = current_user;
                }
            }
            it.app().handle_view_session_request(peer_id, request_id, password, Some(session)).await
        } else {
            it.app().handle_view_session_request(peer_id, request_id, password, None).await
        }
    })
}
```

---

### **Step 12: Update P2PTransferSessionMessage to Include User Info**
**File:** `libs/schema/proto/devlog/bitbridge/session.proto:13-16`

Update to include user information:
```protobuf
message P2PTransferSessionMessage {
  required uint64 order_id = 1;
  repeated ResourceMessage resources = 2;
  optional uint64 user_id = 3;
  optional string alias = 4;
}
```

**Rebuild schema:**
```bash
cd libs/schema && cargo build
```

---

### **Step 13: Update Session Detail Sending/Receiving**
**File:** Wherever P2PTransferSessionMessage is built (likely in peer.rs or transfer commands)

When building P2PTransferSessionMessage, include user_id and alias:
```rust
P2PTransferSessionMessage {
    order_id: session.order_id,
    resources: session.resources.iter().map(|r| to_resource_message(r)).collect(),
    user_id: match &session.target {
        TransferTarget::P2P { from_user, .. } => from_user.as_ref().map(|u| u.id),
        _ => None
    },
    alias: match &session.target {
        TransferTarget::P2P { alias, .. } => alias.clone(),
        _ => None
    },
}
```

When receiving P2PTransferSessionMessage, fetch user if user_id is present:
```rust
if let Some(user_id) = message.user_id {
    if let Ok(user) = it.app().run(RpcOperation::get_user_by_id(user_id)).await {
        if let TransferTarget::P2P { from_user, .. } = &mut session.target {
            *from_user = Some(user);
        }
    }
}

if let Some(alias) = message.alias {
    if let TransferTarget::P2P { alias: session_alias, .. } = &mut session.target {
        *session_alias = Some(alias);
    }
}
```

---

### **Step 14: Merge View Models**
**File:** `shared/src/app/view_models/receive_session.rs:28-61`

Remove `ReceiveCloudSessionViewModel` and update `ReceiveSessionViewModel` to support both P2P and Internet sessions:

```rust
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReceiveSessionViewModel {
    pub id: String,
    pub sender_id: String,
    pub sender_avatar: String,
    pub sender_name: String,
    pub sender_description: String,
    pub alias: Option<String>,
    pub access_url: Option<String>,
    pub password: Option<String>,
    pub password_required: bool,
    pub is_authenticated: bool,
    pub has_details: bool,
    pub is_loading: bool,
    pub image_resources: Vec<ImageReceiveResourceViewModel>,
    pub video_resources: Vec<VideoReceiveResourceViewModel>,
    pub file_resources: Vec<FileReceiveResourceViewModel>,
    pub is_completed: bool,
    pub is_in_progress: bool,
    pub display_download_speed: String,
    pub progress: f64,
    pub display_datetime: String
}
```

---

### **Step 15: Update Transfer Module View Function**
**File:** `shared/src/app/transfer/module.rs:504-759`

Merge the two separate view builders into one unified function:

```rust
fn view(&self, model: &AppModel) -> Self::ViewModel {
    Self::ViewModel {
        transfer_method: model.transfer.selected_method.clone(),
        received_sessions: model
            .transfer
            .sessions
            .iter()
            .filter(|it| it.transfer_type == TransferType::Receive)
            .filter_map(|it| {
                let (sender_id, sender_avatar, sender_name, sender_description, alias, access_url, password, is_required_password, is_loading) = match &it.target {
                    TransferTarget::P2P { from_peer, from_user, alias, is_required_password, .. } => {
                        let peer = from_peer.as_ref()?;
                        let (avatar, name, description) = if let Some(user) = from_user {
                            (user.avatar.clone(), user.name.clone(), "Nearby".to_string())
                        } else {
                            (peer.avatar_url.clone(), peer.name.clone().unwrap_or(peer.device.name.clone()), "Nearby".to_string())
                        };
                        let has_details = !it.resources.is_empty();
                        (peer.id().to_string(), avatar, name, description, alias.clone(), None, None, *is_required_password, !has_details)
                    }
                    TransferTarget::Internet { password, from_user, access_url, is_required_password, .. } => {
                        let access_url = access_url.as_ref()?;
                        let alias = Url::parse(access_url).ok()
                            .and_then(|url| url.query_pairs().find(|it| it.0 == "session").map(|it| it.1.to_string()));
                        let name = match &alias {
                            Some(a) => format!("{} ({})", from_user.name, a),
                            None => from_user.name.to_string()
                        };
                        let is_loading = it.resources.is_empty();
                        (from_user.id.to_string(), from_user.avatar.clone(), name, "Public".to_string(), alias, Some(access_url.clone()), password.clone(), *is_required_password, is_loading)
                    }
                };

                let image_resources = it.resources.iter().filter_map(|resource| {
                    if resource.r#type != ResourceType::Image { return None; }
                    let progress = it.progress.iter().find(|p| p.resource_order_id == resource.order_id)?;
                    Some(ImageReceiveResourceViewModel {
                        model: SelectedResourceViewModel::from(resource),
                        completion: progress.percentage() as f32,
                        is_completed: progress.status.is_completed()
                    })
                }).collect();

                let video_resources = it.resources.iter().filter_map(|resource| {
                    if resource.r#type != ResourceType::Video { return None; }
                    let progress = it.progress.iter().find(|p| p.resource_order_id == resource.order_id)?;
                    Some(VideoReceiveResourceViewModel {
                        model: SelectedResourceViewModel::from(resource),
                        completion: progress.percentage() as f32,
                        is_completed: progress.status.is_completed()
                    })
                }).collect();

                let file_resources = it.resources.iter().filter_map(|resource| {
                    if !matches!(resource.r#type, ResourceType::File | ResourceType::Folder) { return None; }
                    let progress = it.progress.iter().find(|p| p.resource_order_id == resource.order_id)?;
                    Some(FileReceiveResourceViewModel {
                        model: SelectedResourceViewModel::from(resource),
                        completion: progress.percentage() as f32,
                        is_completed: progress.status.is_completed()
                    })
                }).collect();

                Some(ReceiveSessionViewModel {
                    id: it.order_id.to_string(),
                    sender_id,
                    sender_avatar,
                    sender_name,
                    sender_description,
                    alias,
                    access_url,
                    password,
                    password_required: is_required_password,
                    is_authenticated: !it.resources.is_empty(),
                    has_details: !it.resources.is_empty(),
                    is_loading,
                    is_completed: it.is_completed(),
                    is_in_progress: !it.is_completed() && !it.is_canceled(),
                    display_download_speed: it.status().to_string(),
                    progress: it.total_progress(),
                    image_resources,
                    video_resources,
                    file_resources,
                    display_datetime: id_to_datetime(it.order_id)
                        .with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M")
                        .to_string()
                })
            })
            .collect(),
        cloud_session: model.transfer.sessions.iter()
            .filter(|it| matches!(it.transfer_type, TransferType::Send))
            .filter(|it| it.target.is_public())
            .find_map(|it| {
                let (access_url, password) = match &it.target {
                    TransferTarget::Internet { access_url, password, .. } => (access_url.clone(), password.clone()),
                    _ => return None
                };
                Some(CloudSession {
                    display_download_speed: match access_url.is_none() {
                        true => "Initializing...".to_owned(),
                        false => it.status().to_string()
                    },
                    password,
                    session_id: it.order_id.to_string(),
                    is_completed: it.is_completed(),
                    is_in_progress: !it.is_completed() && !it.is_canceled(),
                    progress: it.total_progress(),
                    access_url
                })
            }),
    }
}
```

Update `TransferViewModel`:
```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TransferViewModel {
    transfer_method: TransferMethodSelection,
    received_sessions: Vec<ReceiveSessionViewModel>,
    cloud_session: Option<CloudSession>
}
```

Remove `received_cloud_sessions` field.

---

### **Step 16: Implement Backend RPC Handler**
**File:** Backend RPC service implementation

Add handler for `GetUserById`:
```rust
async fn get_user_by_id(&self, user_id: u64) -> Result<User, Status> {
    let user = self.user_repository.find_by_id(user_id).await?
        .ok_or_else(|| Status::not_found("User not found"))?;

    Ok(User {
        id: user.id,
        email: user.email,
        name: user.name,
        avatar: user.avatar,
    })
}
```

---

### **Step 17: Build TypeScript Types**
**Command:**
```bash
cd /Users/tiendang/Projects/bitbridge
cargo build -p shared_types --target wasm32-unknown-unknown --no-default-features --features typescript
```

This will regenerate TypeScript types with the updated `ReceiveSessionViewModel`.

---

### **Step 18: Update Web UI**
**Files to update:**
- `web-next/wasm/wasm_core.ts:98` - Update type from union to single type
- Any React components using `ReceiveCloudSessionViewModel`

Change:
```typescript
selectedSession: Observable<ReceiveSessionViewModel | ReceiveCloudSessionViewModel> = new Observable()
```

To:
```typescript
selectedSession: Observable<ReceiveSessionViewModel> = new Observable()
```

Update components to use unified model fields:
- Use `sender_avatar` instead of `peer_avatar` or `avatar_url`
- Use `sender_name` instead of `peer_name` or `sender_name`
- Use `sender_description` for "Nearby" vs "Public"
- Use `alias` for session alias (available for both types)
- Use `access_url` to determine if it's a public session

---

## Summary of Changes

| Step | File | Change Type | Description |
|------|------|-------------|-------------|
| 1 | `libs/schema/proto/devlog/bitbridge/request.proto` | Remove | Remove SessionsNotificationMessage |
| 1 | `libs/schema/proto/devlog/bitbridge/session.proto` | Remove | Remove P2PSessionOverviewMessage |
| 2 | `shared/src/app/operations/p2p.rs` | Remove | Remove P2PSessionOverview struct and operations |
| 3 | `shared/src/protocol/webrtc/peer.rs` | Remove | Remove SessionsNotification handler and sender |
| 4 | `shared/src/protocol/webrtc/webrtc.rs` | Remove | Remove send_sessions_notification |
| 5 | `shared/src/shell/executor/p2p.rs` | Remove | Remove SendSessionsNotification executor |
| 6 | `shared/src/app/transfer/module.rs` | Remove | Remove ReceivedSessionsOverview event |
| 7 | `shared/src/app/nearby/command.rs` | Remove | Remove ReceivedSessionsOverview handler |
| 8 | `shared/src/app/operations/rpc.rs` | Add | Add GetUserById RPC operation |
| 9 | `shared/src/entities/target.rs` | Modify | Add from_user and alias to TransferTarget::P2P |
| 10 | `shared/src/app/transfer/module.rs` | Modify | Store user info in StartP2PTransfer |
| 11 | `shared/src/app/transfer/module.rs` | Modify | Include user in ViewSessionRequest handler |
| 12 | `libs/schema/proto/devlog/bitbridge/session.proto` | Modify | Add user_id and alias to P2PTransferSessionMessage |
| 13 | Session detail handlers | Modify | Send/receive user_id in session details |
| 14 | `shared/src/app/view_models/receive_session.rs` | Modify | Remove ReceiveCloudSessionViewModel |
| 15 | `shared/src/app/transfer/module.rs` | Modify | Merge view builders |
| 16 | `backend/src/grpc/rpc_service.rs` | Add | Implement GetUserById handler |
| 17 | Build | Execute | Rebuild TypeScript types |
| 18 | `web-next/**/*.tsx` | Modify | Update UI to use unified view model |

## Benefits of This Simplified Approach

1. **Less Complexity**: No session overview notifications to maintain
2. **On-Demand**: Session details fetched only when needed (via ViewSessionDetail)
3. **Consistent**: Both P2P and Public sessions use same pattern (request details when needed)
4. **Less Network Traffic**: No automatic broadcasting of session lists
5. **Better Privacy**: Sessions not automatically visible, must be requested explicitly
6. **Cleaner Code**: Remove ~200 lines of notification handling code

## New Flow

**Sender:**
1. Create P2P session with user info stored locally
2. Peer connects
3. Wait for ViewSessionDetail request
4. Send session details including user_id
5. Receiver fetches user info and displays

**Receiver:**
1. See peer in nearby list
2. Request session details (existing ViewSessionDetail)
3. Receive session with user_id
4. Fetch user info via GetUserById
5. Display session with user info

## Testing Plan

1. **P2P Session Creation:**
   - Verify user info is stored when creating P2P session
   - Verify alias is stored correctly

2. **P2P Session Details:**
   - Request session details from peer
   - Verify user_id is included in response
   - Verify user info is fetched and displayed

3. **Public Session:**
   - Verify public sessions still work unchanged
   - Same UI appearance as P2P sessions

4. **View Model Unification:**
   - Both nearby and public sessions use same ReceiveSessionViewModel
   - UI renders both identically

5. **Backward Compatibility:**
   - Handle sessions without user_id gracefully
   - Fall back to peer info if user not found
