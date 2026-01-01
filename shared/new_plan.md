# Support feature: Download All Resources for P2P Session

## Overview
Allow receivers to download all resources from a P2P session into a single zip file, with proper progress tracking and resource management.

## Architecture Review

### Current State
- ✅ `OpfsZipWriter` already implemented in `web-next/wasm/src/file_system/zip_writer.rs`
- ✅ `TransferSession` has `resources: Vec<LocalResource>` and `progress: Vec<TransferProgress>`
- ✅ Transfer commands in `shared/src/app/transfer/commands.rs` handle single resource downloads
- ✅ P2P operations support `DownloadResource` for individual files
- ✅ Transfer events in `TransferEvent` enum handle resource-level operations

### Design Decisions

**Question 1: How to track download-all operation?**
- ✅ Add new field to `TransferSession`: `session_resource: Option<LocalResource>`
- ✅ Add TransferProgress to existing `session.progress` vector
- ❌ Don't add to `session.resources` (that remains the actual individual files)
- **Reason**: Separates actual resources from download-all operation, cleaner data model

**Question 2: Structure of the session_resource**
- `order_id: u64::MAX` (special ID to identify download-all progress)
- `name: "<session-order-id>.zip"`
- `size:` Sum of all individual resource sizes
- `r#type: ResourceType::File`
- `path: LocalResourcePath::RelativePath { path: "opfs://<session-order-id>.zip", is_private: false }`

**Question 3: Where should zip file path be determined?**
- ✅ The receiver side (WASM/OPFS) should handle zip file creation
- Path format: `opfs://<session-order-id>.zip` (simple, flat structure)
- Individual entries: `<resource-name>` or `<folder>/<file>` preserving structure

## Implementation Plan

### Phase 1: Modify TransferSession Entity

**1.1 Add session_resource field**
- Location: `shared/src/entities/transfer_session.rs:74`
- Add new field to `TransferSession` struct:
  ```rust
  pub struct TransferSession {
      pub order_id: u64,
      pub resources: Vec<LocalResource>,
      pub session_resource: Option<LocalResource>,  // NEW: For download-all
      pub progress: Vec<TransferProgress>,
      // ... rest of fields
  }
  ```
- Update all constructor methods (`p2p`, `public`, `from_public_overview`) to initialize `session_resource: None`

### Phase 2: Frontend Event & Command (Shared Module)

**2.1 Add DownloadAll event to TransferEvent enum**
- Location: `shared/src/app/transfer/module.rs:52`
- Add new variant:
  ```rust
  RequestDownloadAll {
      session_order_id: u64,
      peer_id: String,
  }
  ```

