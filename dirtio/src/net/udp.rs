use crate::io::registration::IoRegistration;

use std::io;
use std::net::SocketAddr;

use mio::Interest;

pub struct UdpSocket {
    io: IoRegistration<mio::net::UdpSocket>,
}

impl UdpSocket {
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        Ok(Self {
            io: IoRegistration::new(mio::net::UdpSocket::bind(addr)?)?,
        })
    }

    pub fn connect(&self, addr: SocketAddr) -> io::Result<()> {
        self.io.connect(addr)
    }

    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.io
            .registration()
            .async_io(Interest::READABLE, || self.io.recv(buf))
            .await
    }

    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.io
            .registration()
            .async_io(Interest::READABLE, || self.io.recv_from(buf))
            .await
    }

    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        self.io
            .registration()
            .async_io(Interest::WRITABLE, || self.io.send(buf))
            .await
    }

    pub async fn send_to(&self, buf: &[u8], target: SocketAddr) -> io::Result<usize> {
        self.io
            .registration()
            .async_io(Interest::WRITABLE, || self.io.send_to(buf, target))
            .await
    }
}
