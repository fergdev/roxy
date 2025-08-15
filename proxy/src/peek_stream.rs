use bytes::Bytes;
use std::io;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

pub struct PeekStream<S> {
    stream: S,
    buffer: Bytes,
    consumed: usize,
}

impl<S: AsyncRead + AsyncWrite + Unpin> PeekStream<S> {
    pub async fn new(mut stream: S, peek_len: usize) -> io::Result<(Self, Bytes)> {
        let mut buf = vec![0u8; peek_len];
        let n = stream.read(&mut buf).await?;
        buf.truncate(n);
        let bytes = Bytes::from(buf);

        let wrapped = Self {
            stream,
            buffer: bytes.clone(),
            consumed: 0,
        };
        Ok((wrapped, bytes))
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for PeekStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        dst: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.consumed < self.buffer.len() {
            let rem = &self.buffer[self.consumed..];
            let to_copy = rem.len().min(dst.remaining());
            dst.put_slice(&rem[..to_copy]);
            self.consumed += to_copy;
            Poll::Ready(Ok(()))
        } else {
            Pin::new(&mut self.stream).poll_read(cx, dst)
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for PeekStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}