**2.2 Add DownloadAll command handler**
- Location: `shared/src/app/transfer/commands.rs`
- Create `request_download_all_resources` method similar to `request_download_resource:432`
- **Implementation steps**:

  ```rust
  pub async fn request_download_all_resources(
      &self,
      mut session: TransferSession
  ) -> Result<(), CoreError> {
      // 1. Fire operation to generate paths (through shell)
      let resource_names: HashMap<u64, String> = session.resources
          .iter()
          .map(|r| (r.order_id, r.name.clone()))
          .collect();

      let zip_paths = self.run(TransferSessionPersistentOperation::generate_zip_download_paths(
          session.order_id,
          resource_names
      )).await?;

      // 2. Update resources with zip_entry paths
      for resource in &mut session.resources {
          if let Some(path) = zip_paths.resource_paths.get(&resource.order_id) {
              resource.path = path.clone();
          }
      }

      // 3. Create session_resource
      let total_size = session.resources.iter().map(|r| r.size).sum();
      session.session_resource = Some(LocalResource {
          order_id: u64::MAX,
          name: format!("{}.zip", session.order_id),
          size: total_size,
          path: zip_paths.session_path.clone(),
          thumbnail_path: None,
          r#type: ResourceType::File,
      });

      // 4. Create aggregate progress
      let mut aggregate_progress = TransferProgress::new(
          u64::MAX,
          total_size,
          TransferType::Receive
      );
      session.progress.push(aggregate_progress.clone());

      // 5. Update model with modified session
      self.update_model(TransferSessionModelEvent::Update(
          session.id(),
          session.clone().into()
      ));

      // 6. Fire download-all operation (returns stream)
      let mut stream = self.stream_from_shell(
          P2POperation::DownloadAllResources {
              peer_id: session.peer_id().unwrap(),
              session_id: session.order_id,
              session_path: zip_paths.session_path,
              resources: session.resources.clone()
          }.into()
      );

      // 7. Process progress updates from stream
      // Track progress for each individual resource
      let mut resource_progress_map: HashMap<u64, u64> = HashMap::new();

      while let Some(output) = stream.next().await {
          match output {
              CoreOperationOutput::Transfer(
                  TransferOperationOutput::TransferResourceProgressUpdate(progress)
              ) => {
                  // Update this specific resource's progress
                  resource_progress_map.insert(
                      progress.resource_order_id,
                      progress.total_bytes_counter
                  );

                  // SUM all resources to get total downloaded bytes
                  let total_downloaded: u64 = resource_progress_map.values().sum();

                  // Calculate bytes downloaded since last update
                  let previous_total = aggregate_progress.total_bytes_counter;
                  let bytes_delta = total_downloaded - previous_total;

                  // Update aggregate progress with delta
                  aggregate_progress.update_progress(bytes_delta);

                  // Update model
                  self.update_model(TransferSessionModelEvent::Update(
                      session.id(),
                      aggregate_progress.clone().into()
                  ));

                  log::info!(
                      "Resource {} progress: {}/{} bytes. Total: {}/{} bytes ({}%)",
                      progress.resource_order_id,
                      progress.total_bytes_counter,
                      progress.file_size,
                      total_downloaded,
                      total_size,
                      (total_downloaded as f64 / total_size as f64 * 100.0)
                  );
              }
              CoreOperationOutput::Transfer(
                  TransferOperationOutput::TransferCompleted(status)
              ) => {
                  log::info!("Download-all completed with status: {:?}", status);
                  break;
              }
              CoreOperationOutput::Error(e) => {
                  log::error!("Download-all error: {:?}", e);
                  aggregate_progress.fail(e.to_string());
                  self.update_model(TransferSessionModelEvent::Update(
                      session.id(),
                      aggregate_progress.into()
                  ));
                  return Err(e);
              }
              _ => continue
          }
      }

      Ok(())
  }
  ```

### Phase 3: P2P Operation Support

**3.1 Add GenerateZipDownloadPaths operation**
- Location: `shared/src/app/operations/persistent.rs` (or new TransferSessionPersistentOperation)
- Add operation to generate paths:
  ```rust
  GenerateZipDownloadPaths {
      session_order_id: u64,
      resource_names: HashMap<u64, String>
  }
  ```
- **Returns `ZipDownloadPaths` struct** with:
  - `resource_paths: HashMap<u64, LocalResourcePath>` - zip_entry:// paths for each resource
  - `session_path: LocalResourcePath` - final zip file path for the session
- Example return value:
  ```rust
  ZipDownloadPaths {
      resource_paths: {
          1: LocalResourcePath::RelativePath { path: "opfs://zip_entry://123.zip/photo.jpg", ... },
          2: LocalResourcePath::RelativePath { path: "opfs://zip_entry://123.zip/doc.pdf", ... }
      },
      session_path: LocalResourcePath::RelativePath { path: "opfs://123.zip", ... }
  }
  ```

**3.2 Add DownloadAllResources operation**
- Location: `shared/src/app/operations/p2p.rs:18`
- Add new variant to `P2POperation` enum:
  ```rust
  DownloadAllResources {
      peer_id: String,
      session_id: u64,
      session_path: LocalResourcePath,
      resources: Vec<LocalResource>  // Already have zip_entry:// paths
  }
  ```

**3.3 Handler streams progress updates**
- The shell/executor handler for `DownloadAllResources`:
  - Calls `peer.download_all_resources()`
  - Streams `TransferResourceProgressUpdate` events back to command
  - Sends `TransferCompleted` when done

### Phase 4: Repository - Generate Zip Entry Paths

**4.1 Add response struct and methods to TransferSessionRepository trait**
- Location: `shared/src/repository/transfer_session.rs`
- Add response struct:
  ```rust
  pub struct ZipDownloadPaths {
      pub resource_paths: HashMap<u64, LocalResourcePath>,  // Paths for each resource
      pub session_path: LocalResourcePath,                   // Path for the zip file itself
  }
  ```
