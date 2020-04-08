#[macro_use]
extern crate log;

use failure::{self as f, Error};
use std::borrow::Cow;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::process;
use std::thread;
use std::time::Duration;

use libsrt_rs::net::Builder;
use libsrt_rs::net::Connect;
use libsrt_rs::net::{EventKind, Events, Poll, Token};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opts = match get_opts(&args) {
        Ok(res) => res,
        Err(err) => {
            eprintln!("Usage: {} IP:PORT", prog_name(&args[0]));
            eprintln!("{}", err);
            process::exit(1);
        }
    };

    if let Err(err) = run(opts.addr) {
        eprintln!("{}", err);
        process::exit(1);
    }
}

struct Options {
    addr: SocketAddr,
}

fn get_opts(args: &[String]) -> Result<Options, Error> {
    if args.len() < 2 {
        return Err(f::err_msg(format!("no target address")));
    }

    Ok(Options { addr: args[1].parse()? })
}

fn prog_name(path: &str) -> Cow<str> {
    Path::new(path).file_name().unwrap().to_string_lossy()
}

fn run(addr: impl Into<SocketAddr> + 'static) -> Result<(), Error> {
    env_logger::init()?;

    const TOKEN: Token = Token(0);

    let poll = Poll::new()?;
    let mut events = Events::with_capacity(2);

    let mut stream = Builder::new()
        .nonblocking(true)
        .connect(&addr.into())?;

    poll.register(&stream, TOKEN, EventKind::writable())?;
    poll.poll(&mut events, Some(Duration::from_millis(1000)))?;
    if events.iter().next().is_none() {
        return Err(io::Error::new(io::ErrorKind::TimedOut,
                                  "connection timeout").into());
    }
    info!("connection established to {}", stream.peer_addr()?);

    poll.reregister(&stream,
                    TOKEN, EventKind::writable() | EventKind::error())?;

    let message = format!("This message should be sent to the other side");
    'outer: for i in 0..100 {
        info!("write #{} {}", i, message);

        let mut _nsent = 0;
        loop {
            events.clear();

            poll.poll(&mut events, None)?;

            for event in &events {
                match event.token() {
                    TOKEN => {
                        if event.kind().is_error() {
                            info!("connection closed");
                            break 'outer;
                        }
                    }
                    _ => unreachable!(),
                }
            }

            match stream.write(&message.as_bytes()[_nsent..]) {
                Ok(len) => {
                    _nsent += len;
                    if _nsent == message.len() {
                        _nsent = 0;
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    // XXX To avoid the error message:
    // SRT:RcvQ:worker!!FATAL!!:SRT.c: CChannel reported ERROR DURING TRANSMISSION - IPE. INTERRUPTING worker anyway.
    poll.reregister(&stream, TOKEN, EventKind::error())?;
    events.clear();
    poll.poll(&mut events, Some(Duration::from_millis(1000)))?;

    poll.deregister(&stream)?;

    Ok(())
}
