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

pub trait SrtCommonStream : SrtCommon {
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

/// A SRT stream between a local and a remote socket.
pub struct SrtStream(SrtSocket);

impl SrtStream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<SrtStream> {
        Ok(SrtStream(connect_socket(addr)?))
    }

    pub fn input_stream(self) -> io::Result<SrtInputStream> {
        Ok(SrtInputStream(self.0))
    }

    pub fn output_stream(self) -> io::Result<SrtOutputStream> {
        Ok(SrtOutputStream(self.0))
    }
}

impl SrtCommon for SrtStream {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtCommonStream for SrtStream {}

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

/// A SRT input stream between a local and a remote socket.
pub struct SrtInputStream(SrtSocket);

impl SrtCommon for SrtInputStream {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtCommonStream for SrtInputStream {}

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

impl SrtCommon for SrtOutputStream {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl SrtCommonStream for SrtOutputStream {}

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

/// A SRT input socket server, listening for connections.
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
        let (sock, addr) = self.as_inner().accept()?;
        Ok((SrtStream(sock), addr))
    }
}

impl SrtCommon for SrtListener {
    fn as_inner(&self) -> &SrtSocket {
        &self.0
    }
}

impl fmt::Debug for SrtListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("SrtListener");

        if let Ok(addr) = self.local_addr() {
            res.field("listen", &addr);
        }

        res.finish()
    }
}
