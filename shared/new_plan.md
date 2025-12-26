# Streaming Architecture Implementation Plan

## Overview
Refactor session detail and resource download flows to use streaming architecture with **existing Transfer events** (`TransferResourceProgressUpdate`, `ThumbnailUpdated`, `TransferCompleted`, `Error`), following the same pattern as the `upload` function in commands.rs.

**Key Change:** Remove `SessionDetailReceived` and `SessionDetailFailed` events entirely. Use pure streaming with `TransferResourceProgressUpdate` for resources and progress.

---

## Phase 1: Session Detail Streaming

### Current Flow
```
Core (commands.rs)
  → P2POperation::view_session_detail()
    → webrtc.rs::view_session_detail()
      → peer.rs::request_session_detail()
        → Returns complete TransferSession
        → Emits P2POperationOutput::SessionDetailReceived event
  → nearby/command.rs converts to TransferEvent::SessionDetailReceived
  → commands.rs::handle_session_detail_received() handles the event
```

### New Flow (pure streaming - NO SessionDetailReceived)
```
Core (commands.rs)
  → Start stream: P2POperation::ViewSessionDetail
    → executor passes CoreRequest to webrtc.rs::view_session_detail()
      → webrtc.rs starts core stream on peer
      → peer.rs::request_session_detail() streams via CoreRequest
        → For each resource: TransferOperationOutput::TransferResourceProgressUpdate
        → For each thumbnail: TransferOperationOutput::ThumbnailUpdated
        → At end: TransferOperationOutput::TransferCompleted
  → commands.rs::request_session_detail() builds session from streamed resources
```

### Implementation Steps

#### 1.1 Update peer.rs::request_session_detail()
**Location:** `shared/src/protocol/webrtc/peer.rs:549-628`

**Changes:**
- Keep the streaming response loop (already implemented)
- **REMOVE** the SessionDetailReceived emission at lines 619-625
- Change from building complete session to streaming individual resource events
- Return `Result<(), WebRtcErrors>` instead of `Result<TransferSession, WebRtcErrors>`
- Emit `TransferResourceProgressUpdate` for each resource (with the resource embedded)
- Emit `ThumbnailUpdated` when thumbnail is saved
- Emit `TransferCompleted` at the end

**Implementation:**
```rust
pub async fn request_session_detail(
    &self,
    order_id: u64,
    password: Option<String>,
) -> Result<(), WebRtcErrors> {
    use schema::devlog::bitbridge::view_session_detail_response::Result as ResponseResult;
    let request = ViewSessionDetailRequest { order_id, password };

    let mut response = Box::pin(self.msg_channel.stream(Request::ViewSessionRequest(request)).await?);

    while let Some(Response::ViewSessionResponse(resp)) = response.next().await {
        match resp.result {
            Some(ResponseResult::Session(_proto_session)) => {
                // Session metadata received - we don't emit anything here
                // Just continue to wait for resources
            }
            Some(ResponseResult::ResourceUpdated(resource_proto)) => {
                let mut resource = LocalResource {
                    order_id: resource_proto.order_id,
                    name: resource_proto.name,
                    size: resource_proto.size as u64,
                    path: LocalResourcePath::RelativePath {
                        path: format!("received/session_{}/resource_{}", order_id, resource_proto.order_id),
                        is_private: false,
                    },
                    thumbnail_path: None,
                    r#type: resource_proto.r#type.into(),
                };

                // Save thumbnail if present
                if let Some(thumbnail_bytes) = resource_proto.thumbnail_png {
                    match self.resource_repo.save_thumbnail(thumbnail_bytes, resource.order_id).await {
                        Ok(thumbnail_path) => {
                            resource.thumbnail_path = Some(thumbnail_path.clone());

                            // Emit thumbnail update
                            if let Some(core_request) = self.core_request() {
                                core_request.response(CoreOperationOutput::Transfer(
                                    TransferOperationOutput::ThumbnailUpdated(ThumbnailUpdatedEvent {
                                        resource_id: resource.order_id,
                                        path: thumbnail_path,
                                    })
                                )).await;
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to save thumbnail for resource {}: {:?}", resource.order_id, e);
                        }
                    }
                }

                // Create a TransferProgress for this resource
                let progress = TransferProgress::new(
                    resource.order_id,
                    resource.size,
                    crate::entities::transfer_session::TransferType::Receive
                );

                // Emit resource via progress update
                // Note: We need to extend TransferProgress to include resource or use a wrapper
                // For now, emit as a special event that includes the resource
                if let Some(core_request) = self.core_request() {
                    // TODO: This needs architecture discussion - how to pass resource?
                    // Option 1: Extend UpdateAction to handle resource addition
                    // Option 2: Create ResourceReceived event
                    // Option 3: Use model events directly
                    core_request.response(CoreOperationOutput::Transfer(
                        TransferOperationOutput::TransferResourceProgressUpdate(progress)
                    )).await;
                }
            }
            Some(ResponseResult::Error(error_type)) => {
                let error_msg = PeerErrorsMessage::try_from(error_type)
                    .unwrap_or(PeerErrorsMessage::InvalidRequest);

                if let Some(core_request) = self.core_request() {
                    core_request.response(CoreOperationOutput::Error(
                        CoreError::WebRtc(error_msg.to_string())
                    )).await;
                }

                return Err(WebRtcErrors::PeerError(error_msg.to_string()));
            }
            None => break,
        }
    }

    // Emit completion
    if let Some(core_request) = self.core_request() {
        core_request.response(CoreOperationOutput::Transfer(
            TransferOperationOutput::TransferCompleted(TransferSessionStatus::Success)
        )).await;
    }

    Ok(())
}
```

