use crate::web_worker::bridge::{WebWorkerBridge, WorkerMessage};
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput, OpfsWorker};
use anyhow::Result;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use core_services::local_storage::entry::FileEntry;
use core_services::utils::never_send::NeverSend;
use js_sys::Uint8Array;
use shared::core_api::{IOReader, IOWriter};
use std::path::PathBuf;
use std::sync::LazyLock;

pub static OPFS_WORKER: LazyLock<NeverSend<WebWorkerBridge<OpfsWorker>>> =
    LazyLock::new(|| NeverSend(WebWorkerBridge::<OpfsWorker>::spawn("opfs-worker")));

pub struct IOReaderOpfsImpl {
    path: PathBuf,
    position: usize,
    buffer: BytesMut
}

impl IOReaderOpfsImpl {
    pub async fn new(path: PathBuf) -> Result<Self> {
        let path_str = path.to_string_lossy().to_string();

        let msg = WorkerMessage::new(OpfsOperation {
            file_path: path_str,
            operation: FileOperation::Open
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to open file"))?;

        match response.message {
            OpfsOperationOutput::Void => {
                let mut buffer = BytesMut::with_capacity(1024 * 63);
                buffer.resize(1024 * 63, 0);
                Ok(Self { path, position: 0, buffer })
            }
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Failed to open file: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response"))
        }
    }
}

#[async_trait(?Send)]
impl IOReader for IOReaderOpfsImpl {
    async fn next(&mut self, max: Option<u64>) -> Result<Option<&[u8]>> {
        let msg = WorkerMessage::new(OpfsOperation {
            file_path: self.path.to_string_lossy().to_string(),
            operation: FileOperation::Read {
                position: self.position,
                amount: (1024 * 63).min(max.unwrap_or(1024 * 63)) as usize
            }
        });

        let response = OPFS_WORKER.send(msg).await.ok_or(anyhow::anyhow!("Failed to read"))?;

        match response.message {
            OpfsOperationOutput::Binary(data) => {
                if data.length() == 0 {
                    Ok(None)
                } else {
                    data.copy_to(&mut self.buffer[..data.length() as usize]);
                    self.position += data.length() as usize;
                    Ok(Some(&self.buffer[..data.length() as usize]))
                }
            }
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Read error: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response"))
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
