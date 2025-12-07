use crate::file_system::device_file::{DeviceFile, DeviceFolder, WebFile};
use crate::file_system::io::IOReaderBlobImpl;
use crate::file_system::opfs::FileSystemDirectoryHandleExt;
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
use serde::{Deserialize, Serialize};
use shared::entities::local_resource::LocalResourcePath;
use std::cell::OnceCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use n0_future::time::Instant;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, File, FileSystemDirectoryHandle, FileSystemReadWriteOptions, FileSystemRemoveOptions, FileSystemSyncAccessHandle};

/// Web worker that support file system on browser
/// There are two reasons that we use web worker for file system:
/// + Performance, off load heavy logic out of main thread to avoid blocking.
/// + Browser requirement, browser require us to use web worker to be able to access all opfs features.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpfsOperation {
    pub file_path: String,
    pub operation: FileOperation
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileOperation {
    AddFolder {
        path: String,
        files: Vec<WebFile>
    },
    Cursor {
        buffer_size: usize
    },
    CursorNext {
        instance_id: u32,
        max: Option<u64>,
        compressed: bool
    },
    CursorEnd(u32),
    AddFile(DeviceFile),
    GetFile,
    Open,
    WriteNew {
        #[serde(with = "serde_wasm_bindgen::preserve")]
        data: Uint8Array
    },
    Write {
        #[serde(with = "serde_wasm_bindgen::preserve")]
        data: Uint8Array,
        position: usize,
        decompress: bool
    },
    Flush,
    FileEntry,
    LocalResourceInstance,
    GenerateSource,
    Blob,
    ClearAll
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
    Cursor(u32)
}

unsafe impl Send for OpfsOperationOutput {}
unsafe impl Sync for OpfsOperationOutput {}

pub type AMutex<T> = Arc<Mutex<T>>;

#[derive(Clone)]
pub struct OpfsWorker {
    root: OnceCell<Arc<FileSystemDirectoryHandle>>,
    device_files: AMutex<HashMap<String, AMutex<DeviceFile>>>,
    file_handles: AMutex<HashMap<String, AMutex<FileSystemSyncAccessHandle>>>,
    cursors: AMutex<HashMap<u32, AMutex<Box<dyn IOCursor>>>>,
    device_folders: AMutex<HashMap<String, AMutex<DeviceFolder>>>,
    id_gen: Arc<AtomicU32>
}

