use std::borrow::Cow;
use std::error;
use std::ffi::CStr;
use std::fmt;
use std::io;

use libc::c_int as int;

use crate::ffi::{self, SRTSOCKET};

// copied from libstd::sys::cvt
#[doc(hidden)]
pub trait IsMinusOne {
    fn is_minus_one(&self) -> bool;
}

macro_rules! impl_is_minus_one {
    ($($t:ident)*) => ($(impl IsMinusOne for $t {
        fn is_minus_one(&self) -> bool {
            *self == -1
        }
    })*)
}

impl_is_minus_one! { SRTSOCKET }

pub fn cvt<T: IsMinusOne>(t: T) -> io::Result<T> {
    if t.is_minus_one() {
        let err = Error::last_error();
        Err(io::Error::new(err.kind(), err))
    } else {
        Ok(t)
    }
}

pub struct Error<'a> {
    errcode: int,
    errstr: Cow<'a, str>,
}

impl<'a> Error<'a> {
    pub fn new<S>(errcode: int, errstr: S) -> Error<'a>
    where
        S: Into<Cow<'a, str>>,
    {
        Error {
            errcode,
            errstr: errstr.into(),
        }
    }

    pub fn last_error() -> Error<'a> {
        let mut errno: int = 0;
        let errcode = unsafe { ffi::srt_getlasterror(&mut errno) };
        let errstr = unsafe { CStr::from_ptr(ffi::srt_strerror(errcode, errno)).to_string_lossy() };
        Error::new(errcode, errstr)
    }

    pub fn kind(&self) -> io::ErrorKind {
        ffi::srt_errorkind(self.errcode)
    }

    pub fn message(&self) -> &str {
        self.errstr.as_ref()
    }
}

impl<'a> fmt::Debug for Error<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl<'a> fmt::Display for Error<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.message())
    }
}

impl<'a> error::Error for Error<'a> {
    fn description(&self) -> &str {
        self.errstr.as_ref()
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}