**BLOCKER:** The existing `TransferProgress` struct doesn't contain the `LocalResource`. We need to decide how to pass the resource information:
- **Option A:** Add a `resource: Option<LocalResource>` field to `TransferProgress`
- **Option B:** Create a new event type `ResourceReceived { resource: LocalResource }`
- **Option C:** Emit model events directly instead of going through TransferOperationOutput

#### 1.2 Update webrtc.rs::view_session_detail()
**Location:** `shared/src/protocol/webrtc/webrtc.rs:115-129`

**Changes:**
- Accept `core_request: CoreRequest` parameter
- Call `peer.start_core_stream(core_request)` before calling request_session_detail
- Change return to `Result<(), WebRtcErrors>`

**Implementation:**
```rust
pub async fn view_session_detail(
    &self,
    peer_id: String,
    order_id: u64,
    password: Option<String>,
    core_request: CoreRequest,
) -> Result<(), WebRtcErrors> {
    let peer_id = PeerId(peer_id.parse()?);
    if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
        peer.start_core_stream(core_request);
        peer.request_session_detail(order_id, password).await
    } else {
        Err(WebRtcErrors::ConnectionNotFound(peer_id))
    }
}
```

#### 1.3 Update executor/p2p.rs
**Location:** `shared/src/shell/executor/p2p.rs:48-50`

**Changes:**
- Pass `request` (CoreRequest) to `view_session_detail`

**Implementation:**
```rust
P2POperation::ViewSessionDetail { peer_id, order_id, password } => {
    self.web_rtc().view_session_detail(peer_id, order_id, password, request).await?;
    Ok(CoreOperationOutput::None)
}
```

#### 1.4 Update commands.rs::request_session_detail()
**Location:** `shared/src/app/transfer/commands.rs:296-304`

**Changes:**
- Convert to streaming operation (like `upload()`)
- Build TransferSession from streamed resources
- Remove dependency on SessionDetailReceived event

