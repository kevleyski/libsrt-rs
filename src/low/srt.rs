use std::io::{self, Read, Write, IoSlice, IoSliceMut};
use std::net::SocketAddr;
use std::fmt;

use libsrt_sys::{self as sys, Socket};

pub trait Common {
    /// Returns the internal socket.
    fn as_inner(&self) -> &Socket;

    /// Returns the socket address of the local half of this SRT connection.
    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.as_inner().socket_addr()
    }
}

////////////////////////////////////////////////////////////////////////////////
// SRT streams
////////////////////////////////////////////////////////////////////////////////

pub trait CommonStream : Common {
    /// Returns the socket address of the remote peer of this SRT connection.
    fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.as_inner().peer_addr()
    }
}

/// A SRT stream between a local and a remote socket.
pub struct Stream(Socket);

impl Stream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<Stream> {
        sys::init();

        let sock = Socket::new(addr)?;
        sock.connect(addr)?;
        Ok(Stream(sock))
    }

    pub fn input_stream(self) -> io::Result<InputStream> {
        Ok(InputStream(self.0))
    }

    pub fn output_stream(self) -> io::Result<OutputStream> {
        Ok(OutputStream(self.0))
    }
}

impl Common for Stream {
    fn as_inner(&self) -> &Socket {
        &self.0
    }
}

impl CommonStream for Stream {}

impl fmt::Debug for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("Stream");

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
pub struct InputStream(Socket);

impl Common for InputStream {
    fn as_inner(&self) -> &Socket {
        &self.0
    }
}

impl CommonStream for InputStream {}

impl Read for InputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.recv_vectored(bufs)
    }
}

impl Read for &InputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.0.recv_vectored(bufs)
    }
}

impl fmt::Debug for InputStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("InputStream");

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
pub struct OutputStream(Socket);

impl Common for OutputStream {
    fn as_inner(&self) -> &Socket {
        &self.0
    }
}

impl CommonStream for OutputStream {}

impl Write for OutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.send_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl Write for &OutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.0.send_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl fmt::Debug for OutputStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("OutputStream");

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
pub struct Listener(Socket);

impl Listener {
    /// Creates a new `Listener` which will be bound to the specified
    /// address.
    pub fn bind(addr: &SocketAddr) -> io::Result<Listener> {
        sys::init();

        let sock = Socket::new(addr)?;
        sock.bind(addr)?;
        sock.listen(128)?;
        Ok(Listener(sock))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(Stream, SocketAddr)> {
        let (sock, addr) = self.as_inner().accept()?;
        Ok((Stream(sock), addr))
    }
}

impl Common for Listener {
    fn as_inner(&self) -> &Socket {
        &self.0
    }
}

impl fmt::Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("Listener");

        if let Ok(addr) = self.local_addr() {
            res.field("listen", &addr);
        }

        res.finish()
    }
}
