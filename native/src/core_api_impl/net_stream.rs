use std::sync::Arc;
use anyhow::anyhow;
use futures_util::future::join_all;
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use shared::core_api::{NetStream, NetStreamEvent, NetStreamInner};
use shared::core_transfer_protocol::public_cloud::cloud_service::CloudTransferErrors;
use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};
use tokio::task::JoinHandle;
use tokio_util::io::ReaderStream;
use url::Url;
use shared::app::file_system::file::LocalResourcePath;
use shared::app::repository::local_resource::LocalResourceRepository;

pub struct NetStreamImpl {
    pub repository: Arc<dyn LocalResourceRepository>
}

pub struct NetStreamInnerImpl {
    handle: Option<JoinHandle<Result<(), CloudTransferErrors>>>,
    path: LocalResourcePath,
    size: u64,
    url: Url,
    repository: Arc<dyn LocalResourceRepository>
}

#[async_trait::async_trait]
impl NetStream for NetStreamImpl {
    async fn upload_resource(&self, http_url: Url, path: LocalResourcePath, size: u64) -> anyhow::Result<Box<dyn NetStreamInner>> {
        Ok(Box::new(NetStreamInnerImpl {
            path,
            handle: None,
            size,
            url: http_url,
            repository: self.repository.clone()
        }))
    }
}

#[async_trait::async_trait]
impl NetStreamInner for NetStreamInnerImpl {
    async fn start(
        &mut self,
    ) -> anyhow::Result<UnboundedReceiver<NetStreamEvent>> {
        let size = self.size;
        let (nt, nx) = unbounded();
        let (mut writer, reader) = duplex(1024 * 512);
        let stream = ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);
        let mut cursor = self.repository.read(self.path.clone(), 1024 * 1024).await?;

        let upload_url_cloned = self.url.clone();
        let upload_handle: JoinHandle<Result<(), CloudTransferErrors>> = {
            let nt = nt.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let response = client
                    .put(upload_url_cloned.to_string())
                    .header("Content-Length", format!("{size}"))
                    .header("Content-Type", "application/octet-stream")
                    .body(body)
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
                if response.status().is_success() {
                    let _ = nt.unbounded_send(NetStreamEvent::Completed);
                    return Ok(())
                }

                let msg = format!(
                    "STATUS {:?}, msg: {:?}",
                    response.status(),
                    response.text().await
                );

                let _ = nt.unbounded_send(NetStreamEvent::Error(anyhow!(msg.clone())));

                Err(CloudTransferErrors::UploadProcessError(msg))
            })
        };

        let reader_handle: JoinHandle<Result<(), CloudTransferErrors>> = {
            let nt = nt.clone();
            tokio::spawn(async move {
                let mut written_bytes = 0;
                while let Ok(Some(chunk)) = cursor.next().await {
                    if let Err(e) = writer.write_all(&chunk).await {
                        let _ = nt.unbounded_send(NetStreamEvent::Error(anyhow!("Error while wring chunk to stream {e:?}")));
                        return Err(CloudTransferErrors::UploadProcessError(format!("Error writing chunk to stream: {:?}", e.to_string())))
                    }

                    written_bytes += chunk.len();

                    let _ = nt.unbounded_send(NetStreamEvent::Progress {
                        uploaded_bytes: written_bytes as u64,
                    });
                }

                writer.shutdown().await.map_err(|e| anyhow::anyhow!(e))?;
                Ok(())
            })
        };

        let handle = tokio::spawn(async move {
            let result = join_all(vec![upload_handle, reader_handle]).await;

            for r in result {
                if let Err(e) = r {
                    return Err(anyhow!(e).into())
                }

                if let Ok(Err(e)) = r {
                    return Err(e)
                }
            }

            return Ok(())
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

impl Drop for NetStreamInnerImpl {
    fn drop(&mut self) {
        let _ = self.end();
    }
}
