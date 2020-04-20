#![allow(non_camel_case_types, unused_extern_crates, dead_code)] // XXX dead_code
use libc::{c_char, c_void, sockaddr};

pub use libc::c_int as int;
pub type SRTSOCKET = int;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub type SYSSOCKET = int;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("libsrt doesn't compile for this platform yet");

pub type UDPSOCKET = SYSSOCKET;

// This is a duplicate enum. Must be kept in sync with the original UDT enum for
// backward compatibility until all compat is destroyed.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SRT_SOCKOPT {
    SRTO_MSS = 0,        // the Maximum Transfer Unit
    SRTO_SNDSYN = 1,     // if sending is blocking
    SRTO_RCVSYN = 2,     // if receiving is blocking
    SRTO_ISN = 3, // Initial Sequence Number (valid only after srt_connect or srt_accept-ed sockets)
    SRTO_FC = 4,  // Flight flag size (window size)
    SRTO_SNDBUF = 5, // maximum buffer in sending queue
    SRTO_RCVBUF = 6, // UDT receiving buffer size
    SRTO_LINGER = 7, // waiting for unsent data when closing
    SRTO_UDP_SNDBUF = 8, // UDP sending buffer size
    SRTO_UDP_RCVBUF = 9, // UDP receiving buffer size
    // XXX Here space free for 2 options
    // after deprecated ones are removed
    SRTO_RENDEZVOUS = 12, // rendezvous connection mode
    SRTO_SNDTIMEO = 13,   // send() timeout
    SRTO_RCVTIMEO = 14,   // recv() timeout
    SRTO_REUSEADDR = 15,  // reuse an existing port or create a new one
    SRTO_MAXBW = 16,      // maximum bandwidth (bytes per second) that the connection can use
    SRTO_STATE = 17,      // current socket state, see UDTSTATUS, read only
    SRTO_EVENT = 18,      // current available events associated with the socket
    SRTO_SNDDATA = 19,    // size of data in the sending buffer
    SRTO_RCVDATA = 20,    // size of data available for recv
    SRTO_SENDER = 21, // Sender mode (independent of conn mode), for encryption, tsbpd handshake.
    SRTO_TSBPDMODE = 22, // Enable/Disable TsbPd. Enable -> Tx set origin timestamp, Rx deliver packet at origin time + delay
    SRTO_LATENCY = 23, // DEPRECATED. SET: to both SRTO_RCVLATENCY and SRTO_PEERLATENCY. GET: same as SRTO_RCVLATENCY.
    // SRTO_TSBPDDELAY = 23, // ALIAS: SRTO_LATENCY
    SRTO_INPUTBW = 24,      // Estimated input stream rate.
    SRTO_OHEADBW, // MaxBW ceiling based on % over input stream rate. Applies when UDT_MAXBW=0 (auto).
    SRTO_PASSPHRASE = 26, // Crypto PBKDF2 Passphrase size[0,10..64] 0:disable crypto
    SRTO_PBKEYLEN, // Crypto key len in bytes {16,24,32} Default: 16 (128-bit)
    SRTO_KMSTATE, // Key Material exchange status (UDT_SRTKmState)
    SRTO_IPTTL = 29, // IP Time To Live (passthru for system sockopt IPPROTO_IP/IP_TTL)
    SRTO_IPTOS,   // IP Type of Service (passthru for system sockopt IPPROTO_IP/IP_TOS)
    SRTO_TLPKTDROP = 31, // Enable receiver pkt drop
    SRTO_SNDDROPDELAY = 32, // Extra delay towards latency for sender TLPKTDROP decision (-1 to off)
    SRTO_NAKREPORT = 33, // Enable receiver to send periodic NAK reports
    SRTO_VERSION = 34, // Local SRT Version
    SRTO_PEERVERSION, // Peer SRT Version (from SRT Handshake)
    SRTO_CONNTIMEO = 36, // Connect timeout in msec. Ccaller default: 3000, rendezvous (x 10)
    // deprecated: SRTO_TWOWAYDATA, SRTO_SNDPBKEYLEN, SRTO_RCVPBKEYLEN (@c below)
    _DEPRECATED_SRTO_SNDPBKEYLEN = 38, // (needed to use inside the code without generating -Wswitch)
    //
    SRTO_SNDKMSTATE = 40, // (GET) the current state of the encryption at the peer side
    SRTO_RCVKMSTATE,      // (GET) the current state of the encryption at the agent side
    SRTO_LOSSMAXTTL, // Maximum possible packet reorder tolerance (number of packets to receive after loss to send lossreport)
    SRTO_RCVLATENCY, // TsbPd receiver delay (mSec) to absorb burst of missed packet retransmission
    SRTO_PEERLATENCY, // Minimum value of the TsbPd receiver delay (mSec) for the opposite side (peer)
    SRTO_MINVERSION, // Minimum SRT version needed for the peer (peers with less version will get connection reject)
    SRTO_STREAMID,   // A string set to a socket and passed to the listener's accepted socket
    SRTO_SMOOTHER,   // Smoother selection (congestion control algorithm)
    SRTO_MESSAGEAPI, // In File mode, use message API (portions of data with boundaries)
    SRTO_PAYLOADSIZE, // Maximum payload size sent in one UDP packet (0 if unlimited)
    SRTO_TRANSTYPE,  // Transmission type (set of options required for given transmission type)
    SRTO_KMREFRESHRATE, // After sending how many packets the encryption key should be flipped to the new key
    SRTO_KMPREANNOUNCE, // How many packets before key flip the new key is annnounced and after key flip the old one decommissioned
    SRTO_STRICTENC, // Connection to be rejected or quickly broken when one side encryption set or bad password
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SRT_TRANSTYPE {
    SRTT_LIVE,
    SRTT_FILE,
    SRTT_INVALID,
}

