mod error;
mod ffi;
mod poll;
mod socket;

pub fn init() {
    use std::sync::{Once, ONCE_INIT};

    static INIT: Once = ONCE_INIT;

    INIT.call_once(|| unsafe {
        crate::ffi::srt_startup();
    })

    // Note that we explicitly don't schedule a call to
    // `srt_cleanup`. The documentation for that function says
    //
    // > You must not call it when any other thread in the program (i.e. a
    // > thread sharing the same memory) is running. This doesn't just mean
    // > no other thread that is using libsrt.
    //
    // We can't ever be sure of that, so unfortunately we can't call the
    // function.
}

pub use libc::c_int as int;
pub use poll::{Event, EventKind, Events, Poll, Token};
pub use socket::{Socket, SRT_SOCKSTATUS as SOCKSTATUS};
