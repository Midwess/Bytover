use crate::file_api::device_file::WasmFile;
use anyhow::Result;
use async_trait::async_trait;
use bytes::BytesMut;
use core_services::local_storage::entry::FileEntry;
use futures::lock::Mutex;
use futures_channel::oneshot;
use js_sys::{ArrayBuffer, Uint8Array};
use shared::core_api::IOReader;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Blob, FileReader, ProgressEvent};
pub struct IOReaderImpl {
    pub file: Mutex<WasmFile>,
    pub position: u64,
    pub chunk_size: u64,
    pub buffer: BytesMut
}

impl IOReaderImpl {
    pub(crate) fn new(file: WasmFile, chunk_size: u64) -> Self {
        let mut buffer = BytesMut::with_capacity(chunk_size as usize);
        buffer.resize(chunk_size as usize, 0);
        Self {
            file: Mutex::new(file),
            position: 0,
            chunk_size,
            buffer
        }
    }
}

#[async_trait(?Send)]
impl IOReader for IOReaderImpl {
    async fn next(&mut self, max: Option<u64>) -> Result<Option<&[u8]>> {
        let total_size = self.entry().await?.size;

        let file = self.file.lock().await;
        let end = (self.position + self.chunk_size).min(total_size);

        let blob: Blob = file
            .slice_with_f64_and_f64(self.position as f64, end as f64)
            .map_err(|e| anyhow::anyhow!("Failed to slice file: {e:?}"))?;

        let reader = FileReader::new().map_err(|_| anyhow::anyhow!("Failed to create FileReader"))?;

        let (tx, rx) = oneshot::channel::<()>();
        let tx = Rc::new(RefCell::new(Some(tx)));

        let onloadend = Closure::once(Box::new(move |_event: ProgressEvent| {
            if let Some(tx) = tx.borrow_mut().take() {
                let _ = tx.send(());
            }
        }) as Box<dyn FnMut(_)>);

        reader.set_onloadend(Some(onloadend.as_ref().unchecked_ref()));
        reader
            .read_as_array_buffer(&blob)
            .map_err(|_| anyhow::anyhow!("Failed to read blob as array buffer"))?;
        onloadend.forget();

        rx.await.map_err(|_| anyhow::anyhow!("Failed to await file read"))?;

        let array_buffer = reader
            .result()
            .map_err(|it| anyhow::anyhow!("FileReader result was null {it:?}"))?
            .dyn_into::<ArrayBuffer>()
            .map_err(|e| anyhow::anyhow!("Failed to cast result to ArrayBuffer: {e:?}"))?;

        let data = Uint8Array::new(&array_buffer);
        data.copy_to(&mut self.buffer);

        self.position = end;

        let response = &self.buffer[..data.length() as usize];
        Ok(Some(response))
    }

    async fn entry(&self) -> Result<FileEntry> {
        let file = self.file.lock().await;
        Ok(FileEntry {
            is_dir: false,
            modified_at: chrono::Utc::now().into(),
            size: file.size() as u64,
            path: "".into()
        })
    }
}