pub const SRT_INVALID_SOCK: SRTSOCKET = -1;
pub const SRT_ERROR: int = -1;

// library initialization
extern "C" {
    pub fn srt_startup() -> int;
    pub fn srt_cleanup() -> int;
}

// socket operations
extern "C" {
    pub fn srt_socket(af: int, typ: int, protocol: int) -> SRTSOCKET;
    pub fn srt_create_socket() -> SRTSOCKET;
    pub fn srt_bind(u: SRTSOCKET, name: *const sockaddr, namelen: int) -> int;
    pub fn srt_bind_peerof(u: SRTSOCKET, udpsock: UDPSOCKET) -> int;
    pub fn srt_listen(u: SRTSOCKET, backlog: int) -> int;
    pub fn srt_accept(
        u: SRTSOCKET,
        addr: *mut sockaddr,
        addrlen: *mut int
    ) -> SRTSOCKET;
    pub fn srt_connect(
        u: SRTSOCKET,
        name: *const sockaddr,
        namelen: int
    ) -> int;
    pub fn srt_connect_debug(
        u: SRTSOCKET,
        name: *const sockaddr,
        namelen: int,
        forced_isn: int,
    ) -> int;
    pub fn srt_rendezvous(
        u: SRTSOCKET,
        local_name: *const sockaddr,
        local_namelen: int,
        remote_name: *const sockaddr,
        remote_namelen: int,
    ) -> int;
    pub fn srt_close(u: SRTSOCKET) -> int;
    pub fn srt_getpeername(
        u: SRTSOCKET,
        name: *mut sockaddr,
        namelen: *mut int
    ) -> int;
    pub fn srt_getsockname(
        u: SRTSOCKET,
        name: *mut sockaddr,
        namelen: *mut int
    ) -> int;
    pub fn srt_getsockopt(
        u: SRTSOCKET,
        level: int, /*ignored*/
        optname: SRT_SOCKOPT,
        optval: *mut c_void,
        optlen: *mut int,
    ) -> int;
    pub fn srt_setsockopt(
        u: SRTSOCKET,
        level: int, /*ignored*/
        optname: SRT_SOCKOPT,
        optval: *const c_void,
        optlen: int,
    ) -> int;
    pub fn srt_getsockflag(
        u: SRTSOCKET,
        opt: SRT_SOCKOPT,
        optval: *mut c_void,
        optlen: *mut int,
    ) -> int;
    pub fn srt_setsockflag(
        u: SRTSOCKET,
        opt: SRT_SOCKOPT,
        optval: *const c_void,
        optlen: int,
    ) -> int;
}