impl OpfsWorker {
    async fn handle_operation(&self, operation: OpfsOperation) -> OpfsOperationOutput {
        let OpfsOperation { file_path, operation } = operation;
        let root = match self.root.get() {
            Some(r) => r.clone(),
            None => {
                let root_future = JsFuture::from(get_directory());
                let root: FileSystemDirectoryHandle = match root_future.await.map(|it| it.into()) {
                    Err(e) => {
                        log::error!("Failed to get root directory: {:?}", e);
                        return OpfsOperationOutput::Error(JsValue::from(format!("Opfs failed to get root dir {e:?}")));
                    },
                    Ok(it) => it
                };

                let root = Arc::new(root);
                let _ = self.root.set(root.clone());
                root
            }
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

                    let file_handle = root.open_file(&file_path).await?;
                    self.file_handles.lock().await.insert(file_path.clone(), Arc::new(Mutex::new(file_handle)));
                    Ok::<(), JsValue>(())
                }
                .await
                {
                    Ok(_) => OpfsOperationOutput::Void,
                    Err(e) => OpfsOperationOutput::Error(e)
                }
            }
            FileOperation::ClearAll => {
                let remove_options = FileSystemRemoveOptions::new();
                remove_options.set_recursive(true);
                let entries = root.entries();
                let options = FileSystemRemoveOptions::new();
                options.set_recursive(true);
                loop {
                    let Ok(fut) = entries.next() else {
                        break;
                    };
                    let val = JsFuture::from(fut).await;
                    match val {
                        Ok(val) => {
                            let entry: js_sys::Object = val.into();
                            let Ok(done) = js_sys::Reflect::get(&entry, &JsValue::from_str("done")) else {
                                continue;
                            };

                            if done.as_bool().unwrap_or(false) {
                                break;
                            }

                            let Ok(value) = js_sys::Reflect::get(&entry, &JsValue::from_str("value")) else {
                                continue;
                            };

                            let array: js_sys::Array = value.into();
                            let name = array.get(0);
                            let fut = root.remove_entry_with_options(name.as_string().unwrap_or_default().as_str(), &options);
                            let _ = JsFuture::from(fut).await;
                        }
                        Err(e) => {
                            log::error!("Failed to get next entry: {:?}", e);
                            break;
                        }
                    }
                }
                OpfsOperationOutput::Void
            },
            FileOperation::Cursor { buffer_size } => {
                let cursor = if let Some(device_file) = self.device_files.lock().await.get(&file_path) {
                    let guard = device_file.lock().await;
                    match IOReaderBlobImpl::from_file(&guard.file, buffer_size).await {
                        Ok(reader) => Box::new(reader) as Box<dyn IOCursor>,
                        Err(e) => return OpfsOperationOutput::Error(JsValue::from(e.to_string()))
                    }
                } else if let Some(device_folder) = self.device_folders.lock().await.get(&file_path) {
                    match device_folder.lock().await.cursor(buffer_size).await {
                        Ok(cursor) => cursor,
                        Err(e) => return OpfsOperationOutput::Error(JsValue::from(e.to_string()))
                    }
                } else {
                    match root.cursor(&file_path, buffer_size).await {
                        Ok(cursor) => cursor,
                        Err(e) => return OpfsOperationOutput::Error(e)
                    }
                };

                let id = id_gen.fetch_add(1, Ordering::Relaxed);
                self.cursors.lock().await.insert(id, Arc::new(Mutex::new(cursor)));
                OpfsOperationOutput::Cursor(id)
            }
            FileOperation::CursorNext { instance_id, max, compressed } => {
                let disk_tick = Instant::now();
                let Some(cursor) = self.cursors.lock().await.get(&instance_id).cloned() else {
                    return OpfsOperationOutput::Error("Cursor not found".into());
                };

                let mut guard = cursor.lock().await;
                let Ok(Some(data)) = guard.next(max).await else {
                    return OpfsOperationOutput::Binary{
                        data: Uint8Array::new_with_length(0),
                        raw_size: 0,
                        is_compressed_failed: false,
                        read_time_in_micros: 0,
                        compression_time_in_micros: 0
                    };
                };

                let disk_elapsed = disk_tick.elapsed();
                if data.len() == 0 {
                    return OpfsOperationOutput::Binary {
                        data: Uint8Array::new_with_length(0),
                        raw_size: 0,
                        is_compressed_failed: false,
                        read_time_in_micros: 0,
                        compression_time_in_micros: 0
                    };
                }

                let raw_size = data.len();
                let (data, elapsed, failed) = match compressed {
                    true => {
                        let instant = Instant::now();
                        let buf = compress_prepend_size(&data);
                        let is_failed = buf.len() > raw_size;
                        let out = match is_failed {
                            true => Uint8Array::new_from_slice(data),
                            false => Uint8Array::new_from_slice(&buf)
                        };

                        let elapsed = instant.elapsed();
                        (out, elapsed.as_micros() as u64, is_failed)
                    },
                    false => (Uint8Array::new_from_slice(data), 0, false)
                };

                OpfsOperationOutput::Binary {
                    data,
                    raw_size,
                    compression_time_in_micros: elapsed,
                    read_time_in_micros: disk_elapsed.as_micros() as u64,
                    is_compressed_failed: failed
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
                    Err(e) => OpfsOperationOutput::Error(e)
                }
            }
            FileOperation::GetFile => {
                if let Some(device_file) = self.device_files.lock().await.get(&file_path) {
                    return OpfsOperationOutput::File(device_file.lock().await.file.file.clone());
                }

                OpfsOperationOutput::Error("No file selected".into())
            }
            FileOperation::Write { data, position, decompress } => {
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
                    },
                    false => data
                };

                match file_guard.write_with_js_u8_array_and_options(&data, &options) {
                    Ok(written) => OpfsOperationOutput::Written(written as usize),
                    Err(e) => OpfsOperationOutput::Error(e)
                }
            }
            FileOperation::FileEntry => {
                if let Some(file_handle) = self.file_handles.lock().await.get(&file_path).cloned() {
                    let file_guard = file_handle.lock().await;
                    let entry = FileEntry {
                        path: file_path.into(),
                        size: file_guard.get_size().unwrap_or_default() as u64,
                        modified_at: Utc::now().into(),
                        is_dir: false
                    };

                    return match file_guard.get_size() {
                        Ok(_size) => OpfsOperationOutput::FileEntry(entry),
                        Err(e) => OpfsOperationOutput::Error(e)
                    }
                }

                if let Some(device_folder) = self.device_folders.lock().await.get(&file_path).cloned() {
                    let guard = device_folder.lock().await;
                    let entry = FileEntry {
                        path: guard.base_path.clone().into(),
                        size: guard.resource_instance.size,
                        is_dir: false,
                        modified_at: Utc::now().into()
                    };

                    return OpfsOperationOutput::FileEntry(entry);
                }

                if let Some(device_file) = self.device_files.lock().await.get(&file_path).cloned() {
                    let entry = FileEntry {
                        path: file_path.into(),
                        size: device_file.lock().await.file.size() as u64,
                        modified_at: Utc::now().into(),
                        is_dir: false
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
                    return OpfsOperationOutput::LocalResourceInstance(serialize(&guard.resource_instance))
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
                    Err(e) => OpfsOperationOutput::Error(e)
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
                    Err(e) => OpfsOperationOutput::Error(e)
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
            root: Default::default(),
            file_handles: Default::default(),
            device_files: Default::default(),
            cursors: Default::default(),
            id_gen: Arc::new(AtomicU32::new(0)),
            device_folders: Default::default()
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
