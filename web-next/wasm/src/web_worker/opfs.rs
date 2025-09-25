use std::cell::OnceCell;
use crate::get_directory;
use crate::web_worker::bridge::{TrustedWorkerMessage, WorkerMessage};
use chrono::Utc;
use core_services::local_storage::entry::FileEntry;
use core_services::logger::setup;
use futures::lock::Mutex;
use gloo_worker::{HandlerId, Worker, WorkerScope};
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetDirectoryOptions, FileSystemGetFileOptions, FileSystemReadWriteOptions, FileSystemSyncAccessHandle};
use shared::entities::file_system::file::LocalResource;
use crate::file_api::device_file::DeviceFile;

/// Web worker that support file system on browser
/// Open a file handle, keep tracks that handle in a list
/// We want to support read, write, download concurrently on multiple files.
/// That's why there are no Operation::Closed or similar operations,
/// we never want to close the handle once it opens to prevent race condition.

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpfsOperation {
    pub file_path: String,
    pub operation: FileOperation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileOperation {
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
        position: usize
    },
    Read {
        position: usize,
        amount: usize
    },
    Flush,
    FileEntry,
    LocalResourceInstance,
    GenerateSource,
    Blob
}

unsafe impl Send for OpfsOperation {}
unsafe impl Sync for OpfsOperation {}

