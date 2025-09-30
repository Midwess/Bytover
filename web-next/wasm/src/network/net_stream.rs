use crate::file_system::io::OPFS_WORKER;
use crate::file_system::path_extension::WebExtLocalResourcePath;
use crate::web_worker::bridge::WorkerMessage;
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput};
use anyhow::{anyhow, Result};
use bytes::BytesMut;
use core_services::local_storage::stream::IOCursor;
use core_services::wasm::{Body, HttpClient, XhrEvent};
use futures::channel::mpsc;
use futures_channel::mpsc::Receiver;
use n0_future::task::{spawn, JoinHandle};
use n0_future::SinkExt;
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::{MultiPartUpload, MultiPartUploadComplete};
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::core_api::{NetStream, NetStreamEvent, NetStreamInner};
use shared::entities::file_system::file::LocalResourcePath;
use shared::rpc::cloud_server::CloudServer;
use std::sync::Arc;
use tonic_web_wasm_client::Client;
use web_sys::Blob;

const EVENT_QUEUE_SIZE: usize = 8;

pub struct NetStreamImpl {
    pub resource_repo: Arc<dyn LocalResourceRepository>,
    pub server: &'static CloudServer<Client>
}

pub struct NetStreamInnerBlobImpl {
    upload: Upload,
    path: LocalResourcePath,
    server: &'static CloudServer<Client>,
    handle: Option<JoinHandle<Result<()>>>
}

pub struct NetStreamInnerChunkStreamImpl {
    upload: Upload,
    path: LocalResourcePath,
    server: &'static CloudServer<Client>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    handle: Option<JoinHandle<()>>
}

#[async_trait::async_trait(?Send)]
impl NetStream for NetStreamImpl {
    async fn upload_resource(&self, upload: Upload, path: LocalResourcePath) -> Result<Box<dyn NetStreamInner>> {
        let chunk_stream_enabled = matches!(&upload, Upload::Multipart(info) if info.chunk_stream_enabled);

        if chunk_stream_enabled {
            log::info!("Using chunk stream for upload");
            Ok(Box::new(NetStreamInnerChunkStreamImpl {
                resource_repo: self.resource_repo.clone(),
                server: self.server,
                upload,
                path,
                handle: None
            }))
        } else {
            Ok(Box::new(NetStreamInnerBlobImpl {
                server: self.server,
                upload,
                path,
                handle: None
            }))
        }
    }
}

#[async_trait::async_trait(?Send)]
impl NetStreamInner for NetStreamInnerBlobImpl {
    async fn start(&mut self) -> Result<Receiver<NetStreamEvent>> {
        let (mut tx, rx) = mpsc::channel::<NetStreamEvent>(EVENT_QUEUE_SIZE);

        let blob = get_blob_from_path(&self.path).await.ok_or_else(|| anyhow!("No blob to upload"))?;

        let upload = self.upload.clone();
        let server = self.server;

        self.handle = Some(spawn(async move {
            let result = match upload {
                Upload::SingleUrl(url) => {
                    upload_single(&url, Body::Blob(blob), &mut tx).await?;
                    Ok::<_, anyhow::Error>(None)
                }
                Upload::Multipart(upload_info) => {
                    let completion = upload_multipart_blob(upload_info, blob, &mut tx, server).await?;
                    Ok(Some(completion))
                }
            };

            let event = match result {
                Ok(completion) => NetStreamEvent::Completed(completion),
                Err(e) => NetStreamEvent::Error(anyhow!("Upload failed: {:?}", e))
            };

            let _ = tx.send(event).await;
            let _ = tx.close();

            Ok(())
        }));

        Ok(rx)
    }

    async fn end(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        Ok(())
    }
}

#[async_trait::async_trait(?Send)]
impl NetStreamInner for NetStreamInnerChunkStreamImpl {
    async fn start(&mut self) -> Result<Receiver<NetStreamEvent>> {
        let (mut tx, rx) = mpsc::channel::<NetStreamEvent>(EVENT_QUEUE_SIZE);

        let Upload::Multipart(multipart) = self.upload.clone() else {
            return Err(anyhow!("Only multipart upload is supported"));
        };

        let mut cursor = self.resource_repo.read(
            self.path.clone(),
            512 * 1024
        ).await?;
        let server = self.server;

        self.handle = Some(spawn(async move {
            let result = upload_multipart_stream(multipart, &mut cursor, server, &mut tx).await;

            let event = match result {
                Ok(completion) => NetStreamEvent::Completed(Some(completion)),
                Err(e) => NetStreamEvent::Error(anyhow!("Upload failed: {:?}", e))
            };

            let _ = tx.try_send(event);
            let _ = tx.close();
        }));

        Ok(rx)
    }

    async fn end(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        Ok(())
    }
}