// XXX Note that the srctime functionality doesn't work yet and needs fixing.
#[repr(C)]
pub struct SRT_MSGCTRL {
    flags: int,    // Left for future
    msgttl: int,   // TTL for a message, default -1 (delivered always)
    inorder: int, // Whether a message is allowed to supersede partially lost one. Unused in stream and live mode.
    boundary: int, //0:mid pkt, 1(01b):end of frame, 2(11b):complete frame, 3(10b): start of frame
    srctime: u64, // source timestamp (usec), 0: use internal time
    pktseq: i32,  // sequence number of the first packet in received message (unused for sending)
    msgno: i32,   // message number (output value for both sending and receiving)
}

// You are free to use either of these two methods to set SRT_MSGCTRL object
// to default values: either call srt_msgctrl_init(&obj) or obj = srt_msgctrl_default.
extern "C" {
    pub fn srt_msgctrl_init(mctrl: *mut SRT_MSGCTRL);
    pub static srt_msgctrl_default: SRT_MSGCTRL;
}

// enum CodeMajor
pub const MJ_UNKNOWN   : int = -1;
pub const MJ_SUCCESS   : int =  0;
pub const MJ_SETUP     : int =  1;
pub const MJ_CONNECTION: int =  2;
pub const MJ_SYSTEMRES : int =  3;
pub const MJ_FILESYSTEM: int =  4;
pub const MJ_NOTSUP    : int =  5;
pub const MJ_AGAIN     : int =  6;
pub const MJ_PEERERROR : int =  7;

// enum CodeMinor
// MJ_SETUP
pub const MN_NONE           : int =  0;
pub const MN_TIMEOUT        : int =  1;
pub const MN_REJECTED       : int =  2;
pub const MN_NORES          : int =  3;
pub const MN_SECURITY       : int =  4;
// MJ_CONNECTION
pub const MN_CONNLOST       : int =  1;
pub const MN_NOCONN         : int =  2;
// MJ_SYSTEMRES
pub const MN_THREAD         : int =  1;
pub const MN_MEMORY         : int =  2;
// MJ_FILESYSTEM
pub const MN_SEEKGFAIL      : int =  1;
pub const MN_READFAIL       : int =  2;
pub const MN_SEEKPFAIL      : int =  3;
pub const MN_WRITEFAIL      : int =  4;
// MJ_NOTSUP
pub const MN_ISBOUND        : int =  1;
pub const MN_ISCONNECTED    : int =  2;
pub const MN_INVAL          : int =  3;
pub const MN_SIDINVAL       : int =  4;
pub const MN_ISUNBOUND      : int =  5;
pub const MN_NOLISTEN       : int =  6;
pub const MN_ISRENDEZVOUS   : int =  7;
pub const MN_ISRENDUNBOUND  : int =  8;
pub const MN_INVALMSGAPI    : int =  9;
pub const MN_INVALBUFFERAPI : int = 10;
pub const MN_BUSY           : int = 11;
pub const MN_XSIZE          : int = 12;
pub const MN_EIDINVAL       : int = 13;
// MJ_AGAIN
pub const MN_WRAVAIL        : int =  1;
pub const MN_RDAVAIL        : int =  2;
pub const MN_XMTIMEOUT      : int =  3;
pub const MN_CONGESTION     : int =  4;

// enum SRT_ERRNO
pub const SRT_ETIMEOUT       : int = 6003; // XXX MJ_AGAIN * 1000 + XMTIMEOUT

