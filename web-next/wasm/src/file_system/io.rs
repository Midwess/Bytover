use crate::file_system::path_extension::WebExtLocalResourcePath;
use crate::web_worker::bridge::{WebWorkerBridge, WorkerMessage};
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput, OpfsWorker};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use core_services::local_storage::entry::FileEntry;
use core_services::local_storage::stream::IOCursor;
use core_services::utils::never_send::NeverSend;
use devlog_sdk::distributed_id::gen_id;
use js_sys::Uint8Array;
use shared::core_api::{IOReader, IOWriter};
use shared::entities::file_system::file::LocalResourcePath;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, File, FileSystemFileHandle};
use core_services::wasm::extensions::FileExtension;
use crate::file_system::device_file::WasmFile;

pub static OPFS_WORKER: LazyLock<NeverSend<WebWorkerBridge<OpfsWorker>>> =
    LazyLock::new(|| NeverSend(WebWorkerBridge::<OpfsWorker>::spawn("opfs-worker")));

pub struct IOReaderBlobImpl {
    entry: FileEntry,
    blob: NeverSend<Blob>,
    buffer: BytesMut,
    current_pos: u64
}

impl IOReaderBlobImpl {
    pub async fn from_file(file: WasmFile, buffer_size: usize) -> Result<Self> {
        let modified_at = SystemTime::UNIX_EPOCH + Duration::from_millis(file.last_modified() as u64);

        let mut buffer = BytesMut::with_capacity(buffer_size);
        buffer.resize(buffer_size, 0);

        let entry = FileEntry {
            is_dir: false,
            modified_at,
            size: file.size() as u64,
            path: PathBuf::from(file.0.webkit_path().unwrap_or(file.0.name().to_string()))
        };

        Ok(Self {
            entry,
            blob: NeverSend(file.slice().map_err(|it| anyhow!("Failed to slice file {:?}", it))?),
            buffer,
            current_pos: 0
        })
    }

    pub async fn from_file_handle(handle: FileSystemFileHandle, buffer_size: usize) -> Result<Self> {
        let file: File = JsFuture::from(handle.get_file())
            .await
            .map_err(|it| anyhow!("failed to get file {it:?}"))?
            .dyn_into()
            .unwrap();
        Self::from_file(WasmFile(file), buffer_size).await
    }
}

#[async_trait(?Send)]
impl IOCursor for IOReaderBlobImpl {
    async fn next(&mut self, max: Option<u64>) -> Result<Option<&[u8]>> {
        let from = self.current_pos;
        let to = (from + max.unwrap_or(self.buffer.len() as u64).min(self.buffer.len() as u64)).min(self.entry.size);
        if from >= to {
            return Ok(None)
        }

        let amount = to - from;
        let blob = self
            .blob
            .slice_with_f64_and_f64(from as f64, to as f64)
            .map_err(|it| anyhow!("Failed to slice blob {it:?}"))?;
        let js_value = JsFuture::from(blob.array_buffer())
            .await
            .map_err(|it| anyhow!("failed to get array buffer {it:?}"))?;
        let data = Uint8Array::new_with_byte_offset_and_length(&js_value, 0, amount as u32);
        data.copy_to(&mut self.buffer[..data.length() as usize]);

        self.current_pos += amount;
        Ok(Some(&self.buffer[0..(amount as usize)]))
    }

    async fn entry(&self) -> Result<FileEntry> {
        Ok(self.entry.clone())
    }
}

/// This is the bridge to the cursor in opfs worker
pub struct IOReaderOpfsImpl {
    path: PathBuf,
    buffer: BytesMut,
    instance_id: u32
}

impl IOReaderOpfsImpl {
    pub async fn new(path: PathBuf, buffer_size: usize) -> Result<Self> {
        let path_str = path.to_string_lossy().to_string();

        let msg = WorkerMessage::new(OpfsOperation {
            file_path: path_str,
            operation: FileOperation::Cursor { buffer_size }
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to open file"))?;

        match response.message {
            OpfsOperationOutput::Cursor(instance_id) => {
                let mut buffer = BytesMut::with_capacity(buffer_size);
                buffer.resize(buffer_size, 0);
                Ok(Self { path, buffer, instance_id })
            }
            r => Err(anyhow::anyhow!("Failed to open file: {:?}", r))
        }
    }

    pub fn stop(&mut self) {
        let msg = WorkerMessage::new(OpfsOperation {
            file_path: self.path.to_string_lossy().to_string(),
            operation: FileOperation::CursorEnd(self.instance_id)
        });

        OPFS_WORKER.unbounded_send(msg);
    }
}

#[async_trait(?Send)]
impl IOReader for IOReaderOpfsImpl {
    async fn next(&mut self, max: Option<u64>) -> Result<Option<&[u8]>> {
        let msg = WorkerMessage::new(OpfsOperation {
            file_path: self.path.to_string_lossy().to_string(),
            operation: FileOperation::CursorNext {
                instance_id: self.instance_id,
                max
            }
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to read"))?;

        match response.message {
            OpfsOperationOutput::Binary(data) => {
                if data.length() == 0 {
                    Ok(None)
                } else {
                    data.copy_to(&mut self.buffer[..data.length() as usize]);
                    Ok(Some(&self.buffer[..data.length() as usize]))
                }
            }
            r => Err(anyhow::anyhow!("Read error: {:?}", r))
        }
    }

    async fn entry(&self) -> Result<FileEntry> {
        let msg = WorkerMessage::new(OpfsOperation {
            file_path: self.path.to_string_lossy().to_string(),
            operation: FileOperation::FileEntry
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to get size"))?;

        match response.message {
            OpfsOperationOutput::FileEntry(entry) => Ok(entry),
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Size error: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response"))
        }
    }

    async fn end(&mut self) -> Result<()> {
        self.stop();
        Ok(())
    }
}

impl Drop for IOReaderOpfsImpl {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct IOWriterOpfsImpl {
    path: PathBuf,
    position: usize
}

impl IOWriterOpfsImpl {
    pub async fn new(path: PathBuf) -> Result<Self> {
        let path_str = path.to_string_lossy().to_string();

        let msg = WorkerMessage::new(OpfsOperation {
            file_path: path_str,
            operation: FileOperation::Open
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to open file for writing"))?;

        match response.message {
            OpfsOperationOutput::Void => Ok(Self { path, position: 0 }),
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Failed to open file for writing: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response"))
        }
    }
}

#[async_trait(?Send)]
impl IOWriter for IOWriterOpfsImpl {
    async fn write(&mut self, data: Bytes) -> Result<()> {
        let uint8_array = Uint8Array::from(data.as_ref());

        let msg = WorkerMessage::new(OpfsOperation {
            file_path: self.path.to_string_lossy().to_string(),
            operation: FileOperation::Write {
                data: uint8_array,
                position: self.position
            }
        });
        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to write"))?;

        match response.message {
            OpfsOperationOutput::Written(written) => {
                self.position += written;
                Ok(())
            }
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Write error: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response"))
        }
    }

    async fn end(&mut self) -> Result<()> {
        self.flush().await?;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        let msg = WorkerMessage::new(OpfsOperation {
            file_path: self.path.to_string_lossy().to_string(),
            operation: FileOperation::Flush
        });

        OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to flush"))?;
        Ok(())
    }
}
