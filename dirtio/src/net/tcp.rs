use crate::io::registration::IoRegistration;

use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{AsyncRead, AsyncWrite};
use mio::Interest;

pub struct TcpListener {
    io: IoRegistration<mio::net::TcpListener>,
}

impl TcpListener {
    pub async fn accept(&mut self) -> io::Result<(TcpStream, SocketAddr)> {
        let (stream, addr) = self
            .io
            .registration()
            .async_io(Interest::READABLE, || self.io.accept())
            .await?;
        Ok((TcpStream::from_mio(stream)?, addr))
    }

    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        Ok(Self {
            io: IoRegistration::new(mio::net::TcpListener::bind(addr)?)?,
        })
    }
}

pub struct TcpStream {
    io: IoRegistration<mio::net::TcpStream>,
}

impl TcpStream {
    pub fn connect(addr: SocketAddr) -> io::Result<Self> {
        Ok(Self {
            io: IoRegistration::new(mio::net::TcpStream::connect(addr)?)?,
        })
    }

    fn from_mio(io: mio::net::TcpStream) -> io::Result<Self> {
        Ok(Self {
            io: IoRegistration::new(io)?,
        })
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.io
            .registration()
            .poll_io(cx, Interest::READABLE, || (&*self.io).read(buf))
    }
}

impl AsyncWrite for TcpStream {
    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(self.io.shutdown(std::net::Shutdown::Write))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.io
            .registration()
            .poll_io(cx, Interest::WRITABLE, || (&*self.io).flush())
    }

    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.io
            .registration()
            .poll_io(cx, Interest::WRITABLE, || (&*self.io).write(buf))
    }
}
