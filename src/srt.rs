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
pub struct SrtInputStream(SrtSocket);

impl SrtInputStream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<SrtInputStream> {
        Ok(SrtInputStream(connect_socket(addr)?))
    }
}

impl SrtCommon for SrtInputStream {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtStream for SrtInputStream {}

impl Read for SrtInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.recv_vectored(bufs)
    }
}

impl Read for &SrtInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.recv_vectored(bufs)
    }
}

impl fmt::Debug for SrtInputStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtInputStream");

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
pub struct SrtOutputStream(SrtSocket);

impl SrtOutputStream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<SrtOutputStream> {
        Ok(SrtOutputStream(connect_socket(addr)?))
    }
}

impl SrtCommon for SrtOutputStream {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtStream for SrtOutputStream {}

impl Write for SrtOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.send_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl Write for &SrtOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.send_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl fmt::Debug for SrtOutputStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtOutputStream");

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
pub struct SrtInputListener(SrtSocket);

impl SrtInputListener {
    /// Creates a new `SrtInputListener` which will be bound to the specified
    /// address.
    pub fn bind(addr: &SocketAddr) -> io::Result<SrtInputListener> {
        Ok(SrtInputListener(bind_socket(addr)?))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(SrtInputStream, SocketAddr)> {
        let (sock, addr) = self.accept_socket()?;
        Ok((SrtInputStream(sock), addr))
    }
}

impl SrtCommon for SrtInputListener {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtListener for SrtInputListener {}

impl fmt::Debug for SrtInputListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtInputListener");

        if let Ok(addr) = self.local_addr() {
            res.field("listen", &addr);
        }

        res.finish()
    }
}

/// A SRT output socket server, listening for connections.
pub struct SrtOutputListener(SrtSocket);

impl SrtOutputListener {
    /// Creates a new `SrtOutputListener` which will be bound to the specified
    /// address.
    pub fn bind(addr: &SocketAddr) -> io::Result<SrtOutputListener> {
        Ok(SrtOutputListener(bind_socket(addr)?))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(SrtOutputStream, SocketAddr)> {
        let (sock, addr) = self.accept_socket()?;
        Ok((SrtOutputStream(sock), addr))
    }
}

impl SrtCommon for SrtOutputListener {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtListener for SrtOutputListener {}

impl fmt::Debug for SrtOutputListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtOutputListener");

        if let Ok(addr) = self.local_addr() {
            res.field("listen", &addr);
        }

        res.finish()
    }
}
