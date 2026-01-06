use anyhow::{anyhow, Result};
use core_services::local_storage::file_system::FileCursor;
use core_services::local_storage::stream::IOCursor;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures_util::SinkExt;
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::{MultiPartUpload, MultiPartUploadComplete};
use shared::entities::local_resource::LocalResourcePath;
use shared::protocol::rpc::cloud_server::CloudServer;
use shared::repository::local_resource::LocalResourceRepository;
use shared::shell::api::{NetStream, NetStreamEvent, NetStreamInner};
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::io::{AsyncWriteExt, DuplexStream};
use tokio::task::JoinHandle;
use tokio::{io, spawn, try_join};
use tokio_util::io::ReaderStream;
use tonic::transport::Channel;

const EVENT_QUEUE_SIZE: usize = 8;
const READ_CHUNK_SIZE: usize = 256 * 1024;
const MB: u64 = 1024 * 1024;

pub struct NetStreamImpl {
    pub repository: Arc<dyn LocalResourceRepository>,
    pub server: &'static CloudServer<Channel>
}

pub struct NetStreamInnerImpl {
    handle: Option<JoinHandle<()>>,
    path: LocalResourcePath,
    upload: Upload,
    server: &'static CloudServer<Channel>,
    repository: Arc<dyn LocalResourceRepository>
}

#[async_trait::async_trait]
impl NetStream for NetStreamImpl {
    async fn upload_resource(&self, upload: Upload, path: LocalResourcePath) -> Result<Box<dyn NetStreamInner>> {
        Ok(Box::new(NetStreamInnerImpl {
            path,
            server: self.server,
            handle: None,
            upload,
            repository: self.repository.clone()
        }))
    }
}

#[async_trait::async_trait]
impl NetStreamInner for NetStreamInnerImpl {
    async fn start(&mut self) -> Result<Receiver<NetStreamEvent>> {
        let (tx, rx) = channel(EVENT_QUEUE_SIZE);
        let cursor = self.repository.read(self.path.clone(), READ_CHUNK_SIZE, false).await?;
        let upload = self.upload.clone();

        let handle = spawn(Self::upload_task(cursor, upload, self.server, tx));
        self.handle = Some(handle);

        Ok(rx)
    }

    async fn end(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        Ok(())
    }
}

impl NetStreamInnerImpl {
    async fn upload_task(
        mut cursor: Box<dyn IOCursor>,
        upload: Upload,
        server: &'static CloudServer<Channel>,
        mut tx: Sender<NetStreamEvent>
    ) {
        let result = match upload {
            Upload::SingleUrl(url) => Self::single_upload(&url, &mut cursor, &mut tx).await,
            Upload::Multipart(upload_info) => Self::multipart_upload(upload_info, &mut cursor, &mut tx, server).await
        };

        let event = match result {
            Ok(completion) => NetStreamEvent::Completed(completion),
            Err(e) => NetStreamEvent::Error(e)
        };

        let _ = tx.send(event).await;
    }

    async fn single_upload(
        url: &str,
        cursor: &mut Box<dyn IOCursor>,
        tx: &mut Sender<NetStreamEvent>
    ) -> Result<Option<MultiPartUploadComplete>> {
        let chunk_size = cursor.entry().await?.size.min(1024 * 1024 * 1024 * 5);
        Self::upload_chunk(url, cursor, tx, &mut 0, chunk_size).await?;
        Ok(None)
    }

