use std::borrow::Cow;
use std::error;
use std::ffi::CStr;
use std::fmt;
use std::io;

use libc::c_int;

use crate::ffi::{self as srtffi, SRTSOCKET};

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
    errcode: c_int,
    errstr: Cow<'a, str>,
}

impl<'a> Error<'a> {
    pub fn last_error() -> Error<'a> {
        unsafe {
            let mut errno: c_int = 0;
            let errcode = srtffi::srt_getlasterror(&mut errno);
            let errstr = CStr::from_ptr(srtffi::srt_strerror(errcode, errno)).to_string_lossy();
            Error { errcode, errstr }
        }
    }

    pub fn kind(&self) -> io::ErrorKind {
        srtffi::srt_errorkind(self.errcode)
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
