use std::io;
use std::mem;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use libc::{self as c, c_int, sockaddr, sockaddr_storage, socklen_t, sockaddr_in, sockaddr_in6};

use crate::error as srterr;

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
        srterr::cvt(f(&mut storage as *mut _ as *mut _, &mut len))?;
        from_sockaddr(&storage, len)
    }
}