- Add new methods to trait:
  ```rust
  async fn generate_zip_download_paths(
      &self,
      session_order_id: u64,
      resource_names: HashMap<u64, String>
  ) -> Result<ZipDownloadPaths, PersistenceError>;

  async fn start_download_session(
      &self,
      zip_path: LocalResourcePath  // Use the generated path from generate_zip_download_paths
  ) -> Result<(), PersistenceError>;

  async fn stop_download_session(
      &self,
      zip_path: LocalResourcePath  // Same path used in start
  ) -> Result<(), PersistenceError>;
  ```

**4.2 Implement in WASM repository**
- Location: `web-next/wasm/src/repository/transfer_session.rs:157`
- Implement the methods:
  ```rust
  async fn generate_zip_download_paths(
      &self,
      session_order_id: u64,
      resource_names: HashMap<u64, String>
  ) -> Result<ZipDownloadPaths, PersistenceError> {
      let mut resource_paths = HashMap::new();

      // Generate zip entry path for each resource
      for (resource_order_id, resource_name) in resource_names {
          let path = format!("opfs://zip_entry://{}.zip/{}",
                            session_order_id, resource_name);
          resource_paths.insert(
              resource_order_id,
              LocalResourcePath::RelativePath {
                  path,
                  is_private: false
              }
          );
      }

      // Generate path for the final zip file (session resource)
      let session_path = LocalResourcePath::RelativePath {
          path: format!("opfs://{}.zip", session_order_id),
          is_private: false
      };

      Ok(ZipDownloadPaths {
          resource_paths,
          session_path,
      })
  }

  async fn start_download_session(
      &self,
      zip_path: LocalResourcePath
  ) -> Result<(), PersistenceError> {
      // Extract filename from path: "opfs://123.zip" -> "123.zip"
      let zip_filename = zip_path.as_string()
          .strip_prefix("opfs://")
          .unwrap_or(&zip_path.as_string())
          .to_string();

      // Send CreateZipWriter operation to OPFS worker
      let operation = OpfsOperation {
          file_path: zip_filename.clone(),
          operation: FileOperation::CreateZipWriter { zip_filename }
      };

      // Call OPFS worker and wait for response
      // (implementation depends on how OPFS worker is invoked)

      Ok(())
  }

  async fn stop_download_session(
      &self,
      zip_path: LocalResourcePath
  ) -> Result<(), PersistenceError> {
      // Extract filename from path: "opfs://123.zip" -> "123.zip"
      let zip_filename = zip_path.as_string()
          .strip_prefix("opfs://")
          .unwrap_or(&zip_path.as_string())
          .to_string();

      // Send FinalizeZip operation to OPFS worker
      let operation = OpfsOperation {
          file_path: zip_filename.clone(),
          operation: FileOperation::FinalizeZip { zip_filename }
      };

      // Call OPFS worker and wait for response

      Ok(())
  }
  ```

### Phase 5: WASM/OPFS Zip File Handling

**5.1 Add zip_writers HashMap to OpfsWorker**
- Location: `web-next/wasm/src/web_worker/opfs.rs:104`
- Add new field to `OpfsWorker` struct:
  ```rust
  use crate::file_system::zip_writer::OpfsZipWriter;

  pub struct OpfsWorker {
      root: OnceCell<Arc<FileSystemDirectoryHandle>>,
      device_files: AMutex<HashMap<String, AMutex<DeviceFile>>>,
      file_handles: AMutex<HashMap<String, AMutex<FileSystemSyncAccessHandle>>>,
      cursors: AMutex<HashMap<u32, AMutex<Box<dyn IOCursor>>>>,
      device_folders: AMutex<HashMap<String, AMutex<DeviceFolder>>>,
      zip_writers: AMutex<HashMap<String, AMutex<OpfsZipWriter>>>,  // NEW
      id_gen: Arc<AtomicU32>
  }
  ```
- Initialize in `create()` method: `zip_writers: Default::default()`

