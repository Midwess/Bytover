use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use anyhow::Result;
use shared::app::repository::local_resource::LocalResourceRepository;
use shared::core_api::{NetStream, NetStreamEvent, NetStreamInner, UploadRequest, UploadResponse};
use shared::entities::file_system::file::LocalResourcePath;
use std::sync::Arc;
use tokio::{io::{AsyncWriteExt}, spawn, try_join};

pub struct NetStreamImpl {
    pub repository: Arc<dyn LocalResourceRepository>,
}

pub struct NetStreamInnerImpl {
    handle: Option<tokio::task::JoinHandle<Result<()>>>,
    path: LocalResourcePath,
    requests: Vec<UploadRequest>,
    repository: Arc<dyn LocalResourceRepository>,
}

#[async_trait::async_trait]
impl NetStream for NetStreamImpl {
    async fn upload_resource(&self, requests: Vec<UploadRequest>, path: LocalResourcePath) -> Result<Box<dyn NetStreamInner>> {
        Ok(Box::new(NetStreamInnerImpl {
            path,
            handle: None,
            requests,
            repository: self.repository.clone(),
        }))
    }
}

#[async_trait::async_trait]
impl NetStreamInner for NetStreamInnerImpl {
    async fn start(&mut self) -> Result<UnboundedReceiver<NetStreamEvent>> {
        let (tx, rx) = unbounded();
        let mut cursor = self.repository.read(self.path.clone(), 1024 * 1024).await?;

        let requests = self.requests.clone();

        self.handle = Some(spawn(async move {
            let mut responses = Vec::new();
            let mut uploaded = 0u64;

            for req in requests {
                match upload_single(&req, &mut cursor, &tx, &mut uploaded).await {
                    Ok(resp) => {
                        responses.push(resp);
                    }
                    Err(e) => {
                        let _ = tx.unbounded_send(NetStreamEvent::Error(e));
                        return Err(anyhow::anyhow!("Upload failed"));
                    }
                }
            }
            let _ = tx.unbounded_send(NetStreamEvent::Completed(responses));
            Ok(())
        }));

        Ok(rx)
    }

    async fn end(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.await??;
        }
        Ok(())
    }
}

async fn upload_single(
    req: &UploadRequest,
    cursor: &mut Box<dyn core_services::local_storage::stream::IOCursor>,
    tx: &UnboundedSender<NetStreamEvent>,
    uploaded: &mut u64,
) -> Result<UploadResponse> {
    let (mut writer, reader) = tokio::io::duplex(1024 * 512);

    let upload_fut = async {
        let content_length = match req.x_content_length {
            Some(it) => it,
            None => {
                let content = cursor.read_all().await?;
                *uploaded += content.len() as u64;
                let _ = tx.unbounded_send(NetStreamEvent::Progress { uploaded_bytes: *uploaded });
                content.len() as u64
            }
        };

        let client = reqwest::Client::new();
        let resp = client
            .put(req.url.clone())
            .header("Content-Length", content_length)
            .header("Content-Type", "application/octet-stream")
            .body(reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(reader)))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(format!("Upload failed: {}", resp.status())));
        }

        let headers = resp.headers().iter()
            .map(|(k,v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let json = resp.json::<serde_json::Value>().await.ok();

        let response = UploadResponse {
            headers,
            json
        };

        Ok(response)
    };

    let writer_fut = async {
        let Some(content_length) = req.x_content_length else {
            return Ok(())
        };

        let mut written = 0u64;

        while written < content_length {
            // Request next chunk with required max_read
            let max_read = content_length - written;
            let chunk = cursor.next(Some(max_read)).await?.unwrap_or_default();
            if chunk.is_empty() { break; }
            writer.write_all(chunk).await?;
            written += chunk.len() as u64;
            *uploaded += chunk.len() as u64;
            let _ = tx.unbounded_send(NetStreamEvent::Progress { uploaded_bytes: *uploaded });
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
