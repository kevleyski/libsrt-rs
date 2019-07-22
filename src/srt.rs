use std::io::{self, Read, Write, IoSlice, IoSliceMut};
use std::net::SocketAddr;
use std::fmt;

use inner::SrtSocket;

////////////////////////////////////////////////////////////////////////////////
// SRT streams
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct SrtInStream(SrtStream);

#[derive(Debug)]
pub struct SrtOutStream(SrtStream);

impl SrtInStream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<SrtInStream> {
        Ok(SrtInStream(SrtStream::connect(addr)?))
    }

    /// Returns the socket address of the remote peer of this SRT connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    /// Returns the socket address of the local half of this SRT connection.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

impl Read for SrtInStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }
}

impl Read for &SrtInStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.read_vectored(bufs)
    }
}

impl SrtOutStream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<SrtOutStream> {
        Ok(SrtOutStream(SrtStream::connect(addr)?))
    }

    /// Returns the socket address of the remote peer of this SRT connection.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    /// Returns the socket address of the local half of this SRT connection.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

impl Write for SrtOutStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl Write for &SrtOutStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.write_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

struct SrtStream(SrtSocket);

impl SrtStream {
    fn connect(addr: &SocketAddr) -> io::Result<SrtStream> {
        inner::init();

        let sock = SrtSocket::new(addr)?;
        sock.connect(addr)?;
        Ok(SrtStream(sock))
    }

    fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }

    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.recv_vectored(bufs)
    }

    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.send_vectored(bufs)
    }

    fn flush(&self) -> io::Result<()> { Ok(()) }
}

impl fmt::Debug for SrtStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtStream");

        if let Ok(addr) = self.local_addr() {
            res.field("local", &addr);
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

#[derive(Debug)]
pub struct SrtInListener(SrtListener);

#[derive(Debug)]
pub struct SrtOutListener(SrtListener);

impl SrtInListener {
    /// Creates a new `SrtListener` which will be bound to the specified
    /// address.
    pub fn bind(addr: &SocketAddr) -> io::Result<SrtInListener> {
        Ok(SrtInListener(SrtListener::bind(addr)?))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(SrtInStream, SocketAddr)> {
        let (sock, addr) = self.0.accept()?;
        Ok((SrtInStream(sock), addr))
    }

    /// Returns the local socket address of this listener.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

impl SrtOutListener {
    /// Creates a new `SrtListener` which will be bound to the specified
    /// address.
    pub fn bind(addr: &SocketAddr) -> io::Result<SrtOutListener> {
        Ok(SrtOutListener(SrtListener::bind(addr)?))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(SrtOutStream, SocketAddr)> {
        let (sock, addr) = self.0.accept()?;
        Ok((SrtOutStream(sock), addr))
    }

    /// Returns the local socket address of this listener.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

struct SrtListener(SrtSocket);

impl SrtListener {
    /// Creates a new `SrtListener` which will be bound to the specified
    /// address.
    fn bind(addr: &SocketAddr) -> io::Result<SrtListener> {
        inner::init();

        let sock = SrtSocket::new(addr)?;
        sock.bind(addr)?;
        sock.listen(128)?;
        Ok(SrtListener(sock))
    }

    /// Accept a new incoming connection from this listener.
    fn accept(&self) -> io::Result<(SrtStream, SocketAddr)> {
        let (sock, addr) = self.0.accept()?;
        Ok((SrtStream(sock), addr))
    }

    /// Returns the local socket address of this listener.
    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.socket_addr()
    }
}

impl fmt::Debug for SrtListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtListener");

        if let Ok(addr) = self.local_addr() {
            res.field("local", &addr);
        }

        res.finish()
    }
}
