use std::{
    ffi::CStr,
    io::{self, IoSlice, IoSliceMut},
    mem,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};
use libc::{
    self as c, c_char, c_int as int, sockaddr, sockaddr_in, sockaddr_in6, sockaddr_storage,
    socklen_t,
};

use crate::error as err;
pub use crate::ffi::{SRT_SOCKSTATUS, SRT_TRANSTYPE};
use crate::ffi::{self, SRTSOCKET};

pub const SRT_LIVE_DEF_PLSIZE: usize = 1316; // = 188*7, recommended for MPEG TS

#[derive(Debug)]
pub struct Socket(SRTSOCKET);

impl Socket {
    pub fn new(addr: &SocketAddr) -> io::Result<Socket> {
        let fam = match *addr {
            SocketAddr::V4(..) => c::AF_INET,
            SocketAddr::V6(..) => c::AF_INET6,
        };
        Socket::new_raw(fam)
    }

    fn new_raw(af: int) -> io::Result<Socket> {
        let sock = unsafe { err::cvt(ffi::srt_socket(af as int, c::SOCK_DGRAM, c::IPPROTO_UDP))? };
        Ok(Socket(sock))
    }

    pub fn as_raw(&self) -> SRTSOCKET {
        self.0
    }

    pub fn connect(&self, addr: &SocketAddr) -> io::Result<()> {
        let (addrp, len) = into_sockaddr(addr);
        unsafe {
            err::cvt(ffi::srt_connect(self.0, addrp, len as int))?;
        }
        Ok(())
    }

    pub fn bind(&self, addr: &SocketAddr) -> io::Result<()> {
        let (addrp, len) = into_sockaddr(addr);
        unsafe {
            err::cvt(ffi::srt_bind(self.0, addrp, len as int))?;
        }
        Ok(())
    }

    pub fn listen(&self, backlog: usize) -> io::Result<()> {
        unsafe {
            err::cvt(ffi::srt_listen(self.0, backlog as int))?;
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
        sockname(|buf, len| unsafe { ffi::srt_getpeername(self.0, buf, len as *mut _) })
    }

    pub fn socket_addr(&self) -> io::Result<SocketAddr> {
        sockname(|buf, len| unsafe { ffi::srt_getsockname(self.0, buf, len as *mut _) })
    }

    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        let ret = err::cvt(unsafe {
            ffi::srt_recvmsg(self.0, buf.as_mut_ptr() as *mut c_char, buf.len() as int)
        })?;
        Ok(ret as usize)
    }

    pub fn recv_vectored(&self, _bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "not implemented"))
    }

    pub fn send(&self, buf: &[u8]) -> io::Result<usize> {
        let ret = err::cvt(unsafe {
            ffi::srt_sendmsg(self.0, buf.as_ptr() as *const c_char, buf.len() as int)
        })?;
        Ok(ret as usize)
    }

    pub fn send_vectored(&self, _bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "not implemented"))
    }

    pub fn set_recv_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        let mut blocking = (!nonblocking) as int;
        err::cvt(unsafe {
            ffi::srt_setsockopt(
                self.0,
                0,
                ffi::SRT_SOCKOPT::SRTO_RCVSYN,
                &mut blocking as *mut _ as *mut _,
                mem::size_of::<int>() as int,
            )
        })?;
        Ok(())
    }

    pub fn set_send_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        let mut blocking = (!nonblocking) as int;
        err::cvt(unsafe {
            ffi::srt_setsockopt(
                self.0,
                0,
                ffi::SRT_SOCKOPT::SRTO_SNDSYN,
                &mut blocking as *mut _ as *mut _,
                mem::size_of::<int>() as int,
            )
        })?;
        Ok(())
    }

    pub fn set_sender(&self, sender: bool) -> io::Result<()> {
        let mut sender = sender as int;
        err::cvt(unsafe {
            ffi::srt_setsockflag(
                self.0,
                ffi::SRT_SOCKOPT::SRTO_SENDER,
                &mut sender as *mut _ as *mut _,
                mem::size_of::<int>() as int,
            )
        })?;
        Ok(())
    }

    pub fn set_tsbpd_mode(&self, tsbpd_mode: bool) -> io::Result<()> {
        let mut tsbpd_mode = tsbpd_mode as int;
        err::cvt(unsafe {
            ffi::srt_setsockopt(
                self.0,
                0,
                ffi::SRT_SOCKOPT::SRTO_TSBPDMODE,
                &mut tsbpd_mode as *mut _ as *mut _,
                mem::size_of::<int>() as int,
            )
        })?;
        Ok(())
    }

    pub fn set_payload_size(&self, payload_size: usize) -> io::Result<()> {
        let mut payload_size = payload_size as int;
        err::cvt(unsafe {
            ffi::srt_setsockopt(
                self.0,
                0,
                ffi::SRT_SOCKOPT::SRTO_PAYLOADSIZE,
                &mut payload_size as *mut _ as *mut _,
                mem::size_of::<int>() as int,
            )
        })?;
        Ok(())
    }

    pub fn set_trans_type(&self, trans_type: SRT_TRANSTYPE) -> io::Result<()> {
        let mut trans_type = trans_type as int;
        err::cvt(unsafe {
            ffi::srt_setsockopt(
                self.0,
                0,
                ffi::SRT_SOCKOPT::SRTO_TRANSTYPE,
                &mut trans_type as *mut _ as *mut _,
                mem::size_of::<int>() as int,
            )
        })?;
        Ok(())
    }

    pub fn is_broken(&self) -> io::Result<bool> {
        Ok(unsafe {
            ffi::srt_getsockstate(self.0) == ffi::SRT_SOCKSTATUS::SRTS_BROKEN
        })
    }

    pub fn is_closing(&self) -> io::Result<bool> {
        Ok(unsafe {
            ffi::srt_getsockstate(self.0) == ffi::SRT_SOCKSTATUS::SRTS_CLOSING
        })
    }

    pub fn is_closed(&self) -> io::Result<bool> {
        Ok(unsafe {
            ffi::srt_getsockstate(self.0) == ffi::SRT_SOCKSTATUS::SRTS_CLOSED
        })
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        let mut errno: int = 0;
        let errcode = unsafe { ffi::srt_getlasterror(&mut errno) };
        if errno == 0 {
            return Ok(None);
        }
        let errstr = unsafe { CStr::from_ptr(ffi::srt_strerror(errcode, errno)).to_string_lossy() };
        let err = err::Error::new(errcode, errstr);
        Ok(Some(io::Error::new(err.kind(), err)))
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
        SocketAddr::V4(ref a) => (a as *const _ as *const _, mem::size_of_val(a) as socklen_t),
        SocketAddr::V6(ref a) => (a as *const _ as *const _, mem::size_of_val(a) as socklen_t),
    }
}

