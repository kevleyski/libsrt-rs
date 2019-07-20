use std::io::{self, Read, Write, IoSlice, IoSliceMut};
use std::net::SocketAddr;

use inner::SrtSocket;

////////////////////////////////////////////////////////////////////////////////
// SRT streams
////////////////////////////////////////////////////////////////////////////////

pub struct SrtStream(SrtSocket);

impl SrtStream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<SrtStream> {
        inner::init();

        let sock = SrtSocket::new(addr)?;
        sock.connect(addr)?;
        Ok(SrtStream(sock))
    }

    /// Returns the socket address of the remote peer of this SRT connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    /// Returns the socket address of the local half of this SRT connection.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }
}

impl Read for SrtStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Ok(0)  // XXX
    }
}

impl Write for SrtStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        Ok(0)  // XXX
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl Read for &SrtStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Ok(0)  // XXX
    }
}

impl Write for &SrtStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        Ok(0)  // XXX
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

////////////////////////////////////////////////////////////////////////////////
// SRT listeners
////////////////////////////////////////////////////////////////////////////////

pub struct SrtListener(SrtSocket);

impl SrtListener {
    /// Creates a new `SrtListener` which will be bound to the specified
    /// address.
    pub fn bind(addr: &SocketAddr) -> io::Result<SrtListener> {
        inner::init();

        let sock = SrtSocket::new(addr)?;
        sock.bind(addr)?;
        sock.listen(128)?;
        Ok(SrtListener(sock))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(SrtStream, SocketAddr)> {
        let (sock, addr) = self.0.accept()?;
        Ok((SrtStream(sock), addr))
    }

    /// Returns the local socket address of this listener.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }
}
