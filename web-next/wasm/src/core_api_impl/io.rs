use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use shared::core_api::{IOReader, IOWriter};
use crate::file_api::storage::{FileStorage, WasmFile};

pub struct IOReaderImpl {
    file: Mutex<WasmFile>
}

#[async_trait(?Send)]
impl IOReader for IOReaderImpl {
    async fn next(&mut self) -> Result<Option<bytes::Bytes>> {
        todo!()
    }

    async fn total_size(&self) -> Result<u64> {
        Ok(self.file.lock().await.size() as u64)
    }
}

pub struct IOWriterImpl {
    storage: FileStorage,
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
