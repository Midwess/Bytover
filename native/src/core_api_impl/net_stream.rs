use shared::core_api::{NetStream, NetStreamInner};
use shared::core_transfer_protocol::public_cloud::cloud_service::CloudTransferErrors;
use tokio::io::{duplex, AsyncWriteExt};
use tokio::task::JoinHandle;
use tokio_util::io::ReaderStream;
use url::Url;

pub struct NetStreamImpl {}

pub struct NetStreamInnerImpl {
    handle: Option<JoinHandle<Result<(), CloudTransferErrors>>>,
    writer: tokio::io::DuplexStream
}

#[async_trait::async_trait]
impl NetStream for NetStreamImpl {
    async fn start(&self, http_url: Url, size: u64) -> anyhow::Result<Box<dyn NetStreamInner>> {
        let (writer, reader) = duplex(1024 * 512);
        let stream = ReaderStream::new(reader);
        let body = reqwest::Body::wrap_stream(stream);

        let upload_url_cloned = http_url.clone();
        let handle: JoinHandle<Result<(), CloudTransferErrors>> = tokio::spawn(async move {
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
                return Ok(())
            }

            Err(CloudTransferErrors::UploadProcessError(format!(
                "STATUS {:?}, msg: {:?}",
                response.status(),
                response.text().await
            )))
        });

        Ok(Box::new(NetStreamInnerImpl {
            handle: Some(handle),
            writer
        }))
    }
}

#[async_trait::async_trait]
impl NetStreamInner for NetStreamInnerImpl {
    async fn write(&mut self, data: bytes::Bytes) -> anyhow::Result<()> {
        self.writer.write_all(&data).await?;

        Ok(())
    }

    async fn end(&mut self) -> anyhow::Result<()> {
        self.writer.flush().await?;

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