// XXX copied from libstd::net::addr
pub fn from_sockaddr(storage: &sockaddr_storage, len: socklen_t) -> io::Result<SocketAddr> {
    match storage.ss_family as int {
        c::AF_INET => {
            assert!(len as usize >= mem::size_of::<sockaddr_in>());
            Ok(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::from(unsafe {
                    ntoh((*(storage as *const _ as *const sockaddr_in))
                         .sin_addr
                         .s_addr as u32)
                }),
                unsafe {
                    ntoh((*(storage as *const _ as *const sockaddr_in)).sin_port)
                },
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
                unsafe {
                    ntoh((*(storage as *const _ as *const sockaddr_in6)).sin6_port)
                },
                unsafe {
                    ntoh((*(storage as *const _ as *const sockaddr_in6)).sin6_flowinfo)
                },
                unsafe {
                    ntoh((*(storage as *const _ as *const sockaddr_in6)).sin6_scope_id)
                },
            )))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid argument",
        )),
    }
}

// XXX copied from libstd::sys-common::net
pub fn sockname<F>(f: F) -> io::Result<SocketAddr>
where
    F: FnOnce(*mut sockaddr, *mut socklen_t) -> int,
{
    unsafe {
        let mut storage: sockaddr_storage = mem::zeroed();
        let mut len = mem::size_of_val(&storage) as socklen_t;
        err::cvt(f(&mut storage as *mut _ as *mut _, &mut len))?;
        from_sockaddr(&storage, len)
    }
}

// XXX copied from libstd::net::addr
#[doc(hidden)]
trait NetInt {
    fn from_be(i: Self) -> Self;
    fn to_be(&self) -> Self;
}
macro_rules! doit {
    ($($t:ident)*) => ($(impl NetInt for $t {
        fn from_be(i: Self) -> Self { <$t>::from_be(i) }
        fn to_be(&self) -> Self { <$t>::to_be(*self) }
    })*)
}
doit! { i8 i16 i32 i64 isize u8 u16 u32 u64 usize }

// fn hton<I: NetInt>(i: I) -> I { i.to_be() }
fn ntoh<I: NetInt>(i: I) -> I { I::from_be(i) }

#[cfg(test)]
mod socket_tests {
    use super::*;

    #[test]
    fn into_from_sockaddr() {
        let addr = "192.168.128.64:12345".parse().unwrap();
        let (addrp, len) = into_sockaddr(&addr);
        let result = unsafe {
            from_sockaddr(&*(addrp as *const _ as *const _), len).unwrap()
        };
        assert_eq!(addr, result);

        let addr_v6 = "[2001:db8:85a3:0:0:8a2e:370:7334]:23456".parse().unwrap();
        let (addrp_v6, len_v6) = into_sockaddr(&addr_v6);
        let result_v6 = unsafe {
            from_sockaddr(&*(addrp_v6 as *const _ as *const _), len_v6).unwrap()
        };
        assert_eq!(addr_v6, result_v6);
    }
}
