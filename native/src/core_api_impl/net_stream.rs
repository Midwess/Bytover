use anyhow::Result;
use futures::channel::mpsc::{channel, Receiver, Sender};
use futures_util::SinkExt;
use reqwest::Response;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::core_api::{NetStream, NetStreamEvent, NetStreamInner, UploadRequest, UploadResponse};
use shared::entities::file_system::file::LocalResourcePath;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::{spawn, try_join};

pub struct NetStreamImpl {
    pub repository: Arc<dyn LocalResourceRepository>
}

pub struct NetStreamInnerImpl {
    handle: Option<tokio::task::JoinHandle<Result<()>>>,
    path: LocalResourcePath,
    requests: Vec<UploadRequest>,
    repository: Arc<dyn LocalResourceRepository>
}

#[async_trait::async_trait]
impl NetStream for NetStreamImpl {
    async fn upload_resource(&self, requests: Vec<UploadRequest>, path: LocalResourcePath) -> Result<Box<dyn NetStreamInner>> {
        Ok(Box::new(NetStreamInnerImpl {
            path,
            handle: None,
            requests,
            repository: self.repository.clone()
        }))
    }
}

#[async_trait::async_trait]
impl NetStreamInner for NetStreamInnerImpl {
    async fn start(&mut self) -> Result<Receiver<NetStreamEvent>> {
        let (mut tx, rx) = channel(32);
        let mut cursor = self.repository.read(self.path.clone(), 1024 * 1024).await?;
        let requests = self.requests.clone();

        log::info!("Start uploading resource, request from upstream {:?}", requests);
        self.handle = Some(spawn(async move {
            let mut responses = Vec::new();
            let mut uploaded = 0u64;

            for req in requests {
                let task = async {
                    let response = match req.x_content_length {
                        Some(_) => stream(&req, &mut cursor, &mut tx, &mut uploaded).await?,
                        None => {
                            let bytes = cursor.read_all().await?;
                            let content_length = bytes.len() as u64;
                            log::info!("On memory uploading {} bytes", bytes.len());
                            if bytes.is_empty() {
                                return Ok(())
                            }

                            let client = reqwest::Client::new();
                            let resp = client
                                .put(req.url.clone())
                                .header("Content-Length", content_length)
                                .header("Content-Type", "application/octet-stream")
                                .body(bytes)
                                .send()
                                .await?
                                .error_for_status()?;

                            build_response(resp).await
                        }
                    };

                    responses.push(response);
                    Result::<(), anyhow::Error>::Ok(())
                }
                .await;

                if let Err(e) = task {
                    let _ = tx.send(NetStreamEvent::Error(e)).await;
                }
            }

            let _ = tx.try_send(NetStreamEvent::Progress { uploaded_bytes: uploaded });
            let _ = tx.send(NetStreamEvent::Completed(responses)).await;
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

async fn stream(
    req: &UploadRequest,
    cursor: &mut Box<dyn core_services::local_storage::stream::IOCursor>,
    tx: &mut Sender<NetStreamEvent>,
    uploaded: &mut u64
) -> Result<UploadResponse> {
    let Some(content_length) = req.x_content_length else {
        return Err(anyhow::anyhow!("Upload stream must have a content length"));
    };

    let (mut writer, reader) = tokio::io::duplex(1024 * 512);

    let upload_fut = async {
        let client = reqwest::Client::new();
        let resp = client
            .put(req.url.clone())
            .header("Content-Length", content_length)
            .header("Content-Type", "application/octet-stream")
            .body(reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(reader)))
            .send()
            .await?
            .error_for_status()?;

        let response = build_response(resp).await;
        Result::<UploadResponse, anyhow::Error>::Ok(response)
    };

    let writer_fut = async {
        let Some(mut remaining) = req.x_content_length else { return Ok(()) };

        while remaining > 0 {
            let chunk = cursor.next(Some(remaining)).await?.unwrap_or_default();
            remaining -= chunk.len() as u64;
            writer.write_all(chunk).await?;
            *uploaded += chunk.len() as u64;
            if let Err(e) = tx.try_send(NetStreamEvent::Progress { uploaded_bytes: *uploaded }) {
                log::error!("Failed to send progress event: {:?}", e);
            }
        }

        writer.shutdown().await?;
        Ok(())
    };

    let (resp, _) = try_join!(upload_fut, writer_fut)?;
    Ok(resp)
}

impl Drop for NetStreamInnerImpl {
    fn drop(&mut self) {
        let _ = futures::executor::block_on(self.end());
    }
}

async fn build_response(value: Response) -> UploadResponse {
    UploadResponse {
        headers: value.headers().iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string())).collect(),
        body: value.json::<serde_json::Value>().await.ok()
    }
}
