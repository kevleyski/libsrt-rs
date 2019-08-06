use std::io::{self, Read, Write, IoSlice, IoSliceMut};
use std::net::SocketAddr;
use std::fmt;
use std::time::Duration;

use libsrt_sys::{self as sys, Socket};
pub use libsrt_sys::int;
pub use libsrt_sys::{Token, EventKind, Events};

pub trait AsSocket {
    /// Returns the internal socket.
    fn as_socket(&self) -> &Socket;
}

pub trait Bind : AsSocket {
    /// Returns the socket address of the local half of this SRT connection.
    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.as_socket().socket_addr()
    }
}

pub trait Connect : Bind {
    /// Returns the socket address of the remote peer of this SRT connection.
    fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.as_socket().peer_addr()
    }
}

////////////////////////////////////////////////////////////////////////////////
// SRT streams
////////////////////////////////////////////////////////////////////////////////

/// A SRT stream between a local and a remote socket.
pub struct Stream(Socket);

impl Stream {
    /// Opens a SRT connection to a remote host.
    pub fn connect(addr: &SocketAddr) -> io::Result<Stream> {
        sys::init();

        let sock = Socket::new(addr)?;

        sock.set_send_nonblocking(true)?;
        match sock.connect(addr) {
            Ok(_) => {}
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => return Err(e),
        }

        Ok(Stream(sock))
    }

    /// Creates a new `Stream` from the pending socket.
    pub fn from_stream(sock: Socket) -> io::Result<Stream> {
        sock.set_recv_nonblocking(true)?;
        Ok(Stream(sock))
    }

    pub fn input_stream(self) -> io::Result<InputStream> {
        self.0.set_recv_nonblocking(true)?;
        Ok(InputStream(self.0))
    }

    pub fn output_stream(self) -> io::Result<OutputStream> {
        self.0.set_send_nonblocking(true)?;
        Ok(OutputStream(self.0))
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.0.take_error()
    }
}

impl AsSocket for Stream {
    fn as_socket(&self) -> &Socket {
        &self.0
    }
}

impl Bind for Stream {}

impl Connect for Stream {}

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

impl AsSocket for InputStream {
    fn as_socket(&self) -> &Socket {
        &self.0
    }
}

impl Bind for InputStream {}

impl Connect for InputStream {}

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

impl AsSocket for OutputStream {
    fn as_socket(&self) -> &Socket {
        &self.0
    }
}

impl Bind for OutputStream {}

impl Connect for OutputStream {}

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
        sock.set_recv_nonblocking(true)?;
        Ok(Listener(sock))
    }

    /// Accept a new incoming connection from this listener.
    pub fn accept(&self) -> io::Result<(Stream, SocketAddr)> {
        let (sock, addr) = self.as_socket().accept()?;
        Ok((Stream::from_stream(sock)?, addr))
    }
}

impl AsSocket for Listener {
    fn as_socket(&self) -> &Socket {
        &self.0
    }
}

impl Bind for Listener {}

impl fmt::Debug for Listener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = f.debug_struct("Listener");

        if let Ok(addr) = self.local_addr() {
            res.field("listen", &addr);
        }

        res.finish()
    }
}

////////////////////////////////////////////////////////////////////////////////
// SRT Poll
////////////////////////////////////////////////////////////////////////////////

/// Polls for readiness events on all registered values.
pub struct Poll {
    poll: sys::Poll,
}

impl Poll {
    /// Return a new `Poll` handle.
    pub fn new() -> io::Result<Poll> {
        Ok(Poll {
            poll: sys::Poll::new()?,
        })
    }

    /// Register an `AsSocket` instance with the `Poll` instance.
    pub fn register<S: AsSocket>(&self, socket: &S, token: Token, event: EventKind) -> io::Result<()>
    where S: AsSocket
    {
        self.poll.register(socket.as_socket(), token, event)
    }


    /// Re-register an `AsSocket` instance with the `Poll` instance.
    pub fn reregister<S: AsSocket>(&self, socket: &S, token: Token, event: EventKind) -> io::Result<()>
    where S: AsSocket
    {
        self.poll.reregister(socket.as_socket(), token, event)
    }

    /// Deregister an `AsSocket` instance with the `Poll` instance.
    pub fn deregister<S: AsSocket>(&self, socket: &S) -> io::Result<()>
    where S: AsSocket
    {
        self.poll.deregister(socket.as_socket())
    }

    pub fn poll(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<usize> {
        self.poll.poll(events, timeout)
    }
}
