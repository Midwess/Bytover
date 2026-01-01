# Enhancement: Download All Progress UI

## Problem
When pressing "Download All", the button doesn't show progress. The `session_resource` is being added to `session.resources` list instead of the `session_resource` field due to using `ResourceUpdate` event.

## Root Cause
In `commands.rs:550-553`, when updating with `session.session_resource.clone().unwrap().into()`, the `LocalResource` converts to `TransferSessionUpdateEvent::ResourceUpdate`, which calls `replace_resource()` - adding to the resources list instead of setting `session_resource`.

## Implementation Steps

### 1. Backend: Add new update event variant
**File:** `shared/src/app/core/model_events.rs`
- Add `SessionResourceUpdate(LocalResource)` variant to `TransferSessionUpdateEvent` enum

### 2. Backend: Implement UpdateAction for session_resource
**File:** `shared/src/entities/transfer_session.rs`
- Create a wrapper type `SessionResourceUpdate(LocalResource)`
- Implement `UpdateAction<TransferSession>` that sets `data.session_resource = Some(self.0)`

### 3. Backend: Use new event in commands
**File:** `shared/src/app/transfer/commands.rs` (line ~550)
- Change from: `session.session_resource.clone().unwrap().into()`
- Change to: `SessionResourceUpdate(session_resource.clone()).into()`

### 4. Backend: Add download_all fields to ViewModel
**File:** `shared/src/app/view_models/receive_session.rs`
- Add to `ReceiveSessionViewModel`:
  ```rust
  pub download_all_enabled: bool,              // false for cloud/public sessions
  pub download_all_progress: Option<f32>,      // 0.0 - 1.0
  pub download_all_in_progress: bool,
  pub download_all_completed: bool,
  ```

### 5. Backend: Populate download_all fields in view function
**File:** `shared/src/app/transfer/module.rs` (view function ~line 613)
- Set `download_all_enabled`: `!is_cloud && is_p2p && !resources.is_empty()`
- Find progress with `resource_order_id == u64::MAX` (the aggregate progress)
- Map to the new ViewModel fields:
  ```rust
  let download_all_progress = it.progress.iter()
      .find(|p| p.resource_order_id == u64::MAX);

  download_all_enabled: !it.target.is_public() && is_p2p && !it.resources.is_empty(),
  download_all_progress: download_all_progress.map(|p| p.percentage() as f32),
  download_all_in_progress: download_all_progress.map(|p| !p.is_completed()).unwrap_or(false),
  download_all_completed: download_all_progress.map(|p| p.is_success()).unwrap_or(false),
  ```

### 6. Frontend: Update HeaderInfo to show progress
**File:** `web-next/app/transfer/receive_board.tsx` (HeaderInfo function ~line 85)
- Use `download_all_enabled` flag to conditionally show the Download All UI
- When `download_all_in_progress` is true, show `DownloadButtonWithProgress` instead of button
- Example:
  ```tsx
  {download_all_enabled && !isCompleted && (
      <div className="ml-auto">
          {download_all_in_progress || download_all_completed ? (
              <DownloadButtonWithProgress
                  progress={download_all_progress ?? 0}
                  isReady={true}
                  isCompleted={download_all_completed}
                  isInProgress={download_all_in_progress}
                  onDownloadClick={onDownloadAll}
                  size={40}
                  strokeWidth={4}
              />
          ) : (
              <Button variant="outline" size="sm" onClick={onDownloadAll}>
                  <Download className="h-4 w-4" />
                  Download All
              </Button>
          )}
      </div>
  )}
  ```

## Files to Modify
1. `shared/src/app/core/model_events.rs` - Add event variant
2. `shared/src/entities/transfer_session.rs` - Add wrapper type + UpdateAction impl
3. `shared/src/app/transfer/commands.rs` - Use new event
4. `shared/src/app/view_models/receive_session.rs` - Add ViewModel fields
5. `shared/src/app/transfer/module.rs` - Populate fields in view()
6. `web-next/app/transfer/receive_board.tsx` - Show progress UI
