#[macro_use]
extern crate log;

use std::borrow::Cow;
use std::io::{self, Read};
use std::net::SocketAddr;
use std::path::Path;
use std::process;
use std::str;

use bytes::{BytesMut, BufMut};
use failure::{self as f, Error};
use slab::Slab;

use libsrt_rs::net::Builder;
use libsrt_rs::net::{Bind, Connect, Listener, Stream};
use libsrt_rs::net::{EventKind, Events, Poll, Token};

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

const MAX_CONNECTIONS: usize = 1024;

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

struct Connection {
    sock: Stream,
    peer_addr: SocketAddr,
}

fn run(addr: impl Into<SocketAddr> + 'static) -> Result<(), Error> {
    env_logger::init()?;

    const LISTEN_TOKEN: Token = Token(MAX_CONNECTIONS);

    // let addr = args[1].parse()?;
    let listener = Builder::new()
        .nonblocking(true)
        .bind(&addr.into())?;
    info!("listening on {}", listener.local_addr()?);

    let poll = Poll::new()?;

    // Register the listener
    poll.register(&listener, LISTEN_TOKEN, EventKind::readable())?;

    // Create storage for events
    let mut events = Events::with_capacity(2);

    // Used to store the connections
    let mut connections = Slab::with_capacity(MAX_CONNECTIONS);

    // The main event loop
    loop {
        events.clear();

        // Wait for events
        poll.poll(&mut events, None)?;

        for event in &events {
            match event.token() {
                LISTEN_TOKEN => {
                    accept(&listener, &mut connections, &poll)?;
                }
                Token(index) => {
                    let kind = event.kind();

                    if kind.is_error() {
                        info!(
                            "srt read from {}...connection closed",
                            connections.get(index).unwrap().peer_addr
                        );
                        poll.deregister(&connections.get_mut(index).unwrap()
                                        .sock)?;
                        connections.remove(index);
                    } else if kind.is_readable() {
                        match read(&mut connections, index) {
                            Ok(msg) => {
                                info!(
                                    "srt read from {}...{}",
                                    connections.get(index).unwrap()
                                        .sock.peer_addr().unwrap(),
                                    msg
                                );
                            }
                            Err(e) => {
                                error!("{}", e);
                                poll.deregister(&connections
                                                .get_mut(index).unwrap().sock)?;
                                connections.remove(index);
                            }
                        }
                    } // kind.is...
                } // Token(index)
            } // event.token()
        } // for event
    }
}

fn accept(
    listener: &Listener,
    connections: &mut Slab<Connection>,
    poll: &Poll
) -> Result<(), Error> {
    trace!("listener");
    // Perform operations in a loop until `WouldBlock` is
    // encountered.
    loop {
        match listener.accept() {
            Ok((stream, peer_addr)) => {
                if connections.len() < connections.capacity() {
                    info!("connection established from {}", peer_addr);
                    let stream = Builder::new()
                        .nonblocking(true)
                        .accept(stream)?;
                    let index = connections.insert(Connection {
                        sock: stream,
                        peer_addr: peer_addr,
                    });
                    poll.register(
                        &connections.get_mut(index).unwrap().sock,
                        Token(index),
                        EventKind::readable() | EventKind::error(),
                    )?;
                } else {
                    error!("max clients exceeded");
                    // return Err(f::err_msg(format!("max clients exceeded")));
                    return Ok(())
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Socket is not ready anymore, stop accepting
                return Ok(())
            }
            Err(e) => return Err(e.into()), // Unexpected error
        }
    }
}

fn read(
    connections: &mut Slab<Connection>,
    index: usize,
) -> Result<String, Error> {
    let mut buf = BytesMut::with_capacity(DEFAULT_BUF_SIZE);
    loop {
        debug!("srt read from {}...",
              connections.get(index).unwrap().sock.peer_addr().unwrap()
        );
        let mut tmp_buf = [0; DEFAULT_BUF_SIZE];
        match connections.get_mut(index).unwrap().sock.read(&mut tmp_buf) {
            Ok(0) => {
                // XXX DO NOT WORK
                // Socket is closed
                debug!("closed");
                break;
            }
            Ok(ref len) => {
                buf.put(&tmp_buf[0..*len]);
                debug!("got message of length {} << {}",
                      len,
                      str::from_utf8(&tmp_buf[0..*len])?
                );
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Socket is not ready anymore, stop reading
                debug!("not ready");
                break;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    Ok(String::from_utf8(buf.to_vec())?)
}
