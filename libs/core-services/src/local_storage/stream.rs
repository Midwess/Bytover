use crate::local_storage::entry::FileEntry;
use bytes::{Bytes, BytesMut};

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait IOCursor: Send + Sync {
    async fn next(&mut self, max: Option<u64>) -> anyhow::Result<Option<&[u8]>>;

    async fn read_all(&mut self) -> anyhow::Result<Bytes> {
        let mut data = BytesMut::new();
        while let Some(current) = self.next(None).await? {
            data.extend_from_slice(current);
        }

        Ok(data.into())
    }

    async fn read_exact(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> {
        let mut remaining = buf.len();
        let mut offset = 0;

        while remaining > 0 {
            let current = self.next(Some(remaining as u64)).await?;
            match current {
                Some(data) => {
                    let copy_len = data.len();
                    buf[offset..offset + copy_len].copy_from_slice(&data[..copy_len]);
                    offset += copy_len;
                    remaining -= copy_len;
                }
                None => {
                    return Ok(offset);
                }
            }
        }

        Ok(offset)
    }

    async fn entry(&self) -> anyhow::Result<FileEntry>;

    async fn end(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn buffer_size(&self) -> Option<usize> {
        None
    }
}
