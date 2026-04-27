use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio_stream::Stream;

pub struct StdStream {
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Lines<BufReader<ChildStderr>>
}

impl From<Child> for StdStream {
    fn from(mut value: Child) -> Self {
        Self {
            stdout: BufReader::new(value.stdout.take().unwrap()).lines(),
            stderr: BufReader::new(value.stderr.take().unwrap()).lines()
        }
    }
}

impl Stream for StdStream {
    type Item = String;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let up = self.get_mut();
        let mut stdout = Box::pin(up.stdout.next_line());
        let mut stderr = Box::pin(up.stderr.next_line());

        match stdout.as_mut().poll(cx) {
            Poll::Ready(Ok(result)) => return Poll::Ready(result.map(|it| it.to_string())),
            Poll::Ready(Err(_)) => return Poll::Ready(None),
            _ => {}
        };

        match stderr.as_mut().poll(cx) {
            Poll::Ready(Ok(result)) => Poll::Ready(result.map(|it| it.to_string())),
            Poll::Ready(Err(_)) => Poll::Ready(None),
            _ => Poll::Pending
        }
    }
}
