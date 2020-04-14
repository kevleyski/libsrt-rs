use std::{
    cmp,
    collections::{HashMap, HashSet},
    ffi::CStr,
    fmt,
    io,
    iter::FromIterator,
    ops,
    ptr,
    sync::RwLock,
    time::Duration,
};
use crate::error as err;
use crate::ffi::{self, int, SRTSOCKET};
use crate::socket::Socket;

pub struct Poll {
    epid: int,
    socks: RwLock<HashMap<SRTSOCKET, Token>>, // XXX or RefCell
}

/// Polls for readiness events on all registered sockets.
impl Poll {
    /// Return a new `Poll` handle.
    pub fn new() -> io::Result<Poll> {
        let epid = err::cvt(unsafe { ffi::srt_epoll_create() })?;
        Ok(Poll {
            epid: epid,
            socks: RwLock::new(HashMap::new()),
        })
    }

    /// Register the socket on the `Poll` instance.
    pub fn register(&self, sock: &Socket, token: Token, event: EventKind) -> io::Result<()> {
        let e = event.0;
        err::cvt(unsafe { ffi::srt_epoll_add_usock(self.epid, sock.as_raw(), &e) })?;
        self.socks.write().unwrap().insert(sock.as_raw(), token);
        Ok(())
    }

    /// Re-register the socket with the `Poll` instance.
    pub fn reregister(&self, sock: &Socket, token: Token, event: EventKind) -> io::Result<()> {
        let e = event.0;
        err::cvt(unsafe { ffi::srt_epoll_update_usock(self.epid, sock.as_raw(), &e) })?;
        self.socks.write().unwrap().insert(sock.as_raw(), token);
        Ok(())
    }

    /// Deregister the socket from the `Poll` instance.
    pub fn deregister(&self, sock: &Socket) -> io::Result<()> {
        self.socks.write().unwrap().remove(&(sock.as_raw()));
        err::cvt(unsafe { ffi::srt_epoll_remove_usock(self.epid, sock.as_raw()) })?;
        Ok(())
    }

