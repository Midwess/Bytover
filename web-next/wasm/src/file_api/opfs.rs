use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use core_services::utils::never_send::NeverSend;
use js_sys::Uint8Array;
use n0_future::task::spawn;
use shared::core_api::{IOReader, IOWriter};
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use futures::lock::Mutex;
use crate::web_worker::bridge::{WorkerMessage, WebWorkerBridge};
use crate::web_worker::opfs::{OpfsOperation, OpfsOperationOutput, OpfsWorker};

static OPFS_WORKERS: LazyLock<Mutex<HashMap<PathBuf, Arc<NeverSend<WebWorkerBridge<OpfsWorker>>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct IOReaderOpfsImpl {
    path: PathBuf,
    position: usize,
}

impl IOReaderOpfsImpl {
    pub async fn new(path: PathBuf) -> Result<Self> {
        let path_str = path.to_string_lossy().to_string();

        let mut workers = OPFS_WORKERS.lock().await;
        match workers.get(&path) {
            None => {
                log::info!("Opening file for read: {}", path_str);
                let new_worker = Arc::new(NeverSend(WebWorkerBridge::<OpfsWorker>::spawn("opfs-worker")));
                workers.insert(path.clone(), new_worker.clone());
                let msg = WorkerMessage::new(OpfsOperation::Open(path_str.clone()));
                let response = new_worker.send(msg).await.ok_or(anyhow::anyhow!("Failed to open file"))?;
                
                match response.message {
                    OpfsOperationOutput::Void => {
                        log::info!("Opened file");
                    },
                    _ => {
                        workers.remove(&path);
                        return Err(anyhow::anyhow!("Failed to open file"));
                    },
                }
            },
            _ => {}
        };
        
        Ok(Self {
            path,
            position: 0,
        })
    }

    async fn worker(&self) -> Arc<NeverSend<WebWorkerBridge<OpfsWorker>>> {
        let workers = OPFS_WORKERS.lock().await;
        workers.get(&self.path).unwrap().clone()
    }
}

#[async_trait(?Send)]
impl IOReader for IOReaderOpfsImpl {
    async fn next(&mut self) -> Result<Option<Bytes>> {
        let chunk_size = 1024 * 64;
        
        let msg = WorkerMessage::new(OpfsOperation::Read(chunk_size, self.position as u64));
        let response = self.worker().await.send(msg).await.ok_or(anyhow::anyhow!("Failed to read"))?;
        
        match response.message {
            OpfsOperationOutput::Binary(data) => {
                if data.length() == 0 {
                    Ok(None)
                } else {
                    self.position += data.length() as usize;
                    Ok(Some(Bytes::from(data.to_vec())))
                }
            },
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Read error: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    async fn total_size(&self) -> Result<u64> {
        let msg = WorkerMessage::new(OpfsOperation::Size);
        let response = self.worker().await.send(msg).await.ok_or(anyhow::anyhow!("Failed to get size"))?;
        
        match response.message {
            OpfsOperationOutput::Size(size) => Ok(size),
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Size error: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }
}

pub struct IOWriterOpfsImpl {
    path: PathBuf,
    position: u64,
}

impl IOWriterOpfsImpl {
    pub async fn new(path: PathBuf) -> Result<Self> {
        let path_str = path.to_string_lossy().to_string();

        let mut workers = OPFS_WORKERS.lock().await;
        match workers.get(&path) {
            None => {
                log::info!("Opening file for write: {}", path_str);
                let new_worker = Arc::new(NeverSend(WebWorkerBridge::<OpfsWorker>::spawn("opfs-worker")));
                workers.insert(path.clone(), new_worker.clone());
                let msg = WorkerMessage::new(OpfsOperation::Open(path_str.clone()));
                let response = new_worker.send(msg).await.ok_or(anyhow::anyhow!("Failed to open file for writing"))?;
                
                match response.message {
                    OpfsOperationOutput::Void => {}
                    _ => {
                        workers.remove(&path);
                        return Err(anyhow::anyhow!("Unexpected response"))
                    },
                }
            },
            Some(_) => {}
        };
        
        Ok(Self {
            path,
            position: 0,
        })
    }

    async fn worker(&self) -> Arc<NeverSend<WebWorkerBridge<OpfsWorker>>> {
        let workers = OPFS_WORKERS.lock().await;
        workers.get(&self.path).unwrap().clone()
    }
}

#[async_trait(?Send)]
impl IOWriter for IOWriterOpfsImpl {
    async fn write(&mut self, data: Bytes) -> Result<()> {
        let uint8_array = Uint8Array::from(data.as_ref());
        
        let msg = WorkerMessage::new(OpfsOperation::Write(uint8_array, self.position));
        let response = self.worker().await.send(msg).await.ok_or(anyhow::anyhow!("Failed to write"))?;
        
        match response.message {
            OpfsOperationOutput::Written(written) => {
                self.position += written as u64;
                log::info!("Written {} bytes at position {}", written, self.position);
                Ok(())
            },
            OpfsOperationOutput::Error(e) => Err(anyhow::anyhow!("Write error: {:?}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    async fn flush(&mut self) -> Result<()> {
        self.worker().await.send(WorkerMessage::new(OpfsOperation::Flush)).await.ok_or(anyhow::anyhow!("Failed to flush"))?;
        Ok(())
    }
}