**5.2 Enhance Write operation to detect zip paths**
- Location: `web-next/wasm/src/web_worker/opfs.rs:308` (FileOperation::Write handler)
- Detect `zip_entry://` marker in `file_path`
- Example paths:
  - Regular file: `opfs://photo.jpg` or `photo.jpg`
  - Zip entry: `opfs://zip_entry://123123.zip/photo.jpg`
- Parse logic:
  ```rust
  if file_path.contains("zip_entry://") {
      // Handle zip entry write
      // Split by "zip_entry://" and take the part after it
      let path = file_path.split("zip_entry://").nth(1).unwrap();
      // Split: "123123.zip/photo.jpg" -> ["123123.zip", "photo.jpg"]
      let mut parts = path.splitn(2, '/');
      let zip_filename = parts.next().unwrap();  // "123123.zip"
      let entry_name = parts.next().unwrap();     // "photo.jpg"

      // Get or create OpfsZipWriter from HashMap
      // Call write() on the zip writer
  } else {
      // Existing regular file write logic
  }
  ```

**5.3 Enhance Cursor operation to handle zip entries**
- Location: `web-next/wasm/src/web_worker/opfs.rs:195` (FileOperation::Cursor handler)
- When creating cursor, check if path contains `zip_entry://`
- If yes:
  ```rust
  if file_path.contains("zip_entry://") {
      let path = file_path.split("zip_entry://").nth(1).unwrap();
      let (zip_filename, entry_name) = path.split_once('/').unwrap();

      // Get or create OpfsZipWriter from HashMap
      let zip_writer = /* get or create */;

      // Automatically create new entry
      zip_writer.new_entry(entry_name).await?;

      // Return cursor (implementation depends on how cursor works with zip writer)
  }
  ```

**5.4 Add new FileOperation variants for zip management**
- Location: `web-next/wasm/src/web_worker/opfs.rs:38`
- Add to `FileOperation` enum:
  ```rust
  CreateZipWriter {
      zip_filename: String  // e.g., "123123.zip"
  },
  FinalizeZip {
      zip_filename: String
  }
  ```

**5.5 Implement zip operation handlers**
- `CreateZipWriter`: Open OPFS file handle, create `OpfsZipWriter`, store in HashMap
- `Cursor` (when path has zip_entry): Automatically call `new_entry()`, return appropriate cursor
- `Write` (when path is zip entry): Call `zip_writer.write(data).await`
- `FinalizeZip`: Call `zip_writer.finalize().await`, remove from HashMap

### Phase 6: Resource Download Coordination

**6.1 Add download_all_resources method to peer.rs**
- Location: `shared/src/protocol/webrtc/peer.rs` (new method)
- **Add new method to handle download-all orchestration**:
  ```rust
  pub async fn download_all_resources(
      &self,
      core_request: CoreRequest,
      session_id: u64,
      session_path: LocalResourcePath,
      resources: Vec<LocalResource>,
      mut progress: TransferProgress  // Aggregate progress with order_id: u64::MAX
  ) -> Result<(), WebRtcErrors> {
      // 1. Start zip writer
      self.resource_repo.start_download_session(session_path.clone()).await?;

      // 2. Download each resource sequentially
      for resource in resources {
          // resource.path is already "opfs://zip_entry://123.zip/photo.jpg"

          // Use existing request_resource_download (line 649)
          // But use aggregate progress instead of individual
          self.request_resource_download(
              core_request.clone(),
              session_id,
              resource,
              progress.clone()  // All resources update same progress
          ).await?;

          // Update aggregate progress
          core_request
              .response(TransferOperationOutput::TransferResourceProgressUpdate(progress.clone()))
              .await;
      }

      // 3. Finalize zip
      self.resource_repo.stop_download_session(session_path).await?;

      Ok(())
  }
  ```

**6.2 Existing request_resource_download still works**
- Location: `shared/src/protocol/webrtc/peer.rs:649`
- **No changes needed** - already handles zip_entry paths:
  ```rust
  // Line 712: Creates writer with resource.path
  let mut writer = resource_repo.write(resource.path.clone(), compressed).await?;
  //                                    ↑
  //                        path is "opfs://zip_entry://123.zip/photo.jpg"

  // Line 737: Writes bytes → streams to zip entry
  writer.d_write(bytes).await?;
  ```

**6.3 Complete download-all flow example**