    /// Block the current thread and wait for an I/O event on the `Poll` instance.
    pub fn poll(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<usize> {
        let max_socks = self.socks.read().unwrap().len();
        let mut rd_socks: Vec<int> = Vec::with_capacity(max_socks);
        let mut wr_socks: Vec<int> = Vec::with_capacity(max_socks);
        let timeout_ms = if let Some(timeout) = timeout {
            let secs = timeout.as_secs();
            if secs > (i32::max_value() / 1000) as u64 {
                // Duration too large, clamp at maximum value.
                i32::max_value() as i64
            } else {
                secs as i64 * 1000 + timeout.subsec_nanos() as i64 / 1000_000
            }
        } else {
            i32::max_value() as i64
        };

        let mut rd_num = max_socks;
        let mut wr_num = max_socks;
        let ret = unsafe {
            ffi::srt_epoll_wait(
                self.epid,
                rd_socks.as_mut_ptr(),
                &mut rd_num as *mut _ as *mut _,
                wr_socks.as_mut_ptr(),
                &mut wr_num as *mut _ as *mut _,
                timeout_ms,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        if ret > 0 {
            if rd_num > 0 {
                unsafe { rd_socks.set_len(rd_num) };
            }
            if wr_num > 0 {
                unsafe { wr_socks.set_len(wr_num) };
            }
        } else if ret == 0 {
            unsafe {
                rd_socks.set_len(0);
                wr_socks.set_len(0);
            }
        } else {
            let mut errno: int = 0;
            let errcode = unsafe { ffi::srt_getlasterror(&mut errno) };
            if errcode == 6003 {
                // XXX SRT_ETIMEOUT (MJ_AGAIN, XMTIMEOUT)
                unsafe {
                    rd_socks.set_len(0);
                    wr_socks.set_len(0);
                }
            } else {
                let errstr =
                    unsafe { CStr::from_ptr(ffi::srt_strerror(errcode, errno)).to_string_lossy() };
                let err = err::Error::new(errcode, errstr);
                return Err(io::Error::new(err.kind(), err));
            }
        }

        let mut new_evts = Events::with_capacity(cmp::max(rd_socks.len(), wr_socks.len()));
        let mut wr_socks_set = HashSet::<SRTSOCKET>::from_iter(wr_socks.to_vec());
        for sock in rd_socks {
            if sock == ffi::SRT_INVALID_SOCK {
                continue;
            }
            if wr_socks_set.contains(&sock) {
                wr_socks_set.remove(&sock);
                if srt_is_closed(sock) {
                    new_evts.push(Event::new(
                        *(self.socks.read().unwrap().get(&sock).unwrap()),
                        EventKind::error(),
                    ));
                } else {
                    new_evts.push(Event::new(
                        *(self.socks.read().unwrap().get(&sock).unwrap()),
                        EventKind::readable() | EventKind::writable(),
                    ));
                }
            } else {
                if srt_is_closed(sock) {
                    new_evts.push(Event::new(
                        *(self.socks.read().unwrap().get(&sock).unwrap()),
                        EventKind::error(),
                    ));
                } else {
                    new_evts.push(Event::new(
                        *(self.socks.read().unwrap().get(&sock).unwrap()),
                        EventKind::readable(),
                    ));
                }
            }
        }

        for sock in wr_socks {
            if sock == ffi::SRT_INVALID_SOCK {
                continue;
            }
            if wr_socks_set.contains(&sock) {
                if srt_is_closed(sock) {
                    new_evts.push(Event::new(
                        *(self.socks.read().unwrap().get(&sock).unwrap()),
                        EventKind::error(),
                    ));
                } else {
                    new_evts.push(Event::new(
                        *(self.socks.read().unwrap().get(&sock).unwrap()),
                        EventKind::writable(),
                    ));
                }
            }
        }

        let ret = new_evts.len();

        events.append(&mut new_evts);

        Ok(ret)
    }
}

impl Drop for Poll {
    fn drop(&mut self) {
        unsafe {
            ffi::srt_epoll_release(self.epid);
        }
    }
}

fn srt_is_closed(sock: SRTSOCKET) -> bool {
    match unsafe { ffi::srt_getsockstate(sock) } {
        ffi::SRT_SOCKSTATUS::SRTS_BROKEN => true,
        ffi::SRT_SOCKSTATUS::SRTS_NONEXIST => true,
        ffi::SRT_SOCKSTATUS::SRTS_CLOSED => true,
        _ => false,
    }
}

/// A set of readiness event kinds
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventKind(int);

const READABLE: int = ffi::SRT_EPOLL_OPT::SRT_EPOLL_IN as int;
const WRITABLE: int = ffi::SRT_EPOLL_OPT::SRT_EPOLL_OUT as int;
const ERROR: int = ffi::SRT_EPOLL_OPT::SRT_EPOLL_ERR as int;

impl EventKind {
    /// Returns the empty `EventKind` set.
    pub fn empty() -> EventKind {
        EventKind(0)
    }

    /// Returns a `EventKind` representing readable readiness.
    pub fn readable() -> EventKind {
        EventKind(READABLE)
    }

    /// Returns a `EventKind` representing writable readiness.
    pub fn writable() -> EventKind {
        EventKind(WRITABLE)
    }

    /// Returns a `EventKind` representing error readiness.
    pub fn error() -> EventKind {
        EventKind(ERROR)
    }

    /// Returns a `EventKind` representing readiness for all operations.
    pub fn all() -> EventKind {
        EventKind(READABLE | WRITABLE | ERROR)
    }

    /// Returns true if `EventKind` is the empty set
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Returns true if the value includes readable event.
    pub fn is_readable(self) -> bool {
        (self.0 & READABLE) != 0
    }

    /// Returns true if the value includes writable event.
    pub fn is_writable(self) -> bool {
        (self.0 & WRITABLE) != 0
    }

    /// Returns true if the value includes error event.
    pub fn is_error(&self) -> bool {
        (self.0 & ERROR) != 0
    }

    pub fn from_int(val: int) -> EventKind {
        EventKind(val)
    }

    pub fn as_int(&self) -> int {
        self.0
    }
}

impl ops::BitOr for EventKind {
    type Output = Self;

    #[inline]
    fn bitor(self, other: Self) -> Self {
        EventKind(self.0 | other.0)
    }
}

impl ops::BitOrAssign for EventKind {
    #[inline]
    fn bitor_assign(&mut self, other: Self) {
        self.0 = (*self | other).0;
    }
}

impl ops::BitAnd for EventKind {
    type Output = Self;

    #[inline]
    fn bitand(self, other: Self) -> Self {
        EventKind(self.0 & other.0)
    }
}

impl ops::BitAndAssign for EventKind {
    #[inline]
    fn bitand_assign(&mut self, other: Self) {
        self.0 = (*self & other).0;
    }
}

impl fmt::Debug for EventKind {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut one = false;
        if self.is_readable() {
            write!(fmt, "READABLE")?;
            one = true;
        }
        if self.is_writable() {
            if one {
                write!(fmt, " | ")?
            }
            write!(fmt, "WRITABLE")?;
            one = true;
        }
        if self.is_error() {
            if one {
                write!(fmt, " | ")?
            }
            write!(fmt, "ERROR")?;
        }
        Ok(())
    }
}

/// An event returned by [`Poll::poll`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Event {
    token: Token,
    kind: EventKind,
}

impl Event {
    pub fn new(token: Token, kind: EventKind) -> Event {
        Event {
            token: token,
            kind: kind,
        }
    }

    pub fn token(&self) -> Token {
        self.token
    }

    pub fn kind(&self) -> EventKind {
        self.kind
    }
}

/// A collection of readiness events.
pub struct Events {
    events: Vec<Event>,
}

/// A collection of readiness events.
impl Events {
    /// Return a new `Events` capable of holding up to `capacity` events.
    pub fn with_capacity(u: usize) -> Events {
        Events {
            events: Vec::with_capacity(u),
        }
    }

    /// Returns the number of `Event` values that `self` can hold.
    pub fn capacity(&self) -> usize {
        self.events.capacity()
    }

    /// Returns the number of `Event` values
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Returns `true` if `self` contains no `Event` values.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Returns an iterator over the `Event` values.
    pub fn iter(&self) -> Iter {
        Iter {
            inner: self.events.iter(),
        }
    }

    pub fn get(&self, idx: usize) -> Option<Event> {
        self.events.get(idx).map(|event| *event)
    }

    pub fn push(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn append(&mut self, events: &mut Events) {
        self.events.append(&mut events.events)
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// [`Events`] iterator.
#[derive(Debug, Clone)]
pub struct Iter<'a> {
    inner: std::slice::Iter<'a, Event>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        self.inner.next().map(|event| *event)
    }
}

impl<'a> IntoIterator for &'a Events {
    type Item = Event;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token(pub usize);