**Implementation:**
```rust
pub async fn request_session_detail(
    &self,
    peer_id: String,
    order_id: u64,
    password: Option<String>
) -> Result<(), CoreError> {
    // Create initial empty session
    let mut transfer_session = TransferSession {
        order_id,
        resources: vec![],
        progress: vec![],
        transfer_type: TransferType::Receive,
        target: TransferTarget::P2P {
            from_peer: Peer::default(), // TODO: Get actual peer info
            password: password.clone(),
            is_required_password: password.is_some(),
        },
        cancellation_token: CancellationToken::new(),
    };

    // Add session to model immediately so resources can update it
    self.update_model(TransferSessionModelEvent::Add(transfer_session.clone()));

    let mut stream = self.stream_from_shell(
        P2POperation::ViewSessionDetail { peer_id, order_id, password }.into()
    );

    while let Some(output) = stream.next().await {
        match output {
            CoreOperationOutput::Transfer(transfer_output) => match transfer_output {
                TransferOperationOutput::TransferResourceProgressUpdate(progress) => {
                    // Resource received - add to session
                    // BLOCKER: Need resource data from progress event
                    // This depends on solution to Option A/B/C above
                    transfer_session.progress.push(progress.clone());
                    self.update_model(TransferSessionModelEvent::Update(
                        transfer_session.id(),
                        progress.into()
                    ));
                }
                TransferOperationOutput::ThumbnailUpdated(thumbnail) => {
                    // Thumbnail saved - update model
                    self.update_model(TransferSessionModelEvent::Update(
                        transfer_session.id(),
                        thumbnail.into()
                    ));
                }
                TransferOperationOutput::TransferCompleted(_status) => {
                    // Session detail complete - save to persistence
                    if let Err(e) = self.run(TransferSessionPersistentOperation::save(transfer_session.clone())).await {
                        log::error!("Failed to save session: {e:?}");
                    }
                    break;
                }
                other => {
                    log::warn!("Unexpected transfer output: {other:?}");
                }
            },
            CoreOperationOutput::Error(error) => {
                log::error!("Error loading session: {error:?}");
                self.run(DialogOperation::message(
                    format!("{error}"),
                    MessageReason::FailedToLoadSession(order_id)
                )).await;
                return Err(error);
            }
            _ => continue,
        }
    }

    Ok(())
}
```

**REMOVE:**
- `handle_session_detail_received()` function (lines 306-317)

#### 1.5 Clean up event types
**Location:** Multiple files

**Changes:**
- Remove `SessionDetailReceived` from `P2POperationOutput` (p2p.rs:72-74)
- Remove `SessionDetailFailed` from `P2POperationOutput` (p2p.rs:75-78)
- Remove `SessionDetailReceived` from `TransferEvent` (module.rs:104-106)
- Remove `SessionDetailFailed` from `TransferEvent` (module.rs:107-110)
- Remove handlers in nearby/command.rs (lines 151-156)
- Remove handlers in module.rs (lines 419-430)
- Remove `SessionDetailReceived` from `TransferOperationOutput` (transfer.rs:34-36) **after** confirming new approach works

---

## Phase 2: Download Resource Streaming

### Current Flow
```
Core (commands.rs)
  → P2POperation::download_resource()
    → webrtc.rs::download_resource()
      → peer.rs::request_resource_download()
        → Downloads entire resource
        → Returns Ok(()) when complete
  → No progress updates during download
```

### New Flow (following upload pattern)
```
Core (commands.rs)
  → Start stream: P2POperation::DownloadResource
    → executor passes CoreRequest to webrtc.rs::download_resource()
      → webrtc.rs starts core stream on peer
      → peer.rs::request_resource_download() - streams via CoreRequest
        → Emits: TransferOperationOutput::TransferResourceProgressUpdate (periodic progress)
        → Emits: TransferOperationOutput::TransferCompleted (done) or Error
  → commands.rs::request_download_resource() streams like upload()
```

### Implementation Steps

#### 2.1 Update peer.rs::request_resource_download()
**Location:** `shared/src/protocol/webrtc/peer.rs:685-767`

**Changes:**
- Keep existing structure (prefix channel, delimiter handling)
- Add progress tracking with existing `TransferProgress::new()` and `update_progress()`
- Emit `TransferResourceProgressUpdate` events periodically
- Emit `TransferCompleted` on success or error

