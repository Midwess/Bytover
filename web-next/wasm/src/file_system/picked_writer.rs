use anyhow::{anyhow, Result};
use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{FileSystemFileHandle, FileSystemWritableFileStream};

pub struct PickedWriter {
    handle: FileSystemFileHandle,
    stream: Option<FileSystemWritableFileStream>,
}

impl PickedWriter {
    pub fn new(handle: FileSystemFileHandle) -> Self {
        Self { handle, stream: None }
    }

    async fn ensure_stream(&mut self) -> Result<&FileSystemWritableFileStream> {
        if self.stream.is_none() {
            let promise = self.handle.create_writable();
            let value = JsFuture::from(promise)
                .await
                .map_err(|e| anyhow!("Failed to create writable stream: {:?}", e))?;
            let stream: FileSystemWritableFileStream = value
                .dyn_into()
                .map_err(|_| anyhow!("createWritable did not return FileSystemWritableFileStream"))?;
            self.stream = Some(stream);
        }
        Ok(self.stream.as_ref().unwrap())
    }

    pub async fn write_at(&mut self, data: &[u8], position: u64) -> Result<usize> {
        let stream = self.ensure_stream().await?;

        let seek_promise = stream
            .seek_with_f64(position as f64)
            .map_err(|e| anyhow!("Failed to seek: {:?}", e))?;
        JsFuture::from(seek_promise)
            .await
            .map_err(|e| anyhow!("Seek failed: {:?}", e))?;

        let array = Uint8Array::from(data);
        let write_promise = stream
            .write_with_js_u8_array(&array)
            .map_err(|e| anyhow!("Failed to start write: {:?}", e))?;
        JsFuture::from(write_promise)
            .await
            .map_err(|e| anyhow!("Write failed: {:?}", e))?;

        Ok(data.len())
    }

    pub async fn close(mut self) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            let promise = stream.close();
            JsFuture::from(promise)
                .await
                .map_err(|e| anyhow!("Failed to close writable stream: {:?}", e))?;
        }
        Ok(())
    }

    pub async fn abort(mut self, reason: &str) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            let promise = stream.abort_with_reason(&JsValue::from_str(reason));
            JsFuture::from(promise)
                .await
                .map_err(|e| anyhow!("Failed to abort writable stream: {:?}", e))?;
        }
        Ok(())
    }
}
