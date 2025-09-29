use std::pin::Pin;
use async_stream::stream;
use futures::Stream;
use js_sys::Array;
use n0_future::StreamExt;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{File, FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetDirectoryOptions, FileSystemGetFileOptions, FileSystemSyncAccessHandle};
use core_services::local_storage::stream::IOCursor;
use crate::file_api::device_file::WasmFile;

pub type FileStream = Pin<Box<dyn Stream<Item = Result<WasmFile, anyhow::Error>>>>;

#[async_trait::async_trait(?Send)]
pub trait FileSystemDirectoryHandleExt {
    async fn open_file(&self, path: &str) -> Result<FileSystemSyncAccessHandle, JsValue>;
    async fn open_file_async(&self, path: &str) -> Result<FileSystemFileHandle, JsValue>;
    // Return either FileSystemAccessHandle
    // or FileSystemDirectoryHandle if it is a folder
    async fn access(&self, path: &str) -> Result<JsValue, JsValue>;
    fn file_stream(&self) -> FileStream;
    async fn cursor(&self, path: &str) -> Result<Box<dyn IOCursor>, JsValue>;
    async fn size(&self) -> Result<u64, JsValue>;
}

#[async_trait::async_trait(?Send)]
impl FileSystemDirectoryHandleExt for FileSystemDirectoryHandle {
    async fn open_file(&self, path: &str) -> Result<FileSystemSyncAccessHandle, JsValue> {
        let file_async_handle: FileSystemFileHandle = self.access(path).await?.dyn_into()?;
        let file_sync_handle: FileSystemSyncAccessHandle = JsFuture::from(file_async_handle.create_sync_access_handle()).await?.into();

        Ok(file_sync_handle)
    }

    async fn open_file_async(&self, path: &str) -> Result<FileSystemFileHandle, JsValue> {
        let file_async_handle: FileSystemFileHandle = self.access(path).await?.dyn_into()?;
        Ok(file_async_handle)
    }

    async fn access(&self, path: &str) -> Result<JsValue, JsValue> {
        let path_parts: Vec<&str> = path.split('/').collect();
        let entry_name = path_parts.last().ok_or("Empty path")?;
        let dir_parts = &path_parts[..path_parts.len() - 1];

        let mut current_dir = self.clone();

        let dir_options = FileSystemGetDirectoryOptions::new();
        dir_options.set_create(true);
        for dir_name in dir_parts {
            if !dir_name.is_empty() {
                let dir_future = JsFuture::from(current_dir.get_directory_handle_with_options(dir_name, &dir_options));
                current_dir = dir_future.await?.into();
            }
        }

        let dir_options = FileSystemGetDirectoryOptions::new();
        dir_options.set_create(true);
        let dir_handle_result = JsFuture::from(current_dir.get_directory_handle_with_options(entry_name, &dir_options)).await;

        if let Ok(dir_handle_js) = dir_handle_result {
            let dir_handle: FileSystemDirectoryHandle = dir_handle_js.into();
            Ok(dir_handle.into())
        } else {
            let file_options = FileSystemGetFileOptions::new();
            file_options.set_create(true);
            let file_handle_js = JsFuture::from(current_dir.get_file_handle_with_options(entry_name, &file_options)).await?;
            let file_handle: FileSystemFileHandle = file_handle_js.into();
            Ok(file_handle.into())
        }
    }

    fn file_stream(&self) -> FileStream {
        let dir_handle = self.clone();

        let stream = stream! {
            let mut dir_stack = vec![dir_handle];

            while let Some(current_dir) = dir_stack.pop() {
                let entries = current_dir.entries();

                loop {
                    let entry_result = JsFuture::from(
                        entries.next().map_err(|e| anyhow::anyhow!("{e:?}"))?
                    ).await;

                    let entry_js = match entry_result {
                        Ok(js_val) => js_val,
                        Err(e) => {
                            yield Err(anyhow::anyhow!("{e:?}"));
                            break;
                        }
                    };

                    if entry_js.is_undefined() {
                        break;
                    }

                    let entry_array: Array = entry_js.dyn_into()
                        .map_err(|e| anyhow::anyhow!("Failed to convert entry: {e:?}"))?;
                    let handle = entry_array.get(1);

                    let kind = js_sys::Reflect::get(&handle, &JsValue::from_str("kind"))
                        .map_err(|e| anyhow::anyhow!("Failed to get kind: {e:?}"))?
                        .as_string()
                        .unwrap_or_default();

                    match kind.as_str() {
                        "file" => {
                            let file_handle = handle.unchecked_into::<FileSystemFileHandle>();
                            let js_value = JsFuture::from(file_handle.get_file()).await.unwrap();
                            let file: File = js_value.dyn_into().unwrap();
                            yield Ok(WasmFile(file));
                        }
                        "directory" => {
                            let dir_handle = handle.unchecked_into::<FileSystemDirectoryHandle>();
                            dir_stack.push(dir_handle);
                        }
                        _ => {}
                    }
                }
            }
        };

        Box::pin(stream)
    }

    async fn cursor(&self, path: &str) -> Result<Box<dyn IOCursor>, JsValue> {
        let handle = self.access(path).await?;
        todo!()
    }

    async fn size(&self) -> Result<u64, JsValue> {
        let mut stream = self.file_stream();

        let mut total_size = 0u64;
        while let Some(file_result) = stream.next().await {
            let file = file_result.map_err(|it| JsValue::from(it.to_string()))?;
            total_size += file.size() as u64;
        }

        Ok(total_size)
    }
}