**Implementation:**
```rust
pub async fn request_resource_download(
    &self,
    session_order_id: u64,
    resource: LocalResource,
) -> Result<(), WebRtcErrors> {
    let resource_order_id = resource.order_id;
    let resource_size = resource.size;
    let transfer_id = TRANSFER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

    let request = DownloadResourceRequest {
        session_order_id,
        resource_order_id,
        transfer_id: transfer_id as u32,
    };

    self.msg_channel.notify(Request::DownloadResourceRequest(request)).await?;

    let prefix = transfer_id;
    let (tx, mut rx) = mpsc::channel::<Packet>(1024);

    {
        let mut channels = self.prefix_channels.lock().await;
        channels.insert(prefix, tx);
    }

    let resource_repo = self.resource_repo.clone();
    let prefix_channels = self.prefix_channels.clone();

    // Wait for start delimiter
    let start_delimiter = loop {
        if let Some(packet) = rx.next().await {
            if let Ok(delimiter) = TransferDelimiterShema::from_start_packet(&packet, session_order_id) {
                break delimiter;
            }
        } else {
            log::warn!("Channel closed before receiving start delimiter");
            if let Some(core_request) = self.core_request() {
                core_request.response(CoreOperationOutput::Error(
                    CoreError::WebRtc("Channel closed before start delimiter".to_string())
                )).await;
            }
            return Err(WebRtcErrors::InvalidDelimiter("Channel closed before start delimiter".into()));
        }
    };

    let Some(resource_id) = start_delimiter.resource_id() else {
        log::error!("Start delimiter missing resource_id");
        if let Some(core_request) = self.core_request() {
            core_request.response(CoreOperationOutput::Error(
                CoreError::WebRtc("Start delimiter missing resource_id".to_string())
            )).await;
        }
        return Err(WebRtcErrors::InvalidDelimiter("Start delimiter missing resource_id".into()));
    };

    let compressed = start_delimiter.compressed();

    // Create progress tracker
    let mut progress = TransferProgress::new(
        resource_id,
        resource_size,
        crate::entities::transfer_session::TransferType::Receive
    );

    // Emit initial progress (0%)
    if let Some(core_request) = self.core_request() {
        core_request.response(CoreOperationOutput::Transfer(
            TransferOperationOutput::TransferResourceProgressUpdate(progress.clone())
        )).await;
    }

    let mut writer = match resource_repo.write(resource.path.clone(), compressed).await {
        Ok(w) => w,
        Err(e) => {
            log::error!("Failed to create writer: {:?}", e);
            progress.fail(format!("Failed to create writer: {:?}", e));
            if let Some(core_request) = self.core_request() {
                core_request.response(CoreOperationOutput::Transfer(
                    TransferOperationOutput::TransferResourceProgressUpdate(progress)
                )).await;
                core_request.response(CoreOperationOutput::Error(
                    CoreError::WebRtc(format!("Failed to create writer: {:?}", e))
                )).await;
            }
            return Err(WebRtcErrors::InvalidDelimiter(format!("Failed to create writer: {:?}", e)));
        }
    };

    loop {
        let Some(packet) = rx.next().await else {
            log::warn!("Channel closed before receiving end delimiter");
            progress.fail("Channel closed during transfer".to_string());
            if let Some(core_request) = self.core_request() {
                core_request.response(CoreOperationOutput::Transfer(
                    TransferOperationOutput::TransferResourceProgressUpdate(progress)
                )).await;
                core_request.response(CoreOperationOutput::Error(
                    CoreError::WebRtc("Channel closed during transfer".to_string())
                )).await;
            }
            break;
        };

        if TransferDelimiterShema::from_end_packet(&packet, session_order_id).is_ok() {
            log::info!("Received end delimiter for resource {}", resource_id);

            // Mark as complete and emit
            progress.complete();
            if let Some(core_request) = self.core_request() {
                core_request.response(CoreOperationOutput::Transfer(
                    TransferOperationOutput::TransferResourceProgressUpdate(progress)
                )).await;

                core_request.response(CoreOperationOutput::Transfer(
                    TransferOperationOutput::TransferCompleted(TransferSessionStatus::Success)
                )).await;
            }
            break;
        }

        if TransferDelimiterShema::from_hold_packet(&packet).is_ok() {
            continue;
        }

        // Write data
        let bytes = Bytes::from(packet.to_vec());
        let packet_size = bytes.len() as u64;

        if let Err(e) = writer.write(bytes).await {
            log::error!("Failed to write data: {:?}", e);
            progress.fail(format!("Write failed: {:?}", e));
            if let Some(core_request) = self.core_request() {
                core_request.response(CoreOperationOutput::Transfer(
                    TransferOperationOutput::TransferResourceProgressUpdate(progress)
                )).await;
                core_request.response(CoreOperationOutput::Error(
                    CoreError::WebRtc(format!("Write failed: {:?}", e))
                )).await;
            }
            break;
        }

        // Update progress
        progress.update_progress(packet_size);

        // Emit progress update (throttled by core_request's throttling mechanism)
        if let Some(core_request) = self.core_request() {
            core_request.response_throttle(CoreOperationOutput::Transfer(
                TransferOperationOutput::TransferResourceProgressUpdate(progress.clone())
            )).await;
        }
    }

    prefix_channels.lock().await.remove(&prefix);
    log::info!("Completed download for resource {}", resource_id);

    Ok(())
}
```

