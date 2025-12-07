use crate::file_system::device_file::{wasm_file, WebFile};
use crate::web_worker::bridge::{WebWorkerBridge, WorkerMessage};
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput, OpfsWorker};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use core_services::local_storage::entry::FileEntry;
use core_services::local_storage::stream::IOCursor;
use core_services::utils::never_send::NeverSend;
use js_sys::Uint8Array;
use shared::shell::api::{CIOCursor, DIOWriter, IOReader, IOWriter};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};
use n0_future::time::Instant;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, File, FileSystemFileHandle};
use shared::utils::compression::CompressStats;

pub static OPFS_WORKER: LazyLock<NeverSend<WebWorkerBridge<OpfsWorker>>> =
    LazyLock::new(|| NeverSend(WebWorkerBridge::<OpfsWorker>::spawn("opfs-worker")));
pub struct IOReaderBlobImpl {
    entry: FileEntry,
    blob: NeverSend<Blob>,
    buffer: BytesMut,
    current_pos: u64
}

impl IOReaderBlobImpl {
    pub async fn from_file(file: &WebFile, buffer_size: usize) -> Result<Self> {
        let modified_at = SystemTime::UNIX_EPOCH + Duration::from_millis(file.last_modified() as u64);
        let buffer_size = buffer_size.min(file.size() as usize);

        let mut buffer = BytesMut::with_capacity(buffer_size);
        buffer.resize(buffer_size, 0);

        let entry = FileEntry {
            is_dir: false,
            modified_at,
            size: file.size() as u64,
            path: PathBuf::from(file.webkit_path.clone().unwrap_or(file.name().to_string()))
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
        Self::from_file(&wasm_file(file), buffer_size).await
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
    instance_id: u32,
    compression_stats: CompressStats,
}

impl IOReaderOpfsImpl {
    pub async fn new(path: PathBuf, buffer_size: usize, compressed: bool) -> Result<Self> {
        let path_str = path.to_string_lossy().to_string();

        let msg = WorkerMessage::new(OpfsOperation {
            file_path: path_str,
            operation: FileOperation::Cursor { buffer_size }
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to open file"))?;

        match response.message {
            OpfsOperationOutput::Cursor(instance_id) => {
                let mut buffer = BytesMut::with_capacity(buffer_size + 1);
                let compression_stats = CompressStats::new(path.clone().to_str().unwrap_or_default());
                buffer.resize(buffer_size + 1, 0);
                Ok(Self { path, buffer, instance_id, compression_stats })
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
impl CIOCursor for IOReaderOpfsImpl {
    fn compression_stats_mut(&mut self) -> &mut CompressStats {
        &mut self.compression_stats
    }

    async fn c_next(&mut self, max: Option<u64>) -> Result<Option<(&[u8], usize)>> {
        if !self.compression_stats.is_compression_support() {
            return self.next(max).await.map(|it| it.map(|it| (it, it.len())))
        }

        let msg = WorkerMessage::new(OpfsOperation {
            file_path: self.path.to_string_lossy().to_string(),
            operation: FileOperation::CursorNext {
                instance_id: self.instance_id,
                max,
                compressed: self.compression_stats.should_compress()
            }
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to read"))?;
        match response.message {
            OpfsOperationOutput::Binary { data, raw_size, compression_time_in_micros, read_time_in_micros, is_compressed_failed } => {
                if data.length() == 0 {
                    Ok(None)
                }
                else {
                    if self.compression_stats.add_chunk_stats(raw_size, compression_time_in_micros, data.length() as usize, read_time_in_micros) {
                        self.buffer[0] = 1u8;
                    }
                    else {
                        self.buffer[0] = 0u8;
                    }

                    // This won't happen, but just in case, we don't want it to be panic
                    if self.buffer.len() < data.length() as usize + 1 {
                        log::info!("Buffer size is too small, resizing to {}", data.length() as usize + 1);
                        self.buffer.resize(data.length() as usize + 1, 0);
                    }

                    data.copy_to(&mut self.buffer[1..data.length() as usize + 1]);
                    Ok(Some((&self.buffer[..data.length() as usize + 1], raw_size)))
                }
            }
            r => Err(anyhow::anyhow!("Read error: {:?}", r))
        }
    }
}

#[async_trait(?Send)]
impl IOReader for IOReaderOpfsImpl {
    fn buffer_size(&self) -> Option<usize> {
        Some(self.buffer.capacity())
    }

    async fn next(&mut self, max: Option<u64>) -> Result<Option<&[u8]>> {
        let msg = WorkerMessage::new(OpfsOperation {
            file_path: self.path.to_string_lossy().to_string(),
            operation: FileOperation::CursorNext {
                instance_id: self.instance_id,
                max,
                compressed: false
            }
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to read"))?;

        match response.message {
            OpfsOperationOutput::Binary {data, ..} => {
                if data.length() == 0 {
                    Ok(None)
                }
                else {
                    if self.buffer.len() < data.length() as usize {
                        self.buffer.resize(data.length() as usize, 0);
                    }

                    data.copy_to(&mut self.buffer[..data.length() as usize]);
                    Ok(Some((&self.buffer[..data.length() as usize])))
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
    position: usize,
    compression_support: bool,
}

impl IOWriterOpfsImpl {
    pub async fn new(path: PathBuf, compression_support: bool) -> Result<Self> {
        let path_str = path.to_string_lossy().to_string();

        let msg = WorkerMessage::new(OpfsOperation {
            file_path: path_str,
            operation: FileOperation::Open
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to open file for writing"))?;

        match response.message {
            OpfsOperationOutput::Void => Ok(Self { path, position: 0, compression_support }),
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Failed to open file for writing: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response"))
        }
    }
}

impl IOWriterOpfsImpl {
    async fn opfs_write(&mut self, data: &[u8], decompress: bool) -> Result<usize> {
        let uint8_array = Uint8Array::from(data);

        let msg = WorkerMessage::new(OpfsOperation {
            file_path: self.path.to_string_lossy().to_string(),
            operation: FileOperation::Write {
                data: uint8_array,
                position: self.position,
                decompress
            }
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to write"))?;

        match response.message {
            OpfsOperationOutput::Written(written) => {
                self.position += written;
                Ok(written)
            }
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Write error: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response"))
        }
    }
}

#[async_trait(?Send)]
impl IOWriter for IOWriterOpfsImpl {
    async fn write(&mut self, data: Bytes) -> Result<usize> {
        self.opfs_write(&data, false).await
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

#[async_trait(?Send)]
impl DIOWriter for IOWriterOpfsImpl {
    async fn d_write(&mut self, data: Bytes) -> Result<Option<usize>> {
        if self.compression_support {
            let compressed = data[0] == 1;

            self.opfs_write(&data[1..], compressed).await.map(|s| Some(s))
        }
        else {
            self.write(Bytes::from(data)).await.map(|it| Some(it))
        }
    }
}
