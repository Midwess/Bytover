use crate::local_storage::entry::FileEntry;
use crate::local_storage::stream::IOCursor;
use anyhow::Result;
use async_stream::stream;
use async_zip::base::write::{EntryStreamWriter, ZipFileWriter};
use async_zip::{Compression, ZipEntry, ZipEntryBuilder};
use chrono::Utc;
use futures::Stream;
use n0_future::io::AsyncWriteExt;
use n0_future::StreamExt;
use std::path::PathBuf;
use std::pin::Pin;

impl FileEntry {
    pub fn zip_entry(&self, base_path: &PathBuf) -> ZipEntry {
        let name = self.relative_path(base_path).unwrap().to_string_lossy().to_string();
        ZipEntryBuilder::new(name.into(), Compression::Stored)
            .last_modification_date(Utc::now().into())
            .uncompressed_size(self.size)
            .compressed_size(self.size)
            .build()
    }
}

pub type ZipInputStream = Pin<Box<dyn Stream<Item = Result<Box<dyn IOCursor>>> + Send + Sync>>;

enum WriterState {
    ProcessingEntry {
        writer: EntryStreamWriter<Vec<u8>>,
        cursor: Box<dyn IOCursor>
    },
    Finalizing {
        writer: Vec<u8>
    },
    Completed
}

pub struct ZipStream {
    inputs: ZipInputStream,
    zip_entry: FileEntry,
    state: WriterState,
    chunk_size: usize,
    buffer: Vec<u8>,
    buffer_pos: usize,
    buffer_len: usize
}

impl ZipStream {
    pub async fn new(inputs: Vec<Box<dyn IOCursor>>, path: PathBuf, max_chunk_size: usize) -> Result<Self> {
        let mut total_size = 0u64;
        for input in inputs.iter() {
            let entry = input.entry().await?;
            total_size += entry.size;
        }

        let zip_entry = FileEntry {
            is_dir: false,
            modified_at: Utc::now().into(),
            size: total_size,
            path: path.clone()
        };

        let inputs = stream! {
            for input in inputs {
                yield Ok(input);
            }
        };

        Self::new_from_stream(Box::pin(inputs), zip_entry, max_chunk_size).await
    }

    pub async fn new_from_stream(
        mut stream: ZipInputStream,
        zip_entry: FileEntry,
        max_chunk_size: usize
    ) -> Result<Self> {
        let chunk_size = max_chunk_size.min(zip_entry.size as usize);

        // Pre-allocate buffer to avoid frequent reallocations
        let buffer = vec![0u8; chunk_size];

        let writer = Vec::with_capacity(chunk_size);
        let zip_writer = ZipFileWriter::new(writer);

        let Some(Ok(first_cursor)) = stream.next().await else {
            return Err(anyhow::anyhow!("No entries to zip"));
        };

        let first_entry = first_cursor.entry().await?.zip_entry(&zip_entry.path);
        let entry_writer = zip_writer.write_entry_stream(first_entry).await?;

        Ok(Self {
            zip_entry,
            chunk_size,
            inputs: stream,
            buffer,
            buffer_pos: 0,
            buffer_len: 0,
            state: WriterState::ProcessingEntry {
                writer: entry_writer,
                cursor: first_cursor
            }
        })
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    async fn fill_buffer_from_writer(&mut self, read_size: usize) -> Result<bool> {
        let writer_data = match &mut self.state {
            WriterState::ProcessingEntry { writer, .. } => writer.most_inner_mut(),
            WriterState::Finalizing { writer } => Some(writer),
            WriterState::Completed => return Ok(false)
        };

        if let Some(data) = writer_data {
            let available = data.len().min(read_size);
            if available > 0 {
                if self.buffer.len() < available {
                    self.buffer.resize(available, 0);
                }

                self.buffer[..available].copy_from_slice(data.drain(..available).as_slice());
                self.buffer_pos = 0;
                self.buffer_len = available;
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn process_current_cursor(&mut self, max_read: Option<u64>) -> Result<bool> {
        if let WriterState::ProcessingEntry { writer, cursor } = &mut self.state {
            if let Some(chunk) = cursor.next(max_read).await? {
                writer.write_all(chunk).await?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn advance_to_next_entry(&mut self) -> Result<bool> {
        let WriterState::ProcessingEntry { writer, .. } = std::mem::replace(&mut self.state, WriterState::Completed)
        else {
            return Ok(false);
        };

        let zip_writer = writer.close().await?;

        match self.inputs.next().await {
            Some(Ok(next_cursor)) => {
                let entry = next_cursor.entry().await?.zip_entry(&self.zip_entry.path);
                let new_entry_writer = zip_writer.write_entry_stream(entry).await?;

                self.state = WriterState::ProcessingEntry {
                    writer: new_entry_writer,
                    cursor: next_cursor
                };
                Ok(true)
            }
            Some(Err(e)) => Err(e),
            None => {
                let final_data = zip_writer.close().await?;
                self.state = WriterState::Finalizing { writer: final_data };
                Ok(true)
            }
        }
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl IOCursor for ZipStream {
    async fn next(&mut self, max_read: Option<u64>) -> Result<Option<&[u8]>> {
        let read_size = max_read.unwrap_or(self.chunk_size as u64) as usize;
        let read_size = read_size.min(self.chunk_size);

        loop {
            // First, try to serve from existing buffer data
            if self.fill_buffer_from_writer(read_size).await? {
                return Ok(Some(&self.buffer[self.buffer_pos..self.buffer_pos + self.buffer_len]));
            }

            // Try to process current cursor
            if self.process_current_cursor(max_read).await? {
                continue;
            }

            // Need to advance to next entry or finalize
            if !self.advance_to_next_entry().await? {
                return Ok(None);
            }
        }
    }

    async fn entry(&self) -> Result<FileEntry> {
        Ok(self.zip_entry.clone())
    }

    fn buffer_size(&self) -> Option<usize> {
        Some(self.chunk_size)
    }
}