```rust
// ========================================
// In commands.rs: request_download_all_resources()
// ========================================
pub async fn request_download_all_resources(&self, mut session: TransferSession) {
    // 1. Fire operation to generate paths (not direct repository call!)
    let resource_names = session.resources.iter()
        .map(|r| (r.order_id, r.name.clone()))
        .collect();

    let paths = self.run(TransferSessionPersistentOperation::generate_zip_download_paths(
        session.order_id,
        resource_names
    )).await?;

    // 2. Update resources with zip_entry paths
    for resource in &mut session.resources {
        resource.path = paths.resource_paths[&resource.order_id].clone();
        // resource.path is now: "opfs://zip_entry://123.zip/photo.jpg"
    }

    // 3. Create session_resource
    session.session_resource = Some(LocalResource {
        order_id: u64::MAX,
        name: format!("{}.zip", session.order_id),
        size: total_size,
        path: paths.session_path.clone(), // "opfs://123.zip"
        r#type: ResourceType::File,
        ...
    });

    // 4. Create aggregate progress
    let mut aggregate_progress = TransferProgress::new(u64::MAX, total_size, TransferType::Receive);
    session.progress.push(aggregate_progress.clone());

    // Update model
    self.update_model(TransferSessionModelEvent::Update(session.id(), session.clone().into()));

    // 5. Fire download-all operation (returns stream!)
    let mut stream = self.stream_from_shell(
        P2POperation::DownloadAllResources {
            peer_id: session.peer_id().unwrap(),
            session_id: session.order_id,
            session_path: paths.session_path,
            resources: session.resources.clone()
        }.into()
    );

    // 6. Process stream of progress updates
    // Track each resource's progress to calculate total
    let mut resource_progress_map: HashMap<u64, u64> = HashMap::new();

    while let Some(output) = stream.next().await {
        match output {
            CoreOperationOutput::Transfer(
                TransferOperationOutput::TransferResourceProgressUpdate(progress)
            ) => {
                // Track this resource's bytes
                resource_progress_map.insert(
                    progress.resource_order_id,
                    progress.total_bytes_counter
                );

                // SUM all resources = total downloaded
                let total_downloaded: u64 = resource_progress_map.values().sum();

                // Calculate delta and update aggregate
                let bytes_delta = total_downloaded - aggregate_progress.total_bytes_counter;
                aggregate_progress.update_progress(bytes_delta);

                // Update UI with total progress
                self.update_model(TransferSessionModelEvent::Update(
                    session.id(),
                    aggregate_progress.clone().into()
                ));
            }
            CoreOperationOutput::Transfer(TransferOperationOutput::TransferCompleted(_)) => {
                break;  // ✅ Download complete!
            }
            _ => continue
        }
    }
}

// ========================================
// In shell/executor: Handle DownloadAllResources operation
// ========================================
// This is called by the shell when commands fires P2POperation::DownloadAllResources
async fn handle_download_all_resources(
    peer: &WebRtcPeer,
    session_id: u64,
    session_path: LocalResourcePath,
    resources: Vec<LocalResource>
) -> impl Stream<Item = CoreOperationOutput> {
    // Call peer.download_all_resources() and stream back progress
    peer.download_all_resources(core_request, session_id, session_path, resources).await
}

// ========================================
// In peer.rs: download_all_resources() (NEW METHOD)
// ========================================
pub async fn download_all_resources(
    &self,
    core_request: CoreRequest,
    session_id: u64,
    session_path: LocalResourcePath,
    resources: Vec<LocalResource>
) -> Result<(), WebRtcErrors> {
    // 1. Start zip writer
    self.resource_repo.start_download_session(session_path.clone()).await?;
    //    ↓ extracts "123.zip" from "opfs://123.zip"
    //    ↓ sends CreateZipWriter("123.zip") to OPFS worker
    //    ↓ OpfsWorker creates OpfsZipWriter, stores in HashMap

    // 2. Download each resource (progress streams back to command!)
    for (index, resource) in resources.iter().enumerate() {
        // resource.path = "opfs://zip_entry://123.zip/photo.jpg"

        // Create individual progress for this resource
        let resource_progress = TransferProgress::new(
            resource.order_id,
            resource.size,
            TransferType::Receive
        );

        self.request_resource_download(
            core_request.clone(),
            session_id,
            resource.clone(),
            resource_progress
        ).await?;
        //    ↓ Inside request_resource_download:649
        //    ↓ Line 712: writer = resource_repo.write(resource.path, ...)
        //    ↓           Repository creates Cursor("opfs://zip_entry://123.zip/photo.jpg")
        //    ↓           OPFS worker detects zip_entry://
        //    ↓           OPFS worker calls zip_writer.new_entry("photo.jpg")
        //    ↓ Line 737: writer.d_write(bytes)
        //    ↓           Writes stream to zip entry
        //    ↓ Line 740: core_request.response(TransferResourceProgressUpdate)
        //    ↓           Progress streams back to command.rs!

        log::info!("Completed resource {}/{}", index + 1, resources.len());
    }

    // 3. Finalize zip
    self.resource_repo.stop_download_session(session_path).await?;
    //    ↓ extracts "123.zip" from "opfs://123.zip"
    //    ↓ sends FinalizeZip("123.zip") to OPFS worker
    //    ↓ OpfsWorker calls zip_writer.finalize()
    //    ↓ Removes from HashMap

    // 4. Send completion
    core_request.response(
        TransferOperationOutput::TransferCompleted(TransferSessionStatus::Success)
    ).await;

    Ok(())
}
```

