use anyhow::Result;
use async_trait::async_trait;
use core_services::local_storage::entry::FileEntry;
use core_services::local_storage::stream::IOCursor;
use shared::shell::api::{CIOCursor, DIOWriter, IOWriter};
use shared::utils::compression::CompressStats;
use std::path::PathBuf;
use bytes::{Bytes, BytesMut};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use n0_future::time::Instant;

pub struct DIOWriterWrapper<W: IOWriter> {
    inner: W,
    compression_support: bool,
}

pub struct FileEntryWriter {
    file: FileEntry,
}

#[async_trait]
impl IOWriter for FileEntryWriter {
    async fn write(&mut self, data: bytes::Bytes) -> anyhow::Result<usize> {
        let len = data.len();
        self.file.write(data.into()).await?;
        Ok(len)
    }
}

impl<W: IOWriter> DIOWriterWrapper<W> {
    pub fn new(inner: W, compression_support: bool) -> Self {
        Self {
            inner,
            compression_support,
        }
    }
}

impl DIOWriterWrapper<FileEntryWriter> {
    pub async fn from_path(path: PathBuf, compression_support: bool) -> Result<Self> {
        let file = FileEntry::new(None, path).await?;
        Ok(Self {
            inner: FileEntryWriter { file },
            compression_support,
        })
    }
}

#[async_trait]
impl<W: IOWriter> IOWriter for DIOWriterWrapper<W> {
    async fn write(&mut self, data: bytes::Bytes) -> anyhow::Result<usize> {
        self.inner.write(data).await
    }

    async fn flush(&mut self) -> anyhow::Result<()> {
        self.inner.flush().await
    }

    async fn end(&mut self) -> anyhow::Result<()> {
        self.inner.end().await
    }
}

#[async_trait]
impl<W: IOWriter> DIOWriter for DIOWriterWrapper<W> {
    async fn d_write(&mut self, data: Bytes) -> anyhow::Result<Option<usize>> {
        if self.compression_support {
            let compressed = data[0] == 1;
            let data_to_write = if compressed {
                let data_slice = data[1..].to_vec();
                tokio::task::spawn_blocking(move || {
                    decompress_size_prepended(&data_slice)
                }).await.map_err(|e| anyhow::anyhow!("Decompression task failed: {}", e))?
                    .map_err(|e| anyhow::anyhow!("Decompression failed: {}", e))?
            } else {
                data[1..].to_vec()
            };
            let written = data_to_write.len();
            self.inner.write(Bytes::from(data_to_write)).await?;
            Ok(Some(written))
        } else {
            let written = self.inner.write(data).await?;
            Ok(Some(written))
        }
    }
}

pub struct CIOCursorBoxWrapper {
    inner: Box<dyn IOCursor>,
    stats: CompressStats,
    buffer: BytesMut,
}

impl CIOCursorBoxWrapper {
    pub fn new(inner: Box<dyn IOCursor>, file_name: &str) -> Self {
        let stats = CompressStats::new(file_name);
        let buffer = BytesMut::with_capacity(inner.buffer_size().unwrap_or(1024) + 1);
        Self {
            inner,
            stats,
            buffer,
        }
    }
}

#[async_trait]
impl IOCursor for CIOCursorBoxWrapper {
    async fn next(&mut self, max: Option<u64>) -> Result<Option<&[u8]>> {
        let result = self.inner.next(max).await;
        result
    }

    async fn entry(&self) -> Result<FileEntry> {
        self.inner.entry().await
    }

    async fn end(&mut self) -> Result<()> {
        self.inner.end().await
    }

    fn buffer_size(&self) -> Option<usize> {
        self.inner.buffer_size()
    }
}

#[async_trait]
impl CIOCursor for CIOCursorBoxWrapper {
    async fn c_next(&mut self, max: Option<u64>) -> Result<Option<(&[u8], usize)>> {
        let read_start = Instant::now();
        let data = self.inner.next(max).await?;
        let read_time_micro = read_start.elapsed().as_micros() as u64;

        if !self.stats.is_compression_support() {
            if let Some(data) = data {
                return Ok(Some((data, data.len())));
            }

            return Ok(None);
        }

        let Some(raw_data) = data else {
            return Ok(None);
        };

        let raw_size = raw_data.len();
        
        if !self.stats.should_compress() {
            if self.buffer.len() < raw_size + 1 {
                self.buffer.resize(raw_size + 1, 0);
            }

            self.buffer[0] = 0u8;
            self.buffer[1..raw_size + 1].copy_from_slice(raw_data);
            self.stats.add_chunk_stats(raw_size, 0, raw_size, read_time_micro);
            return Ok(Some((&self.buffer[..raw_size + 1], raw_size)));
        }
        
        let compress_start = Instant::now();
        let raw_data_vec = raw_data.to_vec();
        let compressed = tokio::task::spawn_blocking(move || {
            compress_prepend_size(&raw_data_vec)
        }).await.map_err(|e| anyhow::anyhow!("Compression task failed: {}", e))?;
        let compression_time_micro = compress_start.elapsed().as_micros() as u64;
        
        let compressed_size = compressed.len();
        let is_compressed_success = self.stats.add_chunk_stats(raw_size, compression_time_micro, compressed_size, read_time_micro);
        
        if is_compressed_success {
            if self.buffer.len() < compressed_size + 1 {
                self.buffer.resize(compressed_size + 1, 0);
            }
            self.buffer[0] = 1u8;
            self.buffer[1..compressed_size + 1].copy_from_slice(&compressed);
            Ok(Some((&self.buffer[..compressed_size + 1], raw_size)))
        } else {
            if self.buffer.len() < raw_size + 1 {
                self.buffer.resize(raw_size + 1, 0);
            }
            self.buffer[0] = 0u8;
            self.buffer[1..raw_size + 1].copy_from_slice(raw_data);
            Ok(Some((&self.buffer[..raw_size + 1], raw_size)))
        }
    }

    fn compression_stats_mut(&mut self) -> &mut CompressStats {
        &mut self.stats
    }
}