    async fn multipart_upload(
        mut upload: MultiPartUpload,
        cursor: &mut Box<dyn IOCursor>,
        tx: &mut Sender<NetStreamEvent>,
        server: &CloudServer<Channel>
    ) -> Result<Option<MultiPartUploadComplete>> {
        let mut completion = MultiPartUploadComplete {
            e_tags: vec![],
            context_token: upload.context_token.clone()
        };

        let mut uploaded = 0u64;
        let total_size = cursor.entry().await?.size;
        log::info!("Total size of resource is {} bytes; content_length from server {}", total_size, upload.x_content_length);

        // This is the main chunk size, every chunk must be equal in size except the last one
        let main_chunk_size = upload.x_content_length;

        // Upload all main chunks
        while uploaded < total_size && !upload.is_last {
            let etag = match Self::upload_chunk(&upload.upload_url, cursor, tx, &mut uploaded, upload.x_content_length as u64).await {
                Ok(etag) => etag,
                Err(e) => {
                    log::error!("Failed to upload: {upload:?}");
                    return Err(e);
                }
            };
            completion.e_tags.push(etag);
            upload = match server.complete_part_upload(&upload.context_token).await? {
                Some(mut continue_upload) => {
                    let remaining = total_size - uploaded.min(total_size);
                    continue_upload.x_content_length = continue_upload.x_content_length.min(remaining as u32);
                    continue_upload.is_last = continue_upload.is_last || continue_upload.x_content_length != main_chunk_size;
                    continue_upload
                }
                None => {
                    return Ok(Some(completion));
                }
            };
        }

        // When we reach this point, it means that the file size has been incorrectly calculated
        // we need to dump all remaining data to a temporary file and upload it in chunks
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_path_buf();
        let mut temp_writer = tokio::fs::File::create(&temp_path).await?;

        let mut written_bytes = 0u64;
        loop {
            let Some(data) = cursor.next(None).await? else { break };
            temp_writer.write_all(data).await?;
            written_bytes += data.len() as u64;
        }
        temp_writer.flush().await?;
        drop(temp_writer);

        if written_bytes == 0 {
            log::info!("No remaining data to upload");
            return Ok(Some(completion));
        }

        log::info!("Wrote {} bytes to temporary file, preparing to upload in chunks", written_bytes);

        let mut temp_cursor: Box<dyn IOCursor> = Box::new(FileCursor::new(temp_path, READ_CHUNK_SIZE).await?);

        let mut remaining = written_bytes;
        while remaining > 0 {
            let chunk_size = remaining.min(main_chunk_size as u64);

            let etag = Self::upload_chunk(&upload.upload_url, &mut temp_cursor, tx, &mut uploaded, chunk_size).await?;
            completion.e_tags.push(etag);

            remaining -= chunk_size;

            if remaining > 0 {
                upload = match server.complete_part_upload(&upload.context_token).await? {
                    Some(continue_upload) => continue_upload,
                    None => {
                        log::warn!(
                            "Server did not provide continuation upload but we still have {} bytes remaining",
                            remaining
                        );
                        break;
                    }
                };
            }
        }

        log::info!("Resource upload completed {:?}", completion);
        Ok(Some(completion))
    }

    async fn upload_chunk(
        url: &str,
        cursor: &mut Box<dyn IOCursor>,
        tx: &mut Sender<NetStreamEvent>,
        uploaded: &mut u64,
        chunk_size: u64
    ) -> Result<String> {
        log::info!("Uploading chunk of size {} bytes", chunk_size);

        let (mut writer, reader) = io::duplex(5 * MB as usize);

        let upload_task = Self::perform_upload(url, reqwest::Body::wrap_stream(ReaderStream::new(reader)), chunk_size);
        let write_task = Self::write_data(&mut writer, cursor, tx, chunk_size, uploaded);

        let (etag, _) = try_join!(upload_task, write_task)?;
        Ok(etag)
    }

    async fn perform_upload(url: &str, body: impl Into<reqwest::Body>, content_length: u64) -> Result<String> {
        let client = reqwest::Client::new();
        let response = client
            .put(url)
            .header("Content-Length", content_length)
            .header("Content-Type", "application/octet-stream")
            .body(body)
            .send()
            .await?
            .error_for_status()?;

        let etag = response
            .headers()
            .get("etag")
            .ok_or_else(|| anyhow!("Missing etag in response"))?
            .to_str()
            .map_err(|_| anyhow!("Invalid ETag header"))?
            .trim_matches('"')
            .to_string();

        Ok(etag)
    }

    async fn write_data(
        writer: &mut DuplexStream,
        cursor: &mut Box<dyn IOCursor>,
        tx: &mut Sender<NetStreamEvent>,
        chunk_size: u64,
        total_uploaded: &mut u64
    ) -> Result<()> {
        let mut remaining = chunk_size;
        let _ = tx.try_send(NetStreamEvent::Progress {
            uploaded_bytes: *total_uploaded
        });

        let mut _uploaded_amount = 0;
        while remaining > 0 {
            let Some(data) = cursor.next(Some(remaining)).await? else { break };
            let data_len = data.len() as u64;

            writer.write_all(data).await?;
            remaining -= data_len;
            *total_uploaded += data_len;
            _uploaded_amount += data_len;

            let _ = tx.try_send(NetStreamEvent::Progress {
                uploaded_bytes: *total_uploaded
            });

            writer.flush().await?;
        }

        writer.shutdown().await?;
        Ok(())
    }
}

impl Drop for NetStreamInnerImpl {
    fn drop(&mut self) {
        let _ = futures::executor::block_on(self.end());
    }
}
