use std::io::{self, IoSlice, IoSliceMut};
use std::mem;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use libc::{self as c, c_char, c_int, sockaddr, sockaddr_storage, sockaddr_in, sockaddr_in6, socklen_t};

use crate::error as err;
use crate::ffi::{self, SRTSOCKET};
pub use crate::ffi::{SRT_SOCKSTATUS};

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
        let sock = unsafe { err::cvt(ffi::srt_socket(af as c_int, c::SOCK_DGRAM, c::IPPROTO_UDP))? };
        Ok(Socket(sock))
    }

    pub fn as_raw(&self) -> SRTSOCKET {
        self.0
    }

    pub fn connect(&self, addr: &SocketAddr) -> io::Result<()> {
        let (addrp, len) = into_sockaddr(addr);
        unsafe {
            err::cvt(ffi::srt_connect(self.0, addrp, len as c_int))?;
        }
        Ok(())
    }

    pub fn bind(&self, addr: &SocketAddr) -> io::Result<()> {
        let (addrp, len) = into_sockaddr(addr);
        unsafe {
            err::cvt(ffi::srt_bind(self.0, addrp, len as c_int))?;
        }
        Ok(())
    }

    pub fn listen(&self, backlog: usize) -> io::Result<()> {
        unsafe {
            err::cvt(ffi::srt_listen(self.0, backlog as c_int))?;
        }
        Ok(())
    }

    pub fn accept(&self) -> io::Result<(Socket, SocketAddr)> {
        let mut storage: sockaddr_storage = unsafe { mem::zeroed() };
        let mut len = mem::size_of_val(&storage) as socklen_t;
        let sock = unsafe {
            err::cvt(ffi::srt_accept(
                self.0,
                &mut storage as *mut _ as *mut _,
                &mut len as *mut _ as *mut _,
            ))?
        };
        let addr = from_sockaddr(&storage, len)?;
        Ok((Socket(sock), addr))
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        sockname(|buf, len| unsafe {
            ffi::srt_getpeername(self.0, buf, len as *mut _)
        })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        sockname(|buf, len| unsafe {
            ffi::srt_getsockname(self.0, buf, len as *mut _)
        })
    }

    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        let ret = err::cvt(unsafe {
            ffi::srt_recvmsg(
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
        let ret = err::cvt(unsafe {
            ffi::srt_sendmsg(
                self.0,
                buf.as_ptr() as *const c_char,
                buf.len() as c_int)
        })?;
        Ok(ret as usize)
    }

    pub fn send_vectored(&self, _bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "not implemented"))
    }

    pub fn set_recv_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        let mut blocking = (! nonblocking) as libc::c_int;
        err::cvt(unsafe {
            ffi::srt_setsockopt(
                self.0,
                0,
                ffi::SRT_SOCKOPT::SRTO_RCVSYN,
                &mut blocking as *mut _ as *mut _,
                mem::size_of::<c_int>() as c_int)
        })?;
        Ok(())
    }

    pub fn set_send_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        let mut blocking = (! nonblocking) as libc::c_int;
        err::cvt(unsafe {
            ffi::srt_setsockopt(
                self.0,
                0,
                ffi::SRT_SOCKOPT::SRTO_SNDSYN,
                &mut blocking as *mut _ as *mut _,
                mem::size_of::<c_int>() as c_int)
        })?;
        Ok(())
    }

    pub fn getsockstate(&self) -> io::Result<ffi::SRT_SOCKSTATUS> {
        Ok(unsafe { ffi::srt_getsockstate(self.0) })
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            ffi::srt_close(self.0);
        }
    }
}

// XXX copied from libstd::net::addr
pub fn into_sockaddr(addr: &SocketAddr) -> (*const sockaddr, socklen_t) {
    match *addr {
        SocketAddr::V4(ref a) => (
            a as *const _ as *const _,
            mem::size_of_val(a) as socklen_t,
        ),
        SocketAddr::V6(ref a) => (
            a as *const _ as *const _,
            mem::size_of_val(a) as socklen_t,
        ),
    }
}

// XXX copied from libstd::net::addr
pub fn from_sockaddr(storage: &sockaddr_storage, len: socklen_t) -> io::Result<SocketAddr> {
    match storage.ss_family as c_int {
        c::AF_INET => {
            assert!(len as usize >= mem::size_of::<sockaddr_in>());
            Ok(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::from(unsafe {
                    (*(storage as *const _ as *const sockaddr_in))
                        .sin_addr
                        .s_addr as u32
                }),
                unsafe { (*(storage as *const _ as *const sockaddr_in)).sin_port },
            )))
        }
        c::AF_INET6 => {
            assert!(len as usize >= mem::size_of::<sockaddr_in6>());
            Ok(SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::from(unsafe {
                    (*(storage as *const _ as *const sockaddr_in6))
                        .sin6_addr
                        .s6_addr
                }),
                unsafe { (*(storage as *const _ as *const sockaddr_in6)).sin6_port },
                unsafe { (*(storage as *const _ as *const sockaddr_in6)).sin6_flowinfo },
                unsafe { (*(storage as *const _ as *const sockaddr_in6)).sin6_scope_id },
            )))
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid argument",)),
    }
}

// XXX copied from libstd::sys-common::net
pub fn sockname<F>(f: F) -> io::Result<SocketAddr>
    where F: FnOnce(*mut sockaddr, *mut socklen_t) -> c_int
{
    unsafe {
        let mut storage: sockaddr_storage = mem::zeroed();
        let mut len = mem::size_of_val(&storage) as socklen_t;
        err::cvt(f(&mut storage as *mut _ as *mut _, &mut len))?;
        from_sockaddr(&storage, len)
    }
}
