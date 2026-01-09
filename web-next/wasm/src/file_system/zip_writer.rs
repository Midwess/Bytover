use anyhow::Result;
use async_zip::base::write::{EntryStreamWriter, ZipFileWriter};
use async_zip::{Compression, ZipEntryBuilder};
use chrono::Utc;
use core_services::wasm::opfs::SyncAccessHandleWriter;
use web_sys::FileSystemSyncAccessHandle;

enum WriterState {
    Writer(ZipFileWriter<SyncAccessHandleWriter>),
    Entry(EntryStreamWriter<SyncAccessHandleWriter>),
    Closed
}

pub struct OpfsZipWriter {
    state: WriterState
}

impl OpfsZipWriter {
    pub fn new(handle: FileSystemSyncAccessHandle) -> Self {
        let writer = SyncAccessHandleWriter::new(handle);
        let zip_writer = ZipFileWriter::new(writer);
        Self {
            state: WriterState::Writer(zip_writer)
        }
    }

    pub async fn new_entry(&mut self, name: &str) -> Result<()> {
        let state = std::mem::replace(&mut self.state, WriterState::Closed);

        let zip_writer = match state {
            WriterState::Writer(w) => w,
            WriterState::Entry(e) => e.close().await?,
            WriterState::Closed => return Err(anyhow::anyhow!("Writer is closed"))
        };

        let entry = ZipEntryBuilder::new(name.into(), Compression::Stored)
            .last_modification_date(Utc::now().into())
            .build();

        let entry_writer = zip_writer.write_entry_stream(entry).await?;
        self.state = WriterState::Entry(entry_writer);
        Ok(())
    }

    pub async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        match &mut self.state {
            WriterState::Entry(entry) => {
                use futures::io::AsyncWriteExt;
                entry.write_all(bytes).await?;
                Ok(())
            }
            _ => Err(anyhow::anyhow!("No entry open. Call new_entry first."))
        }
    }

    pub async fn finalize(mut self) -> Result<SyncAccessHandleWriter> {
        let state = std::mem::replace(&mut self.state, WriterState::Closed);

        let zip_writer = match state {
            WriterState::Writer(w) => w,
            WriterState::Entry(e) => e.close().await?,
            WriterState::Closed => return Err(anyhow::anyhow!("Writer is closed"))
        };

        let mut inner = zip_writer.close().await?;
        use futures::io::AsyncWriteExt;
        inner.close().await?;
        Ok(inner)
    }
}
