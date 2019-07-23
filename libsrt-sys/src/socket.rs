use std::io::{self, IoSlice, IoSliceMut};
use std::mem;
use std::net::SocketAddr;

use libc::{self as c, c_char, c_int, sockaddr_storage, socklen_t};

use crate::error as srterr;
use crate::ffi::{self as srtffi, SRTSOCKET};
use crate::net as srtnet;

pub struct Socket(SRTSOCKET);

impl Socket {
    pub fn new(addr: &SocketAddr) -> io::Result<Socket> {
        let fam = match *addr {
            SocketAddr::V4(..) => c::AF_INET,
            SocketAddr::V6(..) => c::AF_INET6,
        };
        Socket::new_raw(fam)
    }

    fn new_raw(af: c_int) -> io::Result<Socket> {
        let sock = unsafe { srterr::cvt(srtffi::srt_socket(af as c_int, c::SOCK_DGRAM, c::IPPROTO_UDP))? };
        Ok(Socket(sock))
    }

    pub fn connect(&self, addr: &SocketAddr) -> io::Result<()> {
        let (addrp, len) = srtnet::into_sockaddr(addr);
        unsafe {
            srterr::cvt(srtffi::srt_connect(self.0, addrp, len as c_int))?;
        }
        Ok(())
    }

    pub fn bind(&self, addr: &SocketAddr) -> io::Result<()> {
        let (addrp, len) = srtnet::into_sockaddr(addr);
        unsafe {
            srterr::cvt(srtffi::srt_bind(self.0, addrp, len as c_int))?;
        }
        Ok(())
    }

    pub fn listen(&self, backlog: usize) -> io::Result<()> {
        unsafe {
            srterr::cvt(srtffi::srt_listen(self.0, backlog as c_int))?;
        }
        Ok(())
    }

    pub fn accept(&self) -> io::Result<(Socket, SocketAddr)> {
        let mut storage: sockaddr_storage = unsafe { mem::zeroed() };
        let mut len = mem::size_of_val(&storage) as socklen_t;
        let sock = unsafe {
            srterr::cvt(srtffi::srt_accept(
                self.0,
                &mut storage as *mut _ as *mut _,
                &mut len as *mut _ as *mut _,
            ))?
        };
        let addr = srtnet::from_sockaddr(&storage, len)?;
        Ok((Socket(sock), addr))
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        srtnet::sockname(|buf, len| unsafe {
            srtffi::srt_getpeername(self.0, buf, len as *mut _)
        })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        srtnet::sockname(|buf, len| unsafe {
            srtffi::srt_getsockname(self.0, buf, len as *mut _)
        })
    }

    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        let ret = srterr::cvt(unsafe {
            srtffi::srt_recvmsg(
                self.0,
                buf.as_mut_ptr() as *mut c_char,
                buf.len() as c_int)
        })?;
        Ok(ret as usize)
    }

    pub fn recv_vectored(&self, _bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "not implemented"))
    }

    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
        let ret = srterr::cvt(unsafe {
            srtffi::srt_sendmsg(
                self.0,
                buf.as_ptr() as *const c_char,
                buf.len() as c_int)
        })?;
        Ok(ret as usize)
    }

    pub fn send_vectored(&self, _bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "not implemented"))
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            srtffi::srt_close(self.0);
        }
    }
}
