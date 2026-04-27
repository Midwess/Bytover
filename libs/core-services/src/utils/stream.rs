use std::collections::VecDeque;
use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures::io::{AsyncRead, AsyncWrite};

#[derive(Clone)]
pub struct DuplexConfig {
    pub buffer_size: usize,
    pub max_buffer_size: usize
}

impl Default for DuplexConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,
            max_buffer_size: 65536
        }
    }
}

#[derive(Debug)]
struct BufferState {
    buffer: VecDeque<u8>,
    is_closed: bool,
    max_size: usize
}

impl BufferState {
    fn new(max_size: usize) -> Self {
        Self {
            buffer: VecDeque::new(),
            is_closed: false,
            max_size
        }
    }

    fn write_data(&mut self, data: &[u8]) -> Result<usize, Error> {
        if self.is_closed {
            return Err(Error::new(ErrorKind::BrokenPipe, "stream is closed"));
        }

        let available_space = self.max_size.saturating_sub(self.buffer.len());
        if available_space == 0 {
            return Ok(0);
        }

        let bytes_to_write = std::cmp::min(data.len(), available_space);
        self.buffer.extend(&data[..bytes_to_write]);
        Ok(bytes_to_write)
    }

    fn read_data(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        if buf.is_empty() {
            return Ok(0);
        }

        let bytes_to_read = std::cmp::min(buf.len(), self.buffer.len());
        if bytes_to_read == 0 {
            if self.is_closed {
                return Ok(0);
            } else {
                return Err(Error::new(ErrorKind::WouldBlock, "no data available"));
            }
        }

        for (i, byte) in self.buffer.drain(..bytes_to_read).enumerate() {
            buf[i] = byte;
        }

        Ok(bytes_to_read)
    }

    fn close(&mut self) {
        self.is_closed = true;
    }

    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }
}

pub struct DuplexWriter {
    // Buffer where we write data that will be read by the paired reader
    write_buffer: Arc<Mutex<BufferState>>,
    config: DuplexConfig,
    local_buffer: Vec<u8>
}

pub struct DuplexReader {
    // Buffer from where we read data written by the paired writer
    read_buffer: Arc<Mutex<BufferState>>,
    config: DuplexConfig
}

pub fn duplex() -> (DuplexStream, DuplexStream) {
    duplex_with_config(DuplexConfig::default())
}

pub fn duplex_with_config(config: DuplexConfig) -> (DuplexStream, DuplexStream) {
    let buffer1 = Arc::new(Mutex::new(BufferState::new(config.max_buffer_size)));
    let buffer2 = Arc::new(Mutex::new(BufferState::new(config.max_buffer_size)));

    let stream1 = DuplexStream {
        writer: DuplexWriter {
            write_buffer: buffer1.clone(),
            config: config.clone(),
            local_buffer: Vec::with_capacity(config.buffer_size)
        },
        reader: DuplexReader {
            read_buffer: buffer2.clone(),
            config: config.clone()
        }
    };

    let stream2 = DuplexStream {
        writer: DuplexWriter {
            write_buffer: buffer2.clone(),
            config: config.clone(),
            local_buffer: Vec::with_capacity(config.buffer_size)
        },
        reader: DuplexReader {
            read_buffer: buffer1.clone(),
            config: config.clone()
        }
    };

    (stream1, stream2)
}

pub struct DuplexStream {
    writer: DuplexWriter,
    reader: DuplexReader
}

impl DuplexStream {
    pub fn split(self) -> (DuplexReader, DuplexWriter) {
        (self.reader, self.writer)
    }

    pub fn config(&self) -> &DuplexConfig {
        &self.reader.config
    }

    pub fn has_data_available(&self) -> bool {
        if let Ok(buffer) = self.reader.read_buffer.lock() {
            !buffer.is_empty()
        } else {
            false
        }
    }

    pub fn buffer_stats(&self) -> (usize, usize) {
        let read_len = self.reader.read_buffer.lock().map(|b| b.len()).unwrap_or(0);
        let write_len = self.writer.write_buffer.lock().map(|b| b.len()).unwrap_or(0);
        (read_len, write_len)
    }
}

impl AsyncRead for DuplexStream {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, Error>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}

impl AsyncWrite for DuplexStream {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        Pin::new(&mut self.writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.writer).poll_close(cx)
    }
}

impl AsyncRead for DuplexReader {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, Error>> {
        let mut buffer_state = match self.read_buffer.lock() {
            Ok(guard) => guard,
            Err(_) => return Poll::Ready(Err(Error::other("buffer lock poisoned")))
        };

        match buffer_state.read_data(buf) {
            Ok(bytes_read) => Poll::Ready(Ok(bytes_read)),
            Err(e) if e.kind() == ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Err(e))
        }
    }
}

