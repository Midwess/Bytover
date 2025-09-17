use bytes::{Bytes, BytesMut};
use core_services::local_storage::stream::IOCursor;
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::channel::mpsc;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::core_api::{NetStream, NetStreamEvent, NetStreamInner, UploadRequest, UploadResponse};
use shared::core_transfer_protocol::public_cloud::cloud_service::CloudTransferErrors;
use shared::entities::file_system::file::LocalResourcePath;
use tokio::try_join;
use std::sync::Arc;
use tokio::io::{duplex, AsyncWriteExt};
use tokio::task::JoinHandle;
use tokio_util::io::ReaderStream;

pub struct NetStreamImpl {
    pub repository: Arc<dyn LocalResourceRepository>
}

pub struct NetStreamInnerImpl {
    handle: Option<JoinHandle<Result<(), CloudTransferErrors>>>,
    path: LocalResourcePath,
    requests: Vec<UploadRequest>,
    repository: Arc<dyn LocalResourceRepository>,
    current_request_index: usize,
}

#[async_trait::async_trait]
impl NetStream for NetStreamImpl {
    async fn upload_resource(&self, requests: Vec<UploadRequest>, path: LocalResourcePath) -> anyhow::Result<Box<dyn NetStreamInner>> {
        Ok(Box::new(NetStreamInnerImpl {
            path,
            handle: None,
            requests,
            repository: self.repository.clone(),
            current_request_index: 0,
        }))
    }
}

#[async_trait::async_trait]
impl NetStreamInner for NetStreamInnerImpl {
    async fn start(&mut self) -> anyhow::Result<UnboundedReceiver<NetStreamEvent>> {
        let (nt, nx) = unbounded();
        let mut cursor = self.repository.read(self.path.clone(), 1024 * 1024).await?;
        let requests = self.requests.clone();
        
        let mut bytes = Bytes::new();
        let handle = tokio::spawn(async move {
            let mut responses = Vec::new();
            
            for request in requests {
                match Self::upload_single_request(&request, bytes, &mut cursor, &nt).await {
                    Ok((response, data_left)) => {
                        responses.push(response);
                        bytes = data_left;
                    },
                    Err(e) => {
                        let _ = nt.unbounded_send(NetStreamEvent::Error(e));
                        return Err(CloudTransferErrors::UploadProcessError("Upload failed".to_string()));
                    }
                }
            }
            
            let _ = nt.unbounded_send(NetStreamEvent::Completed(responses));
            Ok(())
        });

        self.handle = Some(handle);
        Ok(nx)
    }

    async fn end(&mut self) -> anyhow::Result<()> {
        let Some(handle) = self.handle.take() else {
            return Ok(());
        };

        let Ok(result) = handle.await else { return Ok(()) };

        result?;

        Ok(())
    }
}

impl NetStreamInnerImpl {
    async fn upload_single_request(
        request: &UploadRequest,
        bytes: Bytes,
        cursor: &mut Box<dyn IOCursor>,
        nt: &mpsc::UnboundedSender<NetStreamEvent>,
    ) -> anyhow::Result<(UploadResponse, Bytes)> {
        let required_bytes = request.x_content_length;
        let (mut writer, reader) = duplex(1024 * 512);
        let stream = ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);
        
        let client = reqwest::Client::new();
        let upload_future = async move {
            let response = client
                .put(request.url.clone())
                .header("Content-Length", request.x_content_length.to_string())
                .header("Content-Type", "application/octet-stream")
                .body(body)
                .send()
                .await;

            match response {
                Ok(response) => Ok(response),
                Err(e) => Err(anyhow::anyhow!("Failed to send upload request: {:?}", e))
            }
        };

        let mut bytes_written = 0u64;
        let writer_task = async {
            let mut chunk = BytesMut::from(bytes);
            let data_left = loop {
                let Some(next_bytes) = cursor.next().await.map_err(|e| anyhow::anyhow!(e))? else {
                    break Bytes::new();
                };

                chunk.extend(next_bytes);
                
                let remaining = required_bytes - bytes_written;
                let write_size = (chunk.len() as usize).min(remaining as usize);
                writer.write_all(&chunk.split_to(write_size)).await?;
                bytes_written += write_size as u64;
                let _ = nt.unbounded_send(NetStreamEvent::Progress {
                    uploaded_bytes: bytes_written
                });

                if bytes_written >= required_bytes {
                    break Bytes::from(chunk);
                }
            };
            
            writer.shutdown().await?;
            Ok::<Bytes, anyhow::Error>(data_left)
        };

        let Ok((upload_result, data_left)) = try_join!(upload_future, writer_task) else {
            return Err(anyhow::anyhow!("Panic occur during upload"));
        };

        if !upload_result.status().is_success() {
            let msg = format!("Upload failed: {:?}", upload_result.status());
            return Err(anyhow::anyhow!(msg));
        }
        
        let headers = upload_result.headers().iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
            
        let json = upload_result.json::<serde_json::Value>().await.ok();
        
        Ok((UploadResponse { headers, json }, data_left.into()))
    }
}

impl Drop for NetStreamInnerImpl {
    fn drop(&mut self) {
        let _ = self.end();
    }
}
