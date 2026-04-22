use crate::file_system::device_file::{DeviceFile, DeviceFolder, WebFile};
use crate::file_system::io::IOReaderBlobImpl;
use crate::file_system::opfs::FileSystemDirectoryHandleExt;
use crate::file_system::path_extension::PICKED_SCHEME;
use crate::file_system::picked_writer::PickedWriter;
use crate::file_system::zip_writer::OpfsZipWriter;
use crate::web_worker::bridge::{TrustedWorkerMessage, WorkerMessage};
use crate::{get_directory, serialize};
use chrono::Utc;
use core_services::local_storage::entry::FileEntry;
use core_services::local_storage::stream::IOCursor;
use core_services::logger::setup;
use devlog_sdk::distributed_id::init_scoped_id_generator;
use futures::lock::Mutex;
use gloo_worker::{HandlerId, Worker, WorkerScope};
use js_sys::Uint8Array;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use n0_future::time::Instant;
use serde::{Deserialize, Serialize};
use shared::entities::local_resource::LocalResourcePath;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Blob, File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemReadWriteOptions, FileSystemRemoveOptions,
    FileSystemSyncAccessHandle,
};

/// Web worker that support file system on browser
/// There are two reasons that we use web worker for file system:
/// + Performance, off load heavy logic out of main thread to avoid blocking.
/// + Browser requirement, browser require us to use web worker to be able to access all opfs features.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpfsOperation {
    pub file_path: String,
    pub operation: FileOperation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileOperation {
    AddFolder {
        path: String,
        files: Vec<WebFile>,
    },
    Cursor {
        buffer_size: usize,
    },
    CursorNext {
        instance_id: u32,
        max: Option<u64>,
        compressed: bool,
    },
    CursorEnd(u32),
    AddFile(DeviceFile),
    GetFile,
    Open,
    WriteNew {
        #[serde(with = "serde_wasm_bindgen::preserve")]
        data: Uint8Array,
    },
    Write {
        #[serde(with = "serde_wasm_bindgen::preserve")]
        data: Uint8Array,
        position: usize,
        decompress: bool,
    },
    Flush,
    FileEntry,
    LocalResourceInstance,
    GenerateSource,
    Blob,
    Init {
        storage_session_id: String,
    },
    CleanUp {
        paths: Vec<String>,
    },
    CreateZipWriter {
        zip_filename: String,
    },
    FinalizeZip {
        zip_filename: String,
    },
    RegisterPickedHandle {
        #[serde(with = "serde_wasm_bindgen::preserve")]
        handle: JsValue,
    },
    FinalizePicked {
        commit: bool,
    },
}

unsafe impl Send for OpfsOperation {}
unsafe impl Sync for OpfsOperation {}

