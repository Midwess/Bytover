use crate::file_system::io::OPFS_WORKER;
use crate::file_system::path_extension::WebExtLocalResourcePath;
use crate::web_worker::bridge::WorkerMessage;
use crate::web_worker::opfs::{FileOperation, OpfsOperation, OpfsOperationOutput};
use anyhow::{anyhow, Result};
use core_services::local_storage::stream::IOCursor;
use core_services::wasm::{Body, HttpClient, XhrEvent};
use futures::channel::mpsc;
use futures_channel::mpsc::Receiver;
use js_sys::Uint8Array;
use n0_future::task::{spawn, JoinHandle};
use n0_future::SinkExt;
use schema::devlog::bitbridge::client_upload_request::Upload;
use schema::devlog::bitbridge::{MultiPartUpload, MultiPartUploadComplete};
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::core_api::{NetStream, NetStreamEvent, NetStreamInner};
use shared::entities::file_system::file::LocalResourcePath;
use shared::rpc::cloud_server::CloudServer;
use std::sync::Arc;
use bytes::BytesMut;
use tonic_web_wasm_client::Client;
use web_sys::Blob;

const EVENT_QUEUE_SIZE: usize = 8;

pub struct NetStreamImpl {
    pub resource_repo: Arc<dyn LocalResourceRepository>,
    pub server: &'static CloudServer<Client>
}

// Only working with Blob from browser
pub struct NetStreamInnerBlobImpl {
    upload: Upload,
    path: LocalResourcePath,
    server: &'static CloudServer<Client>,
    handle: Option<JoinHandle<()>>
}

// Support working with IOCursor from core-service
pub struct NetStreamInnerChunkStreamImpl {
    server: &'static CloudServer<Client>,
    resource_repo: Arc<dyn LocalResourceRepository>,
    upload: Upload,
    path: LocalResourcePath,
    handle: Option<JoinHandle<()>>
}

#[async_trait::async_trait(?Send)]
impl NetStream for NetStreamImpl {
    async fn upload_resource(&self, upload: Upload, path: LocalResourcePath) -> Result<Box<dyn NetStreamInner>> {
        let chunk_stream_enabled = match &upload {
            Upload::Multipart(upload_info) => upload_info.chunk_stream_enabled,
            _ => false
        };

        if chunk_stream_enabled {
            log::info!("Using chunk stream for upload");
            return Ok(Box::new(NetStreamInnerChunkStreamImpl {
                resource_repo: self.resource_repo.clone(),
                server: self.server,
                upload,
                path,
                handle: None
            }))
        }

        Ok(Box::new(NetStreamInnerBlobImpl {
            server: self.server,
            upload,
            path,
            handle: None
        }))
    }
}

#[async_trait::async_trait(?Send)]
impl NetStreamInner for NetStreamInnerBlobImpl {
    async fn start(&mut self) -> Result<Receiver<NetStreamEvent>> {
        let (mut tx, rx) = mpsc::channel::<NetStreamEvent>(EVENT_QUEUE_SIZE);
        let Some(blob) = self.get_blob().await else {
            return Err(anyhow!("No blob to upload"));
        };

        let upload = self.upload.clone();

        let server = self.server;
        self.handle = Some(spawn(async move {
            let result = match upload {
                Upload::SingleUrl(url) => Self::single_upload(&url, blob, &mut tx).await,
                Upload::Multipart(upload_info) => Self::multipart_upload(upload_info, blob, &mut tx, server).await
            };

            let event = match result {
                Ok(completion) => NetStreamEvent::Completed(completion),
                Err(e) => NetStreamEvent::Error(anyhow!("Upload failed: {:?}", e))
            };

            let _ = tx.send(event).await;
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

impl NetStreamInnerBlobImpl {
    async fn single_upload(url: &str, blob: Blob, tx: &mut mpsc::Sender<NetStreamEvent>) -> Result<Option<MultiPartUploadComplete>> {
        upload(url.to_owned(), Body::Blob(blob), 0, tx.clone()).await?;
        Ok(None)
    }

    async fn multipart_upload(
        mut request: MultiPartUpload,
        blob: Blob,
        tx: &mut mpsc::Sender<NetStreamEvent>,
        server: &'static CloudServer<Client>
    ) -> Result<Option<MultiPartUploadComplete>> {
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
            let Some(etag) = upload(request.upload_url.clone(), Body::Blob(chunk_blob), uploaded, tx.clone()).await? else {
                return Err(anyhow!("Failed to upload chunk, missing etag"));
            };

            let Some(next_request) = server.complete_part_upload(&request.context_token).await? else {
                break;
            };

            request = next_request;
            completion.e_tags.push(etag);
            uploaded = end_position;
        }

        Ok(Some(completion))
    }

    async fn get_blob(&self) -> Option<Blob> {
        let Some(opfs_path) = self.path.opfs_path() else {
            return None;
        };

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
}

#[async_trait::async_trait(?Send)]
impl NetStreamInner for NetStreamInnerChunkStreamImpl {
    async fn start(&mut self) -> anyhow::Result<Receiver<NetStreamEvent>> {
        let (mut tx, rx) = mpsc::channel::<NetStreamEvent>(EVENT_QUEUE_SIZE);
        let Upload::Multipart(multipart) = self.upload.clone() else {
            return Err(anyhow!("Only multipart upload is supported"));
        };

        let mut cursor = self.resource_repo.read(self.path.clone(), multipart.x_content_length as usize).await?;
        let server = self.server;
        self.handle = Some(spawn(async move {
            let result = Self::multipart_upload(multipart, &mut cursor, server, &mut tx).await;

            let event = match result {
                Ok(completion) => NetStreamEvent::Completed(Some(completion)),
                Err(e) => NetStreamEvent::Error(anyhow!("Upload failed: {:?}", e))
            };

            let _ = tx.try_send(event);
            let _ = tx.close();
        }));

        Ok(rx)
    }

    async fn end(&mut self) -> anyhow::Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        Ok(())
    }
}

impl NetStreamInnerChunkStreamImpl {
    async fn multipart_upload(
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
        let mut fut = None;
        loop {
            bytes.resize(request.x_content_length as usize, 0);
            let content_length = cursor.read_exact(&mut bytes).await?;
            if content_length == 0 {
                break;
            }

            let u8_array = unsafe { Uint8Array::view(&bytes[..content_length]) };

            if let Some(fut) = fut.take() {
                let Some(etag) = fut.await? else {
                    return Err(anyhow!("Failed to upload chunk, missing etag"));
                };

                completion.e_tags.push(etag);
            }

            let url = request.upload_url.clone();
            fut.replace(upload(url, Body::Uint8Array(u8_array), uploaded, tx.clone()));
            uploaded += content_length as u64;

            let Some(next_request) = server.complete_part_upload(&request.context_token).await? else {
                break;
            };

            request = next_request;
        }

        Ok(completion)
    }

    async fn end(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        Ok(())
    }
}


async fn upload(url: String, body: Body, uploaded: u64, mut tx: mpsc::Sender<NetStreamEvent>) -> Result<Option<String>> {
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
                return Ok(etag)
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
