mod error;
mod ffi;
mod poll;
mod socket;

extern "C" fn cleanup() {
    unsafe {
        crate::ffi::srt_cleanup();
    }
}

pub fn init() {
    use std::sync::Once;

    static INIT: Once = Once::new();

    INIT.call_once(|| unsafe {
        crate::ffi::srt_startup();
        libc::atexit(cleanup);
    })
}

pub use libc::c_int as int;
pub use poll::{Event, EventKind, Events, Poll, Token};
pub use socket::{Socket, SRT_SOCKSTATUS as SOCKSTATUS};