#[derive(Debug, Serialize, Deserialize)]
pub enum OpfsOperationOutput {
    Void,
    Error(#[serde(with = "serde_wasm_bindgen::preserve")] JsValue),
    // Binary and raw size (before compressed) and compression time in microsecond if compressed
    Binary {
        #[serde(with = "serde_wasm_bindgen::preserve")]
        data: Uint8Array,
        raw_size: usize,
        is_compressed_failed: bool,
        compression_time_in_micros: u64,
        read_time_in_micros: u64,
    },
    Written(usize),
    File(#[serde(with = "serde_wasm_bindgen::preserve")] File),
    FileEntry(FileEntry),
    LocalResourceInstance(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    DownloadUrl(String),
    Blob(#[serde(with = "serde_wasm_bindgen::preserve")] Blob),
    Cursor(u32),
}

unsafe impl Send for OpfsOperationOutput {}
unsafe impl Sync for OpfsOperationOutput {}

pub type AMutex<T> = Arc<Mutex<T>>;

pub struct PickedEntry {
    pub writer: PickedWriter,
}

#[derive(Clone)]
pub struct OpfsWorker {
    root: Arc<OnceCell<Arc<FileSystemDirectoryHandle>>>,
    storage_session_id: Arc<OnceCell<String>>,
    device_files: AMutex<HashMap<String, AMutex<DeviceFile>>>,
    file_handles: AMutex<HashMap<String, AMutex<FileSystemSyncAccessHandle>>>,
    cursors: AMutex<HashMap<u32, AMutex<Box<dyn IOCursor>>>>,
    device_folders: AMutex<HashMap<String, AMutex<DeviceFolder>>>,
    zip_writers: AMutex<HashMap<String, AMutex<OpfsZipWriter>>>,
    picked_handles: AMutex<HashMap<String, AMutex<PickedEntry>>>,
    id_gen: Arc<AtomicU32>,
}

impl OpfsWorker {
    async fn get_opfs_root(&self) -> Result<FileSystemDirectoryHandle, JsValue> {
        let root_future = JsFuture::from(get_directory());
        root_future.await.map(|it| it.into())
    }

    async fn handle_operation(&self, operation: OpfsOperation) -> OpfsOperationOutput {
        let OpfsOperation { file_path, operation } = operation;

        if let FileOperation::Init { storage_session_id } = operation {
            let opfs_root = match self.get_opfs_root().await {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Failed to get OPFS root: {:?}", e);
                    return OpfsOperationOutput::Error(e);
                }
            };

            let session_dir_name = format!("session-{}", storage_session_id);
            let session_root = match opfs_root.get_or_create_directory(&session_dir_name).await {
                Ok(dir) => dir,
                Err(e) => {
                    log::error!("Failed to create session directory: {:?}", e);
                    return OpfsOperationOutput::Error(e);
                }
            };

            let _ = self.storage_session_id.set(storage_session_id.clone());
            let _ = self.root.set(Arc::new(session_root));
            log::info!("OPFS session root initialized: {}", session_dir_name);
            return OpfsOperationOutput::Void;
        }

        if let FileOperation::CleanUp { paths } = operation {
            let opfs_root = match self.get_opfs_root().await {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Failed to get OPFS root for cleanup: {:?}", e);
                    return OpfsOperationOutput::Error(e);
                }
            };

            let options = FileSystemRemoveOptions::new();
            options.set_recursive(true);
            for path in paths {
                if path.starts_with(PICKED_SCHEME) {
                    continue;
                }
                let fut = opfs_root.remove_entry_with_options(&path, &options);
                let _ = JsFuture::from(fut).await;
            }
            return OpfsOperationOutput::Void;
        }

        if let FileOperation::RegisterPickedHandle { handle } = operation {
            let cast: Result<FileSystemFileHandle, _> = handle.dyn_into();
            let Ok(fs_handle) = cast else {
                return OpfsOperationOutput::Error(JsValue::from("RegisterPickedHandle requires a FileSystemFileHandle"));
            };
            let mut map = self.picked_handles.lock().await;
            if let Some(existing) = map.remove(&file_path) {
                if let Ok(inner) = Arc::try_unwrap(existing) {
                    let entry = inner.into_inner();
                    let _ = entry.writer.abort("replaced by new registration").await;
                }
            }
            map.insert(
                file_path.clone(),
                Arc::new(Mutex::new(PickedEntry {
                    writer: PickedWriter::new(fs_handle),
                })),
            );
            return OpfsOperationOutput::Void;
        }

        if let FileOperation::FinalizePicked { commit } = operation {
            let entry = self.picked_handles.lock().await.remove(&file_path);
            let Some(entry) = entry else {
                return OpfsOperationOutput::Void;
            };
            let entry = match Arc::try_unwrap(entry) {
                Ok(inner) => inner.into_inner(),
                Err(_) => {
                    return OpfsOperationOutput::Error(JsValue::from("Failed to take picked entry ownership"));
                }
            };
            let result = if commit {
                entry.writer.close().await
            } else {
                entry.writer.abort("cancelled").await
            };
            return match result {
                Ok(_) => OpfsOperationOutput::Void,
                Err(e) => OpfsOperationOutput::Error(JsValue::from(e.to_string())),
            };
        }

        if file_path.starts_with(PICKED_SCHEME) {
            match &operation {
                FileOperation::Open => return OpfsOperationOutput::Void,
                FileOperation::Flush => return OpfsOperationOutput::Void,
                FileOperation::Write { data, position, decompress } => {
                    let entry = self.picked_handles.lock().await.get(&file_path).cloned();
                    let Some(entry) = entry else {
                        return OpfsOperationOutput::Error(JsValue::from("No picked handle registered"));
                    };
                    let bytes = if *decompress {
                        match decompress_size_prepended(data.to_vec().as_slice()) {
                            Ok(out) => out,
                            Err(e) => {
                                return OpfsOperationOutput::Error(JsValue::from(format!("Failed to decompress: {}", e)));
                            }
                        }
                    } else {
                        data.to_vec()
                    };
                    let mut guard = entry.lock().await;
                    return match guard.writer.write_at(&bytes, *position as u64).await {
                        Ok(written) => OpfsOperationOutput::Written(written),
                        Err(e) => OpfsOperationOutput::Error(JsValue::from(e.to_string())),
                    };
                }
                _ => {
                    return OpfsOperationOutput::Error(JsValue::from("Unsupported operation for picked path"));
                }
            }
        }

        let Some(root) = self.root.get().cloned() else {
            log::error!("OPFS root not initialized, call Init first");
            return OpfsOperationOutput::Error(JsValue::from("OPFS root not initialized"));
        };

        let id_gen = self.id_gen.clone();
        match operation {
            FileOperation::Open => {
                match async {
                    if self.file_handles.lock().await.contains_key(&file_path) {
                        return Ok::<(), JsValue>(());
                    }

                    if self.device_files.lock().await.contains_key(&file_path) {
                        return Ok::<(), JsValue>(());
                    }

                    if file_path.contains("zip_entry://") {
                        if let Some(path_after_prefix) = file_path.split("zip_entry://").nth(1) {
                            if let Some((zip_filename, entry_name)) = path_after_prefix.split_once('/') {
                                let zip_writer = {
                                    let writers = self.zip_writers.lock().await;
                                    writers.get(zip_filename).cloned()
                                };

                                if let Some(writer) = zip_writer {
                                    let mut guard = writer.lock().await;
                                    guard
                                        .new_entry(entry_name)
                                        .await
                                        .map_err(|e| JsValue::from(format!("Failed to create zip entry: {}", e)))?;
                                    return Ok::<(), JsValue>(());
                                } else {
                                    return Err(JsValue::from(format!("Zip writer not found for: {}", zip_filename)));
                                }
                            }
                        }
                        return Err(JsValue::from("Invalid zip_entry path format"));
                    }

                    let file_handle = root.open_file(&file_path).await?;
                    self.file_handles.lock().await.insert(file_path.clone(), Arc::new(Mutex::new(file_handle)));
                    Ok::<(), JsValue>(())
                }
                .await
                {
                    Ok(_) => OpfsOperationOutput::Void,
                    Err(e) => OpfsOperationOutput::Error(e),
                }
            }
            FileOperation::Init { .. }
            | FileOperation::CleanUp { .. }
            | FileOperation::RegisterPickedHandle { .. }
            | FileOperation::FinalizePicked { .. } => {
                unreachable!()
            }
            FileOperation::Cursor { buffer_size } => {
                // Check if this is a zip_entry:// path and create entry if needed
                if file_path.contains("zip_entry://") {
                    if let Some(path_after_prefix) = file_path.split("zip_entry://").nth(1) {
                        if let Some((zip_filename, entry_name)) = path_after_prefix.split_once('/') {
                            // Get or wait for zip writer
                            let zip_writer = {
                                let writers = self.zip_writers.lock().await;
                                writers.get(zip_filename).cloned()
                            };

                            if let Some(writer) = zip_writer {
                                let mut guard = writer.lock().await;
                                if let Err(e) = guard.new_entry(entry_name).await {
                                    return OpfsOperationOutput::Error(JsValue::from(format!("Failed to create zip entry: {}", e)));
                                }
                            } else {
                                return OpfsOperationOutput::Error(JsValue::from(format!(
                                    "Zip writer not found for: {}",
                                    zip_filename
                                )));
                            }
                        }
                    }
                }

                let cursor = if let Some(device_file) = self.device_files.lock().await.get(&file_path) {
                    let guard = device_file.lock().await;
                    match IOReaderBlobImpl::from_file(&guard.file, buffer_size).await {
                        Ok(reader) => Box::new(reader) as Box<dyn IOCursor>,
                        Err(e) => return OpfsOperationOutput::Error(JsValue::from(e.to_string())),
                    }
                } else if let Some(device_folder) = self.device_folders.lock().await.get(&file_path) {
                    match device_folder.lock().await.cursor(buffer_size).await {
                        Ok(cursor) => cursor,
                        Err(e) => return OpfsOperationOutput::Error(JsValue::from(e.to_string())),
                    }
                } else {
                    match root.cursor(&file_path, buffer_size).await {
                        Ok(cursor) => cursor,
                        Err(e) => return OpfsOperationOutput::Error(e),
                    }
                };

                let id = id_gen.fetch_add(1, Ordering::Relaxed);
                self.cursors.lock().await.insert(id, Arc::new(Mutex::new(cursor)));
                OpfsOperationOutput::Cursor(id)
            }
            FileOperation::CursorNext {
                instance_id,
                max,
                compressed,
            } => {
                let disk_tick = Instant::now();
                let Some(cursor) = self.cursors.lock().await.get(&instance_id).cloned() else {
                    return OpfsOperationOutput::Error("Cursor not found".into());
                };

                let mut guard = cursor.lock().await;
                let Ok(Some(data)) = guard.next(max).await else {
                    return OpfsOperationOutput::Binary {
                        data: Uint8Array::new_with_length(0),
                        raw_size: 0,
                        is_compressed_failed: false,
                        read_time_in_micros: 0,
                        compression_time_in_micros: 0,
                    };
                };

                let disk_elapsed = disk_tick.elapsed();
                if data.is_empty() {
                    return OpfsOperationOutput::Binary {
                        data: Uint8Array::new_with_length(0),
                        raw_size: 0,
                        is_compressed_failed: false,
                        read_time_in_micros: 0,
                        compression_time_in_micros: 0,
                    };
                }

                let raw_size = data.len();
                let (data, elapsed, failed) = match compressed {
                    true => {
                        let instant = Instant::now();
                        let buf = compress_prepend_size(data);
                        let is_failed = buf.len() > raw_size;
                        let out = match is_failed {
                            true => Uint8Array::new_from_slice(data),
                            false => Uint8Array::new_from_slice(&buf),
                        };

                        let elapsed = instant.elapsed();
                        (out, elapsed.as_micros() as u64, is_failed)
                    }
                    false => (Uint8Array::new_from_slice(data), 0, false),
                };

                OpfsOperationOutput::Binary {
                    data,
                    raw_size,
                    compression_time_in_micros: elapsed,
                    read_time_in_micros: disk_elapsed.as_micros() as u64,
                    is_compressed_failed: failed,
                }
            }
            FileOperation::CursorEnd(instance_id) => {
                if let Some(c) = self.cursors.lock().await.remove(&instance_id) {
                    let _ = c.lock().await.end().await;
                }

                OpfsOperationOutput::Void
            }
            FileOperation::WriteNew { data } => {
                if let Ok(file_handle) = root.open_file(&file_path).await {
                    let _ = file_handle.write_with_js_u8_array(&data);
                    let options = FileSystemReadWriteOptions::new();
                    options.set_at(0f64);
                    let _ = file_handle.write_with_js_u8_array_and_options(&data, &options);
                }

                OpfsOperationOutput::Void
            }
            FileOperation::AddFile(file) => {
                match async {
                    let mut device_files = self.device_files.lock().await;
                    let resource = file.raw_local_resource().clone();
                    device_files.insert(file_path, Arc::new(Mutex::new(file)));
                    Ok::<Uint8Array, JsValue>(resource)
                }
                .await
                {
                    Ok(r) => OpfsOperationOutput::LocalResourceInstance(r),
                    Err(e) => OpfsOperationOutput::Error(e),
                }
            }
            FileOperation::GetFile => {
                if let Some(device_file) = self.device_files.lock().await.get(&file_path) {
                    return OpfsOperationOutput::File(device_file.lock().await.file.file.clone());
                }

                OpfsOperationOutput::Error("No file selected".into())
            }
            FileOperation::Write {
                data,
                position,
                decompress,
            } => {
                if file_path.contains("zip_entry://") {
                    if let Some(path_after_prefix) = file_path.split("zip_entry://").nth(1) {
                        if let Some((zip_filename, _entry_name)) = path_after_prefix.split_once('/') {
                            let zip_writer = {
                                let writers = self.zip_writers.lock().await;
                                writers.get(zip_filename).cloned()
                            };

                            if let Some(writer) = zip_writer {
                                let mut guard = writer.lock().await;

                                let data_vec = match decompress {
                                    true => match decompress_size_prepended(data.to_vec().as_slice()) {
                                        Ok(out) => out,
                                        Err(e) => {
                                            return OpfsOperationOutput::Error(JsValue::from(format!("Failed to decompress: {}", e)))
                                        }
                                    },
                                    false => data.to_vec(),
                                };

                                return match guard.write(&data_vec).await {
                                    Ok(_) => OpfsOperationOutput::Written(data_vec.len()),
                                    Err(e) => OpfsOperationOutput::Error(JsValue::from(format!("Failed to write to zip: {}", e))),
                                };
                            } else {
                                return OpfsOperationOutput::Error(JsValue::from(format!(
                                    "Zip writer not found for: {}",
                                    zip_filename
                                )));
                            }
                        }
                    }
                }

                let Some(file_handle) = self.file_handles.lock().await.get(&file_path).cloned() else {
                    return OpfsOperationOutput::Error("No file handle open".into());
                };

                let file_guard = file_handle.lock().await;
                let options = FileSystemReadWriteOptions::new();
                options.set_at(position as f64);

                let data = match decompress {
                    true => {
                        let out = decompress_size_prepended(data.to_vec().as_slice()).unwrap();
                        Uint8Array::new_from_slice(&out)
                    }
                    false => data,
                };

                match file_guard.write_with_js_u8_array_and_options(&data, &options) {
                    Ok(written) => OpfsOperationOutput::Written(written as usize),
                    Err(e) => OpfsOperationOutput::Error(e),
                }
            }
            FileOperation::FileEntry => {
                if let Some(file_handle) = self.file_handles.lock().await.get(&file_path).cloned() {
                    let file_guard = file_handle.lock().await;
                    let entry = FileEntry {
                        path: file_path.into(),
                        size: file_guard.get_size().unwrap_or_default() as u64,
                        modified_at: Utc::now().into(),
                        is_dir: false,
                    };

                    return match file_guard.get_size() {
                        Ok(_size) => OpfsOperationOutput::FileEntry(entry),
                        Err(e) => OpfsOperationOutput::Error(e),
                    };
                }

                if let Some(device_folder) = self.device_folders.lock().await.get(&file_path).cloned() {
                    let guard = device_folder.lock().await;
                    let entry = FileEntry {
                        path: guard.base_path.clone().into(),
                        size: guard.resource_instance.size,
                        is_dir: false,
                        modified_at: Utc::now().into(),
                    };

                    return OpfsOperationOutput::FileEntry(entry);
                }

                if let Some(device_file) = self.device_files.lock().await.get(&file_path).cloned() {
                    let entry = FileEntry {
                        path: file_path.into(),
                        size: device_file.lock().await.file.size() as u64,
                        modified_at: Utc::now().into(),
                        is_dir: false,
                    };

                    return OpfsOperationOutput::FileEntry(entry);
                }

                OpfsOperationOutput::Error("No file handle open".into())
            }
            FileOperation::LocalResourceInstance => {
                if let Some(device_file) = self.device_files.lock().await.get(&file_path).cloned() {
                    let file_guard = device_file.lock().await;
                    return OpfsOperationOutput::LocalResourceInstance(file_guard.raw_local_resource().clone());
                }

                if let Some(device_folder) = self.device_folders.lock().await.get(&file_path).cloned() {
                    let guard = device_folder.lock().await;
                    return OpfsOperationOutput::LocalResourceInstance(serialize(&guard.resource_instance));
                }

                OpfsOperationOutput::Error("No file selected".into())
            }
            FileOperation::Flush => {
                let Some(file_handle) = self.file_handles.lock().await.get(&file_path).cloned() else {
                    return OpfsOperationOutput::Void;
                };

                let file_guard = file_handle.lock().await;
                let _ = file_guard.flush();
                OpfsOperationOutput::Void
            }
            FileOperation::GenerateSource => {
                match async {
                    let file_handle = root.open_file_async(&file_path).await?;
                    let file = JsFuture::from(file_handle.get_file()).await?;
                    let file: web_sys::File = file.into();
                    let url = web_sys::Url::create_object_url_with_blob(&file)?;
                    Ok::<String, JsValue>(url)
                }
                .await
                {
                    Ok(url) => OpfsOperationOutput::DownloadUrl(url),
                    Err(e) => OpfsOperationOutput::Error(e),
                }
            }
            FileOperation::Blob => {
                match async {
                    if let Some(device_file) = self.device_files.lock().await.get(&file_path).cloned() {
                        let file_guard = device_file.lock().await;
                        let blob = file_guard.file.slice_with_f64_and_f64(0.0, file_guard.file.size())?;
                        return Ok::<Blob, JsValue>(blob);
                    }

                    let file_handle = root.open_file_async(&file_path).await?;
                    let file = JsFuture::from(file_handle.get_file()).await?;
                    let blob: Blob = file.into();
                    Ok::<Blob, JsValue>(blob)
                }
                .await
                {
                    Ok(blob) => OpfsOperationOutput::Blob(blob),
                    Err(e) => OpfsOperationOutput::Error(e),
                }
            }
            FileOperation::AddFolder { files, path } => {
                let mut folders = self.device_folders.lock().await;
                let key = file_path.clone();
                let resource_path = LocalResourcePath::PlatformIdentifier(format!("opfs://{}", file_path.clone()));
                let folder = DeviceFolder::new(resource_path, path.into(), files).await;
                let response = OpfsOperationOutput::LocalResourceInstance(serialize(&folder.resource_instance));
                folders.insert(key, Arc::new(Mutex::new(folder)));

                response
            }
            FileOperation::CreateZipWriter { zip_filename } => {
                match async {
                    let existing_writer = {
                        let mut writers = self.zip_writers.lock().await;
                        writers.remove(&zip_filename)
                    };

                    if let Some(writer) = existing_writer {
                        log::warn!("Cleaning up existing zip writer for: {}", zip_filename);
                        let writer = Arc::try_unwrap(writer).map_err(|_| JsValue::from("Failed to unwrap zip writer"))?.into_inner();
                        if let Err(e) = writer.finalize().await {
                            log::error!("Failed to finalize existing zip writer: {}", e);
                        }
                    }

                    let remove_options = FileSystemRemoveOptions::new();
                    let _ = JsFuture::from(root.remove_entry_with_options(&zip_filename, &remove_options)).await;

                    let sync_handle = root.open_file(&zip_filename).await?;
                    let zip_writer = OpfsZipWriter::new(sync_handle);

                    self.zip_writers.lock().await.insert(zip_filename.clone(), Arc::new(Mutex::new(zip_writer)));
                    log::info!("Created new zip writer for: {}", zip_filename);

                    Ok::<(), JsValue>(())
                }
                .await
                {
                    Ok(_) => OpfsOperationOutput::Void,
                    Err(e) => OpfsOperationOutput::Error(e),
                }
            }
            FileOperation::FinalizeZip { zip_filename } => {
                match async {
                    let zip_writer = {
                        let mut writers = self.zip_writers.lock().await;
                        writers.remove(&zip_filename)
                    };

                    if let Some(writer) = zip_writer {
                        let writer = Arc::try_unwrap(writer).map_err(|_| JsValue::from("Failed to unwrap zip writer"))?.into_inner();
                        writer.finalize().await.map_err(|e| JsValue::from(e.to_string()))?;
                    } else {
                        return Err(JsValue::from(format!("Zip writer not found: {}", zip_filename)));
                    }

                    Ok::<(), JsValue>(())
                }
                .await
                {
                    Ok(_) => OpfsOperationOutput::Void,
                    Err(e) => OpfsOperationOutput::Error(e),
                }
            }
        }
    }
}

impl Worker for OpfsWorker {
    type Input = WorkerMessage<OpfsOperation>;
    type Message = ();
    type Output = WorkerMessage<OpfsOperationOutput>;

    fn create(_: &WorkerScope<Self>) -> Self {
        setup();
        init_scoped_id_generator("Bitbridge".to_owned());

        Self {
            root: Arc::new(OnceCell::new()),
            storage_session_id: Arc::new(OnceCell::new()),
            file_handles: Default::default(),
            device_files: Default::default(),
            cursors: Default::default(),
            id_gen: Arc::new(AtomicU32::new(0)),
            device_folders: Default::default(),
            zip_writers: Default::default(),
        }
    }

    fn update(&mut self, _: &WorkerScope<Self>, _: Self::Message) {}

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
        let scope = scope.clone();
        let worker = self.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let msg_id = msg.id().to_owned();
            let result = worker.handle_operation(msg.message).await;
            scope.respond(id, WorkerMessage::response(msg_id, result));
        });
    }
}
