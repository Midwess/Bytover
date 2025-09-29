use anyhow::Result;
use async_trait::async_trait;
use core_services::local_storage::entry::FileEntry;
use shared::core_api::IOWriter;
use std::path::PathBuf;

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