#[derive(Debug, Serialize, Deserialize)]
pub enum OpfsOperationOutput {
    Void,
    Error(#[serde(with = "serde_wasm_bindgen::preserve")] JsValue),
    Binary(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    Written(usize),
    File(#[serde(with = "serde_wasm_bindgen::preserve")] File),
    FileEntry(FileEntry),
    LocalResourceInstance(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    DownloadUrl(String),
    Blob(#[serde(with = "serde_wasm_bindgen::preserve")] Blob)
}

unsafe impl Send for OpfsOperationOutput {}
unsafe impl Sync for OpfsOperationOutput {}

trait FileSystemDirectoryHandleExt {
    async fn open_file(&self, path: &str) -> Result<FileSystemSyncAccessHandle, JsValue>;
    async fn open_file_async(&self, path: &str) -> Result<FileSystemFileHandle, JsValue>;
}

impl FileSystemDirectoryHandleExt for FileSystemDirectoryHandle {
    async fn open_file(&self, path: &str) -> Result<FileSystemSyncAccessHandle, JsValue> {
        let path_parts: Vec<&str> = path.split('/').collect();
        let file_name = path_parts.last().ok_or("Empty path")?;
        let dir_parts = &path_parts[..path_parts.len() - 1];

        let mut current_dir = self.clone();

        let options = FileSystemGetDirectoryOptions::new();
        options.set_create(true);
        for dir_name in dir_parts {
            if !dir_name.is_empty() {
                let dir_future = JsFuture::from(current_dir.get_directory_handle_with_options(dir_name, &options));
                current_dir = dir_future.await?.into();
            }
        }

        let options = FileSystemGetFileOptions::new();
        options.set_create(true);
        let file_future = JsFuture::from(current_dir.get_file_handle_with_options(file_name, &options));
        let file_handle: FileSystemFileHandle = file_future.await?.into();
        let file_sync_handle: FileSystemSyncAccessHandle = JsFuture::from(file_handle.create_sync_access_handle()).await?.into();

        Ok(file_sync_handle)
    }

    async fn open_file_async(&self, path: &str) -> Result<FileSystemFileHandle, JsValue> {
        let path_parts: Vec<&str> = path.split('/').collect();
        let file_name = path_parts.last().ok_or("Empty path")?;
        let dir_parts = &path_parts[..path_parts.len() - 1];

        let mut current_dir = self.clone();

        let options = FileSystemGetDirectoryOptions::new();
        options.set_create(true);
        for dir_name in dir_parts {
            if !dir_name.is_empty() {
                let dir_future = JsFuture::from(current_dir.get_directory_handle_with_options(dir_name, &options));
                current_dir = dir_future.await?.into();
            }
        }

        let options = FileSystemGetFileOptions::new();
        options.set_create(true);
        let file_future = JsFuture::from(current_dir.get_file_handle_with_options(file_name, &options));
        let file_handle: FileSystemFileHandle = file_future.await?.into();

        Ok(file_handle)
    }
}

pub type AMutex<T> = Arc<Mutex<T>>;

#[derive(Clone)]
pub struct OpfsWorker {
    root: OnceCell<Arc<FileSystemDirectoryHandle>>,
    device_files: AMutex<HashMap<String, AMutex<DeviceFile>>>,
    file_handles: AMutex<HashMap<String, AMutex<FileSystemSyncAccessHandle>>>
}

impl OpfsWorker {
    async fn handle_operation(&self, operation: OpfsOperation) -> OpfsOperationOutput {
        let OpfsOperation { file_path, operation } = operation;
        let root = match self.root.get() {
            Some(r) => r.clone(),
            None => {
                let root_future = JsFuture::from(get_directory());
                let root: FileSystemDirectoryHandle = root_future.await.unwrap().into();
                let _ = self.root.set(Arc::new(root));
                self.root.get().unwrap().clone()
            }
        };

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
                .await {
                    Ok(r) => OpfsOperationOutput::LocalResourceInstance(r),
                    Err(e) => OpfsOperationOutput::Error(e)
                }
            }
            FileOperation::GetFile => {
                if let Some(device_file) = self.device_files.lock().await.get(&file_path) {
                   return OpfsOperationOutput::File(device_file.lock().await.file.0.clone());
                }

                OpfsOperationOutput::Error("No file selected".into())
            }
            FileOperation::Write { data, position } => {
                let Some(file_handle) = self.file_handles.lock().await.get(&file_path).cloned() else {
                    return OpfsOperationOutput::Error("No file handle open".into());
                };

                let file_guard = file_handle.lock().await;
                let options = FileSystemReadWriteOptions::new();
                options.set_at(position as f64);
                match file_guard.write_with_js_u8_array_and_options(&data, &options) {
                    Ok(written) => OpfsOperationOutput::Written(written as usize),
                    Err(e) => OpfsOperationOutput::Error(e)
                }
            }
            FileOperation::Read { position, amount } => {
                if let Some(file_handle) = self.file_handles.lock().await.get(&file_path).cloned() {
                    let file_guard = file_handle.lock().await;
                    let options = FileSystemReadWriteOptions::new();
                    options.set_at(position as f64);
                    let buffer = Uint8Array::new_with_length(amount as u32);
                    return match file_guard.read_with_js_u8_array_and_options(&buffer, &options) {
                        Ok(bytes_read) => OpfsOperationOutput::Binary(buffer.subarray(0, bytes_read as u32)),
                        Err(e) => OpfsOperationOutput::Error(e)
                    }
                }
                if let Some(device_file) = self.device_files.lock().await.get(&file_path).cloned() {
                    let file_guard = device_file.lock().await;
                    let end_position = (position + amount).min(file_guard.file.size() as usize);
                    return match file_guard.file.slice_with_f64_and_f64(position as f64, end_position as f64) {
                        Ok(blob) => match JsFuture::from(blob.array_buffer()).await {
                            Ok(buffer) => OpfsOperationOutput::Binary(Uint8Array::new_with_byte_offset_and_length(&buffer, 0, (end_position - position) as u32)),
                            Err(e) => return OpfsOperationOutput::Error(e)
                        },
                        Err(e) => return OpfsOperationOutput::Error(e)
                    };
                }

                OpfsOperationOutput::Error("No file handle open".into())
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
                    let root_future = JsFuture::from(get_directory());
                    let root: FileSystemDirectoryHandle = root_future.await?.into();
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
                        let blob = file_guard.file.slice_with_f64_and_f64(0.0, file_guard.file.size() as f64).unwrap();
                        return Ok::<Blob, JsValue>(blob);
                    }

                    let root_future = JsFuture::from(get_directory());
                    let root: FileSystemDirectoryHandle = root_future.await?.into();
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
        }
    }
}

impl Worker for OpfsWorker {
    type Input = WorkerMessage<OpfsOperation>;
    type Message = ();
    type Output = WorkerMessage<OpfsOperationOutput>;

    fn create(_: &WorkerScope<Self>) -> Self {
        setup();

        Self {
            root: Default::default(),
            file_handles: Default::default(),
            device_files: Default::default(),
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