**6.4 Handle download completion**
- Track: N resources completed out of M total
- Only finalize zip after ALL resources successfully downloaded
- Handle partial failures (some resources failed)

### Phase 7: Progress Tracking & UI Updates

**7.1 Aggregate progress calculation**
- The download-all TransferProgress (order_id: u64::MAX) is updated as bytes stream in
- Progress calculation: `total_bytes_downloaded / total_size_of_synthetic_resource`
- UI treats it like any other resource download
- Speed and percentage computed automatically by existing TransferProgress logic

**7.2 Error handling**
- If any resource fails: mark download-all as failed
- Cleanup: remove partial zip file
- Allow retry of entire operation

## Key Technical Considerations

### Path Format Design
```
Regular download:     opfs://<resource-name>
Download-all zip:     opfs://<session-id>.zip
Zip entry write:      opfs://zip_entry://<session-id>.zip/<entry-name>

Example:
  opfs://photo.jpg                           → Regular file write
  opfs://123123.zip                          → Final zip file location
  opfs://zip_entry://123123.zip/photo.jpg    → Write photo.jpg into 123123.zip
  opfs://zip_entry://123123.zip/docs/file.pdf → Write into zip with folder structure
```

### State Management
- **Do** create synthetic LocalResource with `order_id: u64::MAX` for download-all
- **Do** store in `session.session_resource: Option<LocalResource>`
- **Do** create corresponding TransferProgress and add to `session.progress` vector
- **Don't** add synthetic resource to `session.resources` (keeps actual files separate)
- **Benefit**: Clean separation between actual resources and download-all operation
- **UI Access**: Check `session.session_resource.is_some()` to show download-all option

### Concurrency
- Download resources **sequentially** (one at a time) to avoid:
  - Zip writer state conflicts (currently not thread-safe)
  - Memory pressure from multiple large streams
- Alternative: Make OpfsZipWriter thread-safe with locking

