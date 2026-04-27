use crate::local_storage::entry::FileEntry;
use crate::local_storage::stream::IOCursor;
use crate::local_storage::zip::ZipStream;
use anyhow::Result;
use async_stream::stream;
use bytes::BytesMut;
use jwalk::WalkDir;
use n0_future::time::SystemTime;
use std::path::{Path, PathBuf};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::task::spawn_blocking;

impl FileEntry {
    pub async fn existing(path: impl AsRef<Path>) -> Result<Self> {
        let path = PathBuf::from(path.as_ref());
        if !path.exists() {
            return Err(anyhow::anyhow!("File does not exist {:?}", path));
        }
        let metadata = path.metadata()?;
        let modified_at = metadata.modified()?;

        Ok(Self {
            is_dir: path.is_dir(),
            size: if path.is_dir() { 0 } else { path.metadata()?.len() },
            modified_at,
            path
        })
    }

    pub async fn new(content: Option<Vec<u8>>, path: impl AsRef<Path>) -> Result<Self> {
        let path = PathBuf::from(path.as_ref());

        if path.is_dir() {
            return Err(anyhow::anyhow!("Path is a directory"));
        }

        if !path.parent().unwrap().try_exists().unwrap_or(false) {
            fs::create_dir_all(path.parent().unwrap()).await?;
        }

        if let Some(content) = content {
            if !path.exists() {
                fs::write(&path, &content).await?;
            }
        }

        if !path.exists() {
            fs::File::create(&path).await?;
        }

        Self::existing(path).await
    }

    pub async fn append(content: Vec<u8>, path: impl AsRef<Path>) -> Result<Self> {
        let path = PathBuf::from(path.as_ref());

        if path.is_dir() {
            return Err(anyhow::anyhow!("Path is a directory"));
        }

        if !path.parent().unwrap().try_exists().unwrap_or(false) {
            fs::create_dir_all(path.parent().unwrap()).await?;
        }

        let mut file = OpenOptions::new().create(true).append(true).open(&path).await?;

        file.write_all(&content).await?;

        Self::existing(path).await
    }

    pub async fn write(&mut self, content: Vec<u8>) -> Result<()> {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path).await?;

        file.write_all(&content).await?;
        Ok(())
    }

    pub async fn write_at(&mut self, content: Vec<u8>, offset: u64) -> Result<()> {
        let mut file = OpenOptions::new().write(true).create(true).open(&self.path).await?;

        file.seek(tokio::io::SeekFrom::Start(offset)).await?;
        file.write_all(&content).await?;
        Ok(())
    }

    pub async fn open(&self) -> Result<fs::File> {
        let file = OpenOptions::new().create(true).write(true).read(true).open(&self.path).await?;

        Ok(file)
    }

    pub async fn open_append(&self) -> Result<fs::File> {
        let file = OpenOptions::new().create(true).append(true).write(true).read(true).open(&self.path).await?;

        Ok(file)
    }

    pub async fn delete(self) -> Result<()> {
        fs::remove_file(&self.path).await?;
        Ok(())
    }

    pub async fn read(&self) -> Result<Vec<u8>> {
        Ok(fs::read(&self.path).await?)
    }

    pub async fn cursor(&self, buffer_size: usize) -> Result<Box<dyn IOCursor>> {
        Ok(Box::new(FileCursor::new(self.path.clone(), buffer_size).await?))
    }
}

#[derive(Debug, Clone)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf
}

impl Folder {
    pub async fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = PathBuf::from(path.as_ref());
        if !path.exists() {
            fs::create_dir_all(&path).await?;
        }

        if path.is_file() {
            return Err(anyhow::anyhow!("Path is a file"));
        }

        let name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid folder name"))?
            .to_string_lossy()
            .into_owned();

