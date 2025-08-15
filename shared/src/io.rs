use std::{
    pin::Pin,
    task::{Context, Poll},
};

use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite};

use std::io;
use std::net::{SocketAddr, UdpSocket};
use tokio::net::TcpListener;

pub async fn local_tcp_listener(port: Option<u16>) -> Result<TcpListener, io::Error> {
    TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port.unwrap_or(0)))).await
}

pub fn local_udp_socket(port: Option<u16>) -> Result<UdpSocket, io::Error> {
    UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], port.unwrap_or(0))))
}

pub struct IOTypeNotSend<S> {
    stream: TokioIo<S>,
}

impl<S> IOTypeNotSend<S> {
    pub fn new(stream: TokioIo<S>) -> Self {
        Self { stream }
    }

    pub fn new_raw(stream: S) -> Self {
        Self {
            stream: TokioIo::new(stream),
        }
    }
}

impl<S: AsyncWrite + Unpin> hyper::rt::Write for IOTypeNotSend<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

impl<S: AsyncRead + Unpin> hyper::rt::Read for IOTypeNotSend<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}

pub struct IOTypeNotSendBox<S> {
    stream: TokioIo<Box<S>>,
}

impl<S> IOTypeNotSendBox<S> {
    pub fn new(stream: TokioIo<Box<S>>) -> Self {
        Self { stream }
    }

    pub fn new_raw(stream: S) -> Self {
        Self {
            stream: TokioIo::new(Box::new(stream)),
        }
    }
}

impl<S: AsyncWrite + Unpin> hyper::rt::Write for IOTypeNotSendBox<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

impl<S: AsyncRead + Unpin> hyper::rt::Read for IOTypeNotSendBox<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: hyper::rt::ReadBufCursor<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}
