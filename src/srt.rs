use std::io::{self, Read, Write, IoSlice, IoSliceMut};
use std::net::SocketAddr;
use std::fmt;

use inner::SrtSocket;

pub trait SrtCommon {
    /// Returns the internal socket.
    fn as_inner(&self) -> &SrtSocket;

    /// Returns the socket address of the local half of this SRT connection.
    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.as_inner().socket_addr()
    }
}

////////////////////////////////////////////////////////////////////////////////
// SRT streams
////////////////////////////////////////////////////////////////////////////////

pub trait SrtStream : SrtCommon {
    /// Returns the socket address of the remote peer of this SRT connection.
    fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.as_inner().peer_addr()
    }
}

fn connect_socket(addr: &SocketAddr) -> io::Result<SrtSocket> {
    inner::init();

    let sock = SrtSocket::new(addr)?;
    sock.connect(addr)?;
    Ok(sock)
}

/// A SRT input stream between a local and a remote socket.
pub struct SrtInStream(SrtSocket);

impl SrtInStream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<SrtInStream> {
        Ok(SrtInStream(connect_socket(addr)?))
    }
}

impl SrtCommon for SrtInStream {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtStream for SrtInStream {}

impl Read for SrtInStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.recv_vectored(bufs)
    }
}

impl Read for &SrtInStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.recv_vectored(bufs)
    }
}

impl fmt::Debug for SrtInStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtInStream");

        if let Ok(addr) = self.local_addr() {
            res.field("local", &addr);
        }

        if let Ok(peer) = self.peer_addr() {
            res.field("peer", &peer);
        }

        res.finish()
    }
}

/// A SRT output stream between a local and a remote socket.
pub struct SrtOutStream(SrtSocket);

impl SrtOutStream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<SrtOutStream> {
        Ok(SrtOutStream(connect_socket(addr)?))
    }
}

impl SrtCommon for SrtOutStream {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtStream for SrtOutStream {}

impl Write for SrtOutStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.send_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl Write for &SrtOutStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.send_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl fmt::Debug for SrtOutStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtOutStream");

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

pub trait SrtListener : SrtCommon {
    /// Accept a new incoming connection from this listener.
    fn accept_socket(&self) -> io::Result<(SrtSocket, SocketAddr)> {
        let (sock, addr) = self.as_inner().accept()?;
        Ok((sock, addr))
    }
}

fn bind_socket(addr: &SocketAddr) -> io::Result<SrtSocket> {
    inner::init();

    let sock = SrtSocket::new(addr)?;
    sock.bind(addr)?;
    sock.listen(128)?;
    Ok(sock)
}

/// A SRT input socket server, listening for connections.
pub struct SrtInListener(SrtSocket);

impl SrtInListener {
    /// Creates a new `SrtInListener` which will be bound to the specified
    /// address.
    pub fn bind(addr: &SocketAddr) -> io::Result<SrtInListener> {
        Ok(SrtInListener(bind_socket(addr)?))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(SrtInStream, SocketAddr)> {
        let (sock, addr) = self.accept_socket()?;
        Ok((SrtInStream(sock), addr))
    }
}

impl SrtCommon for SrtInListener {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtListener for SrtInListener {}

impl fmt::Debug for SrtInListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtInListener");

        if let Ok(addr) = self.local_addr() {
            res.field("listen", &addr);
        }

        res.finish()
    }
}

/// A SRT output socket server, listening for connections.
pub struct SrtOutListener(SrtSocket);

impl SrtOutListener {
    /// Creates a new `SrtOutListener` which will be bound to the specified
    /// address.
    pub fn bind(addr: &SocketAddr) -> io::Result<SrtOutListener> {
        Ok(SrtOutListener(bind_socket(addr)?))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(SrtOutStream, SocketAddr)> {
        let (sock, addr) = self.accept_socket()?;
        Ok((SrtOutStream(sock), addr))
    }
}

impl SrtCommon for SrtOutListener {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtListener for SrtOutListener {}

impl fmt::Debug for SrtOutListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtOutListener");

        if let Ok(addr) = self.local_addr() {
            res.field("listen", &addr);
        }

        res.finish()
    }
}
