use anyhow::Result;
use async_trait::async_trait;
use core_services::local_storage::abstraction::IOCursor;
use core_services::local_storage::file_system::File;
use shared::core_api::{IOReader, IOWriter};
use std::path::PathBuf;

pub struct IOReaderImpl {
    cursor: Box<dyn IOCursor>
}

#[async_trait]
impl IOReader for IOReaderImpl {
    async fn next(&mut self) -> Result<Option<bytes::Bytes>> {
        self.next().await
    }

    async fn total_size(&self) -> Result<u64> {
        self.cursor.total_size().await
    }
}

pub struct IOWriterImpl {
    file: File
}

impl IOWriterImpl {
    pub async fn new(path: PathBuf) -> Result<Self> {
        let file = File::new(None, path).await?;
        Ok(Self { file })
    }
}

#[async_trait]
impl IOWriter for IOWriterImpl {
    async fn write(&mut self, data: bytes::Bytes) -> anyhow::Result<()> {
        self.file.write(data.into()).await?;

        Ok(())
    }
}