// Helper functions
async fn get_blob_from_path(path: &LocalResourcePath) -> Option<Blob> {
    let opfs_path = path.opfs_path()?;

    let resp = OPFS_WORKER
        .send(WorkerMessage::new(OpfsOperation {
            file_path: opfs_path,
            operation: FileOperation::Blob
        }))
        .await?;

    match resp.message {
        OpfsOperationOutput::Blob(blob) => Some(blob),
        _ => None
    }
}

async fn upload_single(url: &str, body: Body, tx: &mut mpsc::Sender<NetStreamEvent>) -> Result<()> {
    upload_with_progress(url.to_owned(), body, 0, tx.clone()).await?;
    Ok(())
}

async fn upload_multipart_blob(
    mut request: MultiPartUpload,
    blob: Blob,
    tx: &mut mpsc::Sender<NetStreamEvent>,
    server: &'static CloudServer<Client>
) -> Result<MultiPartUploadComplete> {
    let mut completion = MultiPartUploadComplete {
        e_tags: vec![],
        context_token: request.context_token.clone()
    };

    let mut uploaded = 0u64;
    let total_size = blob.size() as u64;

    while uploaded < total_size {
        let chunk_size = (total_size - uploaded).min(request.x_content_length as u64);
        let end_position = uploaded + chunk_size;

        let chunk_blob = blob
            .slice_with_i32_and_i32(uploaded as i32, end_position as i32)
            .map_err(|e| anyhow!("Failed to slice blob: {:?}", e))?;

        let etag = upload_with_progress(request.upload_url.clone(), Body::Blob(chunk_blob), uploaded, tx.clone())
            .await?
            .ok_or_else(|| anyhow!("Failed to upload chunk, missing etag"))?;

        completion.e_tags.push(etag);
        uploaded = end_position;

        let Some(next_request) = server.complete_part_upload(&request.context_token).await? else {
            break;
        };
        request = next_request;
    }

    Ok(completion)
}

async fn upload_multipart_stream(
    mut request: MultiPartUpload,
    cursor: &mut Box<dyn IOCursor>,
    server: &'static CloudServer<Client>,
    tx: &mut mpsc::Sender<NetStreamEvent>
) -> Result<MultiPartUploadComplete> {
    let mut uploaded = 0u64;
    let mut completion = MultiPartUploadComplete {
        e_tags: vec![],
        context_token: request.context_token.clone()
    };

    let mut bytes = BytesMut::with_capacity(request.x_content_length as usize);
    let mut pending_upload: Option<JoinHandle<Result<Option<String>>>> = None;

    loop {
        bytes.resize(request.x_content_length as usize, 0);
        let content_length = cursor.read_exact(&mut bytes).await?;
        if content_length == 0 {
            break;
        }

        // Wait for previous upload to complete
        if let Some(fut) = pending_upload.take() {
            let etag = fut.await.unwrap()?.ok_or_else(|| anyhow!("Failed to upload chunk, missing etag"))?;
            completion.e_tags.push(etag);
        }

        // Start new upload
        let upload_data = bytes[..content_length].to_vec();
        pending_upload = Some({
            let url = request.upload_url.clone();
            let tx_clone = tx.clone();
            spawn(async move { upload_with_progress(url, Body::Bytes(upload_data), uploaded, tx_clone).await })
        });

        uploaded += content_length as u64;

        let Some(next_request) = server.complete_part_upload(&request.context_token).await? else {
            break;
        };
        request = next_request;
    }

    // Wait for final upload
    if let Some(fut) = pending_upload {
        let etag = fut.await.unwrap()?.ok_or_else(|| anyhow!("Failed to upload chunk, missing etag"))?;
        completion.e_tags.push(etag);
    }

    Ok(completion)
}

async fn upload_with_progress(url: String, body: Body, uploaded: u64, mut tx: mpsc::Sender<NetStreamEvent>) -> Result<Option<String>> {
    let mut request = HttpClient::new()
        .url(&url)
        .header("content-type", "application/octet-stream")
        .method("PUT")
        .body(body)
        .xhr()
        .map_err(|e| anyhow!("Failed to upload: {:?}", e))?;

    while let Some(event) = request.next_event().await {
        match event {
            XhrEvent::Error(value) => {
                return Err(anyhow!("Failed to upload: {:?}", value));
            }
            XhrEvent::Complete { headers, .. } => {
                let etag = headers.get("etag").map(|tag| tag.trim_matches('"').to_string());
                return Ok(etag);
            }
            XhrEvent::InProgress(value) => {
                let uploaded_bytes = value.loaded() as u64 + uploaded;
                let _ = tx.try_send(NetStreamEvent::Progress { uploaded_bytes });
            }
        }
    }

    Ok(None)
}

impl Drop for NetStreamInnerBlobImpl {
    fn drop(&mut self) {
        let _ = futures::executor::block_on(self.end());
    }
}

impl Drop for NetStreamInnerChunkStreamImpl {
    fn drop(&mut self) {
        let _ = futures::executor::block_on(self.end());
    }
}
