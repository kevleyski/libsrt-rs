use std::io::{self, Read, Write, IoSlice, IoSliceMut};
use std::net::SocketAddr;
use std::fmt;

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
        self.0.recv_vectored(bufs)
    }
}

impl Write for SrtStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.send_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl Read for &SrtStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.recv_vectored(bufs)
    }
}

impl Write for &SrtStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.send_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl fmt::Debug for SrtStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtStream");

        if let Ok(addr) = self.local_addr() {
            res.field("addr", &addr);
        }

        if let Ok(peer) = self.peer_addr() {
            res.field("peer", &peer);
        }

        res.finish()
    }
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

impl fmt::Debug for SrtListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtListener");

        if let Ok(addr) = self.local_addr() {
            res.field("addr", &addr);
        }

        res.finish()
    }
}