#### 2.2 Update webrtc.rs::download_resource()
**Location:** `shared/src/protocol/webrtc/webrtc.rs:159-183`

**Changes:**
- Accept `core_request: CoreRequest` parameter
- Call `peer.start_core_stream(core_request)` before downloading

**Implementation:**
```rust
pub async fn download_resource(
    &self,
    peer_id: String,
    session_order_id: u64,
    resource_order_id: u64,
    core_request: CoreRequest,
) -> Result<(), WebRtcErrors> {
    let peer_id = PeerId(peer_id.parse()?);
    if let Some(peer) = self.shared_context.get_peer(&peer_id).await.and_then(|p| p.upgrade()) {
        peer.start_core_stream(core_request);

        // Create a minimal LocalResource for download
        let resource = LocalResource {
            order_id: resource_order_id,
            name: String::new(),
            size: 0,
            path: LocalResourcePath::RelativePath {
                path: format!("received/session_{}/resource_{}", session_order_id, resource_order_id),
                is_private: false,
            },
            thumbnail_path: None,
            r#type: crate::entities::local_resource::ResourceType::File,
        };

        peer.request_resource_download(session_order_id, resource).await
    } else {
        Err(WebRtcErrors::ConnectionNotFound(peer_id))
    }
}
```

#### 2.3 Update executor/p2p.rs
**Location:** `shared/src/shell/executor/p2p.rs:60-62`

**Changes:**
- Pass `request` (CoreRequest) to `download_resource`

**Implementation:**
```rust
P2POperation::DownloadResource { peer_id, session_order_id, resource_order_id } => {
    self.web_rtc().download_resource(peer_id, session_order_id, resource_order_id, request).await?;
    Ok(CoreOperationOutput::None)
}
```

#### 2.4 Update commands.rs::request_download_resource()
**Location:** `shared/src/app/transfer/commands.rs:335-342`

**Changes:**
- Convert to streaming operation following upload pattern
- Handle `TransferResourceProgressUpdate` and `TransferCompleted` events
- Update model with progress

**Implementation:**
```rust
pub async fn request_download_resource(
    &self,
    peer_id: String,
    session_order_id: u64,
    resource_order_id: u64
) -> Result<(), CoreError> {
    let session_id = TransferSessionId {
        order_id: Some(session_order_id.to_string()),
        transfer_type: Some(TransferType::Receive)
    };

    let mut stream = self.stream_from_shell(
        P2POperation::DownloadResource {
            peer_id,
            session_order_id,
            resource_order_id
        }.into()
    );

    while let Some(output) = stream.next().await {
        match output {
            CoreOperationOutput::Transfer(transfer_output) => match transfer_output {
                TransferOperationOutput::TransferResourceProgressUpdate(progress) => {
                    if progress.is_completed() {
                        log::info!("Download completed: resource {} with status {:?}",
                            progress.resource_order_id, progress.status);
                    }

                    // Update model with progress
                    self.update_model(TransferSessionModelEvent::Update(
                        session_id.clone(),
                        progress.into()
                    ));
                }
                TransferOperationOutput::TransferCompleted(status) => {
                    log::info!("Download stream completed with status: {:?}", status);
                    break;
                }
                other => {
                    log::warn!("Unexpected transfer output during download: {other:?}");
                }
            },
            CoreOperationOutput::Error(error) => {
                log::error!("Download error: {error:?}");
                self.run(DialogOperation::toast(format!("Download failed: {error}"))).await;
                return Err(error);
            }
            _ => continue,
        }
    }

    Ok(())
}
```

---

## Summary of Changes

### Files Modified