// The send/receive functions.
extern "C" {
    pub fn srt_sendmsg(u: SRTSOCKET, buf: *const c_char, len: int) -> int;
    pub fn srt_recvmsg(u: SRTSOCKET, buf: *mut c_char, len: int) -> int;
}

// last error detection
extern "C" {
    pub fn srt_getlasterror(errno_loc: *mut int) -> int;
    pub fn srt_strerror(code: int, errnoval: int) -> *const c_char;
}

// XXX
pub fn srt_errorkind(errcode: int) -> std::io::ErrorKind {
    let major = errcode / 1000;
    let minor = errcode % 1000;
    match major {
        MJ_SETUP => {
            match minor {
                MN_TIMEOUT => std::io::ErrorKind::TimedOut,
                MN_REJECTED => std::io::ErrorKind::ConnectionRefused,
                _ => std::io::ErrorKind::Other,
            }
        },
        MJ_CONNECTION => {
            match minor {
                MN_CONNLOST => std::io::ErrorKind::BrokenPipe,
                MN_NOCONN => std::io::ErrorKind::NotConnected,
                _ => std::io::ErrorKind::Other,
            }
        },
        MJ_SYSTEMRES => std::io::ErrorKind::Other,
        MJ_FILESYSTEM => std::io::ErrorKind::Other,
        MJ_NOTSUP => {
            match minor {
                MN_BUSY => std::io::ErrorKind::AlreadyExists,
                _ => std::io::ErrorKind::Other,
            }
        },
        MJ_AGAIN => {
            match minor {
                MN_WRAVAIL => std::io::ErrorKind::WouldBlock,
                MN_RDAVAIL => std::io::ErrorKind::WouldBlock,
                MN_XMTIMEOUT => std::io::ErrorKind::TimedOut,
                _ => std::io::ErrorKind::WouldBlock, // XXX
            }
        },
        _ => std::io::ErrorKind::Other,
    }
}

// Values returned by srt_getsockstate()
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SRT_SOCKSTATUS {
    SRTS_INIT = 1,
    SRTS_OPENED,
    SRTS_LISTENING,
    SRTS_CONNECTING,
    SRTS_CONNECTED,
    SRTS_BROKEN,
    SRTS_CLOSING,
    SRTS_CLOSED,
    SRTS_NONEXIST,
}

// Socket Status (for problem tracking)
extern "C" {
    pub fn srt_getsockstate(u: SRTSOCKET) -> SRT_SOCKSTATUS;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SRT_EPOLL_OPT {
    SRT_EPOLL_OPT_NONE = 0x0, // fallback
    SRT_EPOLL_IN = 0x1,
    SRT_EPOLL_OUT = 0x4,
    SRT_EPOLL_ERR = 0x8,
}

extern "C" {
    pub fn srt_epoll_create() -> int;
    pub fn srt_epoll_add_usock(
        epid: int,
        u: SRTSOCKET,
        events: *const int
    ) -> int;
    pub fn srt_epoll_add_ssock(
        epid: int,
        s: SYSSOCKET,
        events: *const int
    ) -> int;
    pub fn srt_epoll_remove_usock(epid: int, u: SRTSOCKET) -> int;
    pub fn srt_epoll_remove_ssock(epid: int, s: SYSSOCKET) -> int;
    pub fn srt_epoll_update_usock(
        epid: int,
        u: SRTSOCKET,
        events: *const int
    ) -> int;
    pub fn srt_epoll_update_ssock(
        epid: int,
        s: SYSSOCKET,
        events: *const int
    ) -> int;
    pub fn srt_epoll_wait(
        epid: int,
        read_fds: *mut SRTSOCKET,
        read_num: *mut int,
        write_fds: *mut SRTSOCKET,
        write_num: *mut int,
        timeout_ms: i64,
        lr_fds: *mut SYSSOCKET,
        lr_num: *mut int,
        lw_fds: *mut SYSSOCKET,
        lw_num: *mut int,
    ) -> int;
    pub fn srt_epoll_release(epid: int) -> int;
}