impl AsyncWrite for DuplexWriter {
    fn poll_write(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        // Add data to local buffer for batching
        let available_space = self.config.buffer_size.saturating_sub(self.local_buffer.len());
        if available_space == 0 {
            // Local buffer is full, flush it first
            if let Err(e) = self.flush_local_buffer() {
                return Poll::Ready(Err(e));
            }
        }

        let bytes_to_buffer = std::cmp::min(buf.len(), available_space);
        self.local_buffer.extend_from_slice(&buf[..bytes_to_buffer]);

        // If local buffer is getting large, try to flush
        if self.local_buffer.len() >= self.config.buffer_size / 2 {
            self.flush_local_buffer().ok();
        }

        Poll::Ready(Ok(bytes_to_buffer))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.flush_local_buffer() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(e))
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        // Flush any remaining data
        if let Err(e) = self.flush_local_buffer() {
            return Poll::Ready(Err(e));
        }

        // Mark the paired reader's buffer as closed
        if let Ok(mut buffer) = self.write_buffer.lock() {
            buffer.close();
        }

        Poll::Ready(Ok(()))
    }
}

impl DuplexWriter {
    fn flush_local_buffer(&mut self) -> Result<(), Error> {
        if self.local_buffer.is_empty() {
            return Ok(());
        }

        let mut buffer_state = self.write_buffer.lock().map_err(|_| Error::other("buffer lock poisoned"))?;

        let bytes_written = buffer_state.write_data(&self.local_buffer)?;

        // Remove written bytes from local buffer
        self.local_buffer.drain(..bytes_written);

        if !self.local_buffer.is_empty() {
            // Couldn't write all data - buffer is full
            return Err(Error::new(ErrorKind::WouldBlock, "buffer full"));
        }

        Ok(())
    }
}

// Implement Drop to ensure proper cleanup
impl Drop for DuplexWriter {
    fn drop(&mut self) {
        // Best effort flush on drop
        self.flush_local_buffer().ok();

        // Mark paired reader as closed
        if let Ok(mut buffer) = self.write_buffer.lock() {
            buffer.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::io::{AsyncReadExt, AsyncWriteExt};

    // Simple executor for testing without tokio
    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        use std::task::{RawWaker, RawWakerVTable, Waker};

        static VTABLE: RawWakerVTable =
            RawWakerVTable::new(|_| RawWaker::new(std::ptr::null(), &VTABLE), |_| {}, |_| {}, |_| {});

        let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
        let mut cx = Context::from_waker(&waker);

        let mut future = Box::pin(future);
        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(result) => return result,
                Poll::Pending => {
                    // In a real implementation, we would wait for the future to be ready
                    // For testing, we just continue polling
                    std::thread::yield_now();
                }
            }
        }
    }

    #[test]
    fn test_basic_communication() {
        block_on(async {
            let (mut stream1, mut stream2) = duplex();

            // Write from stream1 to stream2
            stream1.write_all(b"hello").await.unwrap();
            stream1.flush().await.unwrap();

            // Read from stream2
            let mut buffer = vec![0u8; 5];
            stream2.read_exact(&mut buffer).await.unwrap();
            assert_eq!(&buffer, b"hello");

            // Write from stream2 to stream1
            stream2.write_all(b"world").await.unwrap();
            stream2.flush().await.unwrap();

            // Read from stream1
            let mut buffer = vec![0u8; 5];
            stream1.read_exact(&mut buffer).await.unwrap();
            assert_eq!(&buffer, b"world");
        });
    }

    #[test]
    fn test_large_data_transfer() {
        block_on(async {
            let (mut stream1, mut stream2) = duplex();

            // Test with 10KB of data
            let test_data = vec![0xAB; 10240];

            stream1.write_all(&test_data).await.unwrap();
            stream1.flush().await.unwrap();

            let mut buffer = vec![0u8; test_data.len()];
            stream2.read_exact(&mut buffer).await.unwrap();

            assert_eq!(buffer, test_data);
        });
    }

    #[test]
    fn test_buffer_stats() {
        let (stream1, _stream2) = duplex();
        let (read_len, write_len) = stream1.buffer_stats();
        assert_eq!(read_len, 0);
        assert_eq!(write_len, 0);
    }

    #[test]
    fn test_split_functionality() {
        block_on(async {
            let (stream1, stream2) = duplex();
            let (mut reader1, mut writer1) = stream1.split();
            let (mut reader2, mut writer2) = stream2.split();

            writer1.write_all(b"test").await.unwrap();
            writer1.flush().await.unwrap();

            let mut buffer = vec![0u8; 4];
            reader2.read_exact(&mut buffer).await.unwrap();
            assert_eq!(&buffer, b"test");
        });
    }
}
