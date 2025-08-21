use crate::file_api::storage::{FileStorage, WasmFile};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use core_services::utils::never_send::NeverSend;
use futures::lock::Mutex;
use futures_channel::oneshot;
use js_sys::{ArrayBuffer, Uint8Array};
use shared::core_api::{IOReader, IOWriter};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, FileReader, ProgressEvent};
pub struct IOReaderImpl {
    pub file: Mutex<WasmFile>,
    pub position: u64,
    pub chunk_size: u64
}

#[async_trait(?Send)]
impl IOReader for IOReaderImpl {
    async fn next(&mut self) -> Result<Option<Bytes>> {
        let total_size = self.total_size().await?;

        if self.position >= total_size {
            return Ok(None);
        }

        let file = self.file.lock().await;
        let end = (self.position + self.chunk_size).min(total_size);

        let blob: Blob = file
            .slice_with_f64_and_f64(self.position as f64, end as f64)
            .map_err(|e| anyhow::anyhow!("Failed to slice file: {e:?}"))?;

        let reader = FileReader::new().map_err(|_| anyhow::anyhow!("Failed to create FileReader"))?;

        // Setup oneshot channel to wait for `onloadend` event
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
        onloadend.forget(); // Keep alive until called

        // Wait for the read to complete
        rx.await.map_err(|_| anyhow::anyhow!("Failed to await file read"))?;

        let array_buffer = reader
            .result()
            .map_err(|it| anyhow::anyhow!("FileReader result was null"))?
            .dyn_into::<ArrayBuffer>()
            .map_err(|e| anyhow::anyhow!("Failed to cast result to ArrayBuffer: {e:?}"))?;

        let data = Uint8Array::new(&array_buffer);
        let mut vec = vec![0u8; data.length() as usize];
        data.copy_to(&mut vec);

        self.position = end;

        Ok(Some(Bytes::from(vec)))
    }

    async fn total_size(&self) -> Result<u64> {
        Ok(self.file.lock().await.size() as u64)
    }
}

pub struct IOWriterImpl {
    storage: FileStorage
}

impl IOWriterImpl {
    pub async fn new(storage: FileStorage) -> Result<Self> {
        Ok(Self { storage })
    }
}

#[async_trait(?Send)]
impl IOWriter for IOWriterImpl {
    async fn write(&mut self, data: bytes::Bytes) -> anyhow::Result<()> {
        Ok(())
    }
}
