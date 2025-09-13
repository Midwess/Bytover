use crate::get_directory;
use crate::web_worker::bridge::{TrustedWorkerMessage, WorkerMessage};
use core_services::logger::setup;
use futures::lock::Mutex;
use gloo_worker::{HandlerId, Worker, WorkerScope};
use js_sys::Uint8Array;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    FileSystemDirectoryHandle,
    FileSystemFileHandle,
    FileSystemGetDirectoryOptions,
    FileSystemGetFileOptions,
    FileSystemReadWriteOptions,
    FileSystemSyncAccessHandle
};

trait FileSystemDirectoryHandleExt {
    async fn open_file(&self, path: &str) -> Result<FileSystemSyncAccessHandle, JsValue>;
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OpfsOperation {
    Open(String),
    Write(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array, u64),
    Read(usize, u64),
    Flush,
    Size,
    Close
}

unsafe impl Send for OpfsOperation {}
unsafe impl Sync for OpfsOperation {}

#[derive(Debug, Serialize, Deserialize)]
pub enum OpfsOperationOutput {
    Void,
    Error(#[serde(with = "serde_wasm_bindgen::preserve")] JsValue),
    Binary(#[serde(with = "serde_wasm_bindgen::preserve")] Uint8Array),
    Written(usize),
    Size(u64)
}

unsafe impl Send for OpfsOperationOutput {}
unsafe impl Sync for OpfsOperationOutput {}

#[derive(Clone)]
pub struct OpfsWorker {
    file_handle: Arc<Mutex<Option<FileSystemSyncAccessHandle>>>
}

impl Worker for OpfsWorker {
    type Input = WorkerMessage<OpfsOperation>;
    type Message = ();
    type Output = WorkerMessage<OpfsOperationOutput>;

    fn create(_: &WorkerScope<Self>) -> Self {
        setup();
        Self {
            file_handle: Default::default()
        }
    }

    fn update(&mut self, _: &WorkerScope<Self>, _: Self::Message) {}

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input, id: HandlerId) {
        let scope = scope.clone();
        let worker = self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let msg_id = msg.id().to_owned();
            let mut file_guard = worker.file_handle.lock().await;
            let result = match msg.message {
                OpfsOperation::Open(path) => {
                    log::info!("Opening file {path}");
                    match async {
                        let root_future = JsFuture::from(get_directory());
                        let root: FileSystemDirectoryHandle = root_future.await?.into();
                        let file_handle = root.open_file(&path).await?;
                        log::info!("File opened {path}");
                        *file_guard = Some(file_handle);
                        Ok::<(), JsValue>(())
                    }
                    .await
                    {
                        Ok(_) => OpfsOperationOutput::Void,
                        Err(e) => OpfsOperationOutput::Error(e)
                    }
                }
                OpfsOperation::Write(data, position) => match file_guard.as_ref() {
                    Some(file_handle) => {
                        let options = FileSystemReadWriteOptions::new();
                        options.set_at(position as f64);
                        match file_handle.write_with_u8_array_and_options(data.to_vec().as_slice(), &options) {
                            Ok(written) => OpfsOperationOutput::Written(written as usize),
                            Err(e) => OpfsOperationOutput::Error(e)
                        }
                    }
                    None => OpfsOperationOutput::Error("No file handle open".into())
                },
                OpfsOperation::Read(size, position) => match file_guard.as_ref() {
                    Some(file_handle) => {
                        let options = FileSystemReadWriteOptions::new();
                        options.set_at(position as f64);
                        let buffer = Uint8Array::new(&JsValue::from(size as f64));
                        match file_handle.read_with_js_u8_array_and_options(&buffer, &options) {
                            Ok(s) => OpfsOperationOutput::Binary(buffer.subarray(0, s as u32)),
                            Err(e) => OpfsOperationOutput::Error(e)
                        }
                    }
                    None => OpfsOperationOutput::Error("No file handle open".into())
                },
                OpfsOperation::Size => match file_guard.as_ref() {
                    Some(file_handle) => match file_handle.get_size() {
                        Ok(size) => OpfsOperationOutput::Size(size as u64),
                        Err(e) => OpfsOperationOutput::Error(e)
                    },
                    None => OpfsOperationOutput::Error("No file handle open".into())
                },
                OpfsOperation::Flush => match file_guard.as_ref() {
                    Some(file_handle) => {
                        let _ = file_handle.flush();
                        OpfsOperationOutput::Void
                    }
                    None => OpfsOperationOutput::Void
                },
                OpfsOperation::Close => match file_guard.as_ref() {
                    Some(file_handle) => {
                        let _ = file_handle.flush();
                        file_handle.close();
                        *file_guard = None;
                        log::info!("File closed");
                        OpfsOperationOutput::Void
                    }
                    None => OpfsOperationOutput::Void
                }
            };

            scope.respond(id, WorkerMessage::response(msg_id, result));
        });
    }
}