        Ok(Self { name, path })
    }

    pub async fn delete(self) -> Result<()> {
        fs::remove_dir_all(&self.path).await?;
        Ok(())
    }

    #[cfg(feature = "zip")]
    pub async fn cursor(&self, chunk_size: usize) -> Result<Box<dyn IOCursor>> {
        let entry = FileEntry {
            is_dir: false,
            modified_at: SystemTime::now(),
            size: self.zip_store_size().await,
            path: self.path.clone()
        };

        let path = self.path.clone();
        let stream = stream! {
            let mut pending_paths = vec![path];

            while let Some(current_path) = pending_paths.pop() {
                if current_path.is_dir() {
                    let mut dir_entries = fs::read_dir(&current_path).await?;
                    while let Some(dir_entry) = dir_entries.next_entry().await? {
                        pending_paths.push(dir_entry.path());
                    }
                }
                else {
                    let Ok(file_size) = current_path.metadata().map(|it| it.len()) else {
                        continue;
                    };
                    let cursor = FileCursor::new(current_path, chunk_size.min(file_size as usize)).await?;
                    yield Ok(Box::new(cursor) as Box<dyn IOCursor>);
                }
            }
        };

        let cursor = ZipStream::new_from_stream(Box::pin(stream), entry, chunk_size).await?;
        Ok(Box::new(cursor))
    }

    pub async fn size(&self) -> u64 {
        let path = self.path.clone();
        spawn_blocking(move || {
            let total: u64 = WalkDir::new(&path)
                .parallelism(jwalk::Parallelism::RayonNewPool(10))
                .process_read_dir(|_, _, _, _| {})
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let meta = entry.metadata().ok()?;
                    if meta.is_file() {
                        Some(meta.len())
                    } else {
                        None
                    }
                })
                .sum();
            total
        })
        .await
        .unwrap()
    }

    // Almost correct, not accurate for ZIP Store
    #[cfg(feature = "zip")]
    pub async fn zip_store_size(&self) -> u64 {
        let path = self.path.clone();
        spawn_blocking(move || {
            const LOCAL_FILE_HEADER: u64 = 30;
            const CENTRAL_DIR_HEADER: u64 = 46;
            const END_OF_CENTRAL_DIR: u64 = 22;
            const ZIP64_EXTRA_FIELD: u64 = 20;

            let entries: Vec<_> = WalkDir::new(&path)
                .parallelism(jwalk::Parallelism::RayonNewPool(10))
                .process_read_dir(|_, _, _, _| {})
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    let meta = entry.metadata().ok()?;
                    if meta.is_file() {
                        Some(meta.len())
                    } else {
                        None
                    }
                })
                .collect();

            let file_data_size: u64 = entries.iter().sum();
            let file_count = entries.len() as u64;

            // Calculate overhead for store mode (uncompressed)
            let local_headers = file_count * LOCAL_FILE_HEADER;
            let central_dir_headers = file_count * CENTRAL_DIR_HEADER;

            // Add ZIP64 fields if needed (file > 4GB or archive > 4GB)
            let zip64_overhead = if file_data_size > u32::MAX as u64 {
                file_count * ZIP64_EXTRA_FIELD + 56
            } else {
                0
            };

            // Total = file data + local headers + central directory + end marker + ZIP64 overhead
            file_data_size + local_headers + central_dir_headers + END_OF_CENTRAL_DIR + zip64_overhead
        })
        .await
        .unwrap()
    }
}

#[derive(Debug)]
pub struct FileCursor {
    file: fs::File,
    path: PathBuf,
    position: usize,
    is_eof: bool,
    buffer: BytesMut
}

impl FileCursor {
    pub async fn new(path: PathBuf, buffer_size: usize) -> Result<Self> {
        let mut buffer = BytesMut::with_capacity(buffer_size);
        buffer.resize(buffer_size, 0u8);
        Ok(Self {
            file: fs::File::open(&path).await?,
            path,
            position: 0,
            is_eof: false,
            buffer
        })
    }

    pub fn is_eof(&self) -> bool {
        self.is_eof
    }
}

#[async_trait::async_trait]
impl IOCursor for FileCursor {
    async fn next(&mut self, max_read: Option<u64>) -> Result<Option<&[u8]>> {
        if self.is_eof {
            return Ok(None);
        }

        let read_size = max_read.unwrap_or(self.buffer.len() as u64) as usize;
        let read_size = read_size.min(self.buffer.len());
        let read_buffer = &mut self.buffer.as_mut()[..read_size];
        let bytes_read = self.file.read(read_buffer).await?;
        if bytes_read == 0 {
            self.is_eof = true;
            return Ok(None);
        }

        self.position += bytes_read;
        Ok(Some(&self.buffer[..bytes_read]))
    }

    async fn entry(&self) -> Result<FileEntry> {
        FileEntry::existing(self.path.clone()).await
    }

    fn buffer_size(&self) -> Option<usize> {
        Some(self.buffer.capacity())
    }
}