#### Phase 1: Session Detail Streaming
1. **peer.rs**
   - `request_session_detail()`: Return `Result<(), WebRtcErrors>`, stream events via CoreRequest
   - Remove `SessionDetailReceived` emission at lines 619-625

2. **webrtc.rs**
   - `view_session_detail()`: Accept `CoreRequest`, call `peer.start_core_stream()`

3. **executor/p2p.rs**
   - `ViewSessionDetail` case: Pass `request` to `view_session_detail()`

4. **commands.rs**
   - `request_session_detail()`: Convert to streaming, build session from events
   - **REMOVE** `handle_session_detail_received()` function

5. **Event cleanup** (multiple files)
   - Remove `P2POperationOutput::SessionDetailReceived`
   - Remove `P2POperationOutput::SessionDetailFailed`
   - Remove `TransferEvent::SessionDetailReceived`
   - Remove `TransferEvent::SessionDetailFailed`
   - Remove handlers in nearby/command.rs and module.rs

#### Phase 2: Download Resource Streaming
1. **peer.rs**
   - `request_resource_download()`: Create `TransferProgress`, emit progress updates via CoreRequest

2. **webrtc.rs**
   - `download_resource()`: Accept `CoreRequest`, call `peer.start_core_stream()`

3. **executor/p2p.rs**
   - `DownloadResource` case: Pass `request` to `download_resource()`

4. **commands.rs**
   - `request_download_resource()`: Convert to streaming, handle progress events

### Events Used (Existing)
- `TransferOperationOutput::TransferResourceProgressUpdate` - progress updates (existing `TransferProgress` struct)
- `TransferOperationOutput::ThumbnailUpdated` - thumbnails
- `TransferOperationOutput::TransferCompleted` - completion signal
- `CoreOperationOutput::Error` - errors

### Events Removed
- `P2POperationOutput::SessionDetailReceived` - replaced with streaming
- `P2POperationOutput::SessionDetailFailed` - replaced with streaming errors
- `TransferEvent::SessionDetailReceived` - no longer needed
- `TransferEvent::SessionDetailFailed` - no longer needed
- `TransferOperationOutput::SessionDetailReceived` - **possibly keep temporarily** for backward compatibility

### No New Enum Variants Needed
All events already exist in `TransferOperationOutput` - we just use them in a streaming pattern!

---

## Architecture Decision Required

### BLOCKER: How to pass `LocalResource` in Phase 1?

**Problem:** In `request_session_detail()`, we need to emit each received resource so that `commands.rs` can build the `TransferSession.resources` vector. However, `TransferProgress` (used in `TransferResourceProgressUpdate`) doesn't contain a `LocalResource` field.

**Current `TransferProgress` structure:**
```rust
pub struct TransferProgress {
    pub resource_order_id: u64,
    pub file_size: u64,
    total_bytes_counter: u64,
    bytes_per_second: u64,
    start_time_utc_ms: u64,
    bytes_sec_counter: u64,
    last_update_time_ms: u64,
    pub transfer_type: TransferType,
    pub status: TransferStatus,
}
```

**Options:**

**Option A: Extend TransferProgress (RECOMMENDED)**
- Add `resource: Option<LocalResource>` field to `TransferProgress`
- When receiving session detail: include resource in first progress update
- When updating progress during download: set resource to `None`
- **Pros:** Minimal changes, reuses existing event, clear semantics
- **Cons:** Adds optional field that's only used in one scenario

**Option B: Create new ResourceReceived event**
- Add `TransferOperationOutput::ResourceReceived { resource: LocalResource }` variant
- Emit separate event for resource metadata vs. progress
- **Pros:** Clean separation of concerns
- **Cons:** Adds new event type, more complex flow

**Option C: Keep SessionDetailReceived temporarily**
- Keep `SessionDetailReceived` event for Phase 1, only refactor Phase 2
- Defer session detail streaming to future iteration
- **Pros:** Smaller initial change, lower risk
- **Cons:** Inconsistent patterns, delayed benefit

**Option D: Use model events directly**
- Bypass `TransferOperationOutput` and emit `TransferSessionModelEvent` directly from peer
- **Pros:** Most direct path
- **Cons:** Violates layering, couples protocol layer to UI model

**DECISION NEEDED:** Please choose an option before proceeding with implementation.
