use anyhow::Result;
use async_trait::async_trait;
use core_services::local_storage::entry::FileEntry;
use core_services::local_storage::stream::IOCursor;
use shared::core_api::{IOReader, IOWriter};
use std::path::PathBuf;

pub struct IOReaderImpl {
    cursor: Box<dyn IOCursor>
}

#[async_trait]
impl IOReader for IOReaderImpl {
    async fn next(&mut self, max_read: Option<u64>) -> Result<Option<&[u8]>> {
        self.cursor.next(max_read).await
    }

    async fn entry(&self) -> Result<FileEntry> {
        self.cursor.entry().await
    }
}

pub struct IOWriterImpl {
    file: FileEntry
}

impl IOWriterImpl {
    pub async fn new(path: PathBuf) -> Result<Self> {
        let file = FileEntry::new(None, path).await?;
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
