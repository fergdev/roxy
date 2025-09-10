use bytes::{Buf, Bytes};

use http::HeaderMap;
use http_body_util::Full;
use http_body_util::combinators::BoxBody;
use hyper::body::{Body, Frame, SizeHint};
use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::error;

use pin_project_lite::pin_project;

use crate::content::{Encodings, encode_body};

pub type BytesBody = BoxBody<Bytes, Infallible>;

pin_project! {
    pub struct BufferBody<T>
    where
        T: Body,
        T: ?Sized,
    {
        pub(crate) collected: Option<BufferedBody>,
        #[pin]
        pub(crate) body: T,
    }
}

#[allow(clippy::expect_used)]
impl<T: Body + ?Sized> Future for BufferBody<T> {
    type Output = Result<BufferedBody, T::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> std::task::Poll<Self::Output> {
        let mut me = self.project();

        loop {
            let frame = std::task::ready!(me.body.as_mut().poll_frame(cx));

            let frame = if let Some(frame) = frame {
                frame?
            } else {
                return Poll::Ready(Ok(me.collected.take().expect("polled after complete")));
            };

            me.collected
                .as_mut()
                .expect("Buffer was not set")
                .push_frame(frame);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct BufferedBody {
    bufs: Vec<Bytes>,
    trailers: Option<HeaderMap>,
}

impl BufferedBody {
    pub fn with_trailers(bufs: Bytes, trailers: HeaderMap) -> BoxBody<Bytes, Infallible> {
        BoxBody::new(Self {
            bufs: vec![bufs],
            trailers: Some(trailers),
        })
    }

    pub fn with_bufs(mut bufs: Vec<Bytes>, trailers: HeaderMap) -> BoxBody<Bytes, Infallible> {
        bufs.reverse();
        BoxBody::new(Self {
            bufs,
            trailers: Some(trailers),
        })
    }

    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.trailers.as_ref()
    }

    pub(crate) fn push_frame<B>(&mut self, frame: Frame<B>)
    where
        B: Buf,
    {
        let frame = match frame.into_data() {
            Ok(mut data) => {
                while data.has_remaining() {
                    data.advance(data.remaining());
                }
                return;
            }
            Err(frame) => frame,
        };

        if let Ok(trailers) = frame.into_trailers() {
            if let Some(current) = &mut self.trailers {
                current.extend(trailers);
            } else {
                self.trailers = Some(trailers);
            }
        };
    }

    pub fn collect_buffered<T>(body: T) -> BufferBody<T>
    where
        T: Body,
        T: Sized,
    {
        BufferBody {
            body,
            collected: Some(BufferedBody::default()),
        }
    }
}

impl Body for BufferedBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let frame = if let Some(buf) = self.bufs.pop() {
            Frame::data(buf)
        } else if let Some(trailers) = self.trailers.take() {
            Frame::trailers(trailers)
        } else {
            return Poll::Ready(None);
        };

        Poll::Ready(Some(Ok(frame)))
    }

    fn size_hint(&self) -> hyper::body::SizeHint {
        let mut hint = SizeHint::new();
        hint.set_lower(self.bufs.len() as u64);
        hint
    }
}

pub fn create_http_body(
    body: Bytes,
    encoding: Option<Vec<Encodings>>,
    trailers: Option<HeaderMap>,
) -> BoxBody<Bytes, Infallible> {
    let body = match encoding {
        Some(enc) => match encode_body(&body, &enc) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to encode body {e}");
                body
            }
        },
        None => body,
    };

    match trailers {
        Some(trailers) => BufferedBody::with_trailers(body, trailers),
        None => BoxBody::new(Full::new(body)),
    }
}