### Memory Efficiency
- Stream data directly to zip entries (don't buffer entire files)
- Use `OpfsZipWriter.write()` for chunked writes
- Leverage existing chunk-based transfer protocol

## Open Questions (Need User Input)

1. **Download Strategy**: Sequential or parallel resource downloads?
   - Sequential: Simpler, less memory, current zip writer limitation
   - Parallel: Faster, needs zip writer synchronization

2. **Failure Handling**: If 1 of 10 resources fails, should we:
   - Keep partial zip with successful resources?
   - Delete entire zip and mark as failed?

3. **UI/UX**: Should download-all show:
   - Only aggregate progress?
   - Aggregate + expandable list of individual resources?

4. **Zip Structure**: For folders, should we:
   - Flatten all files?
   - Preserve folder hierarchy in zip?

## Summary: peer.rs Download-All Implementation

**Answer: peer.rs needs ONE new method!**

### New Method: `download_all_resources()`
Orchestrates the download-all lifecycle:
1. Calls `resource_repo.start_download_session(session_path)`
2. Loops through resources, calling existing `request_resource_download()`
3. Calls `resource_repo.stop_download_session(session_path)`

### Existing Method: `request_resource_download()` (line 649)
**No changes needed** - already works with zip_entry paths:
- **Line 712**: `resource_repo.write(resource.path, ...)` with `zip_entry://` path
- **Repository layer**: Detects `zip_entry://` and routes to OPFS
- **OPFS worker**: Auto-creates zip entries, streams to OpfsZipWriter
- **Line 737**: `writer.d_write(bytes)` streams directly into zip

**The download-all feature works through layered architecture:**
```
Commands (Phase 2.2)
    ↓ calls peer.download_all_resources(session_id, session_path, resources)
peer.rs (NEW method)
    ↓ start_download_session() → loop resources → stop_download_session()
    ↓ calls existing request_resource_download() for each resource
peer.rs (EXISTING method: request_resource_download)
    ↓ calls resource_repo.write("opfs://zip_entry://123.zip/photo.jpg")
Repository (new methods)
    ↓ creates Cursor with OPFS worker
OPFS Worker (enhanced)
    ↓ detects zip_entry://, creates entry, writes to OpfsZipWriter
OpfsZipWriter (existing)
    ↓ streams bytes into 123.zip
```

**What changes:**
- Commands: Fires operations, processes stream, aggregates progress
- Operations: Adds GenerateZipDownloadPaths and DownloadAllResources
- Repository: Adds generate_zip_download_paths(), start/stop_download_session()
- OPFS worker: Detects zip_entry://, manages OpfsZipWriter HashMap
- Shell/Executor: Handles DownloadAllResources operation
- **peer.rs: Adds download_all_resources() orchestration method**

**Progress Flow:**
```
peer.rs downloads resource 1 → sends progress: { resource_id: 1, bytes: 100KB }
peer.rs downloads resource 2 → sends progress: { resource_id: 2, bytes: 50KB }
peer.rs downloads resource 1 → sends progress: { resource_id: 1, bytes: 200KB }
    ↓ all streamed via core_request.response(TransferResourceProgressUpdate)
Shell/Executor
    ↓ streams to
commands.rs
    ↓ tracks each resource in HashMap
    ↓ sums: resource_1(200KB) + resource_2(50KB) = 250KB total
    ↓ calculates: 250KB / 1MB total = 25% complete
    ↓ update_model → UI shows "25% (250KB of 1MB)"
```

**Example:**
```rust
// Session has 3 resources: 500KB + 1MB + 500KB = 2MB total

// Progress updates come in:
1. resource_1: 100KB downloaded
   → total: 100KB / 2MB = 5%

2. resource_2: 200KB downloaded
   → total: (100KB + 200KB) / 2MB = 15%

3. resource_1: 300KB downloaded (completed)
   → total: (300KB + 200KB) / 2MB = 25%

4. resource_3: 150KB downloaded
   → total: (300KB + 200KB + 150KB) / 2MB = 32.5%
```

## Files to Modify/Create

### Modify
1. `shared/src/entities/transfer_session.rs:74` - Add `session_resource: Option<LocalResource>` field
2. `shared/src/app/transfer/module.rs` - Add TransferEvent variant
3. `shared/src/app/transfer/commands.rs` - Add download_all command
4. `shared/src/app/operations/p2p.rs` - Add P2POperation variant
5. `shared/src/repository/transfer_session.rs` - Add `ZipDownloadPaths` struct and methods: `generate_zip_download_paths()`, `start_download_session()`, `stop_download_session()`
6. `web-next/wasm/src/repository/transfer_session.rs:157` - Implement all three methods
7. `web-next/wasm/src/web_worker/opfs.rs:104` - Add zip_writers HashMap and handle zip operations
8. `web-next/wasm/src/web_worker/opfs.rs:38` - Add zip FileOperation variants
9. `shared/src/protocol/webrtc/peer.rs` - Add `download_all_resources()` method

### Create
None - all changes in existing files

### Investigate
1. Where is the P2P download handler that receives resources?
2. How is OPFS write currently implemented? (need to find opfs.rs)
3. What's the current chunk size for transfers?