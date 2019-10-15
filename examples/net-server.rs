use std::io::{self, Read};
use std::net::SocketAddr;
use std::process;
use std::str;

use slab::Slab;

use failure::{self as f, Error};

use libsrt_rs::net::Builder;
use libsrt_rs::net::{Bind, Connect, Listener, InputStream};
use libsrt_rs::net::{EventKind, Events, Poll, Token};

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

const MAX_CONNECTIONS: usize = 1024;

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

struct Connection {
    sock: InputStream,
    peer_addr: SocketAddr,
}

fn run() -> Result<(), Error> {
    const LISTEN_TOKEN: Token = Token(MAX_CONNECTIONS);

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 1 {
        return Err(f::err_msg("Usage: test-server IP:PORT"));
    }

    let addr = args[0].parse()?;
    let listener = Builder::new()
        .nonblocking(true)
        .bind(&addr)?;
    println!("listening on {}", listener.local_addr()?);

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
                        println!(
                            "srt read from {}...connection closed",
                            connections.get(index).unwrap().peer_addr
                        );
                        poll.deregister(&connections.get_mut(index).unwrap().sock)?;
                        connections.remove(index);
                    } else if kind.is_readable() {
                        read(&mut connections, index, &poll)?;
                    }
                }
            }
        }
    }
}

fn accept(listener: &Listener, connections: &mut Slab<Connection>, poll: &Poll) -> Result<(), Error> {
    println!("listener");
    // Perform operations in a loop until `WouldBlock` is
    // encountered.
    loop {
        match listener.accept() {
            Ok((stream, peer_addr)) => {
                println!("connection established from {}", peer_addr);
                let index = connections.insert(Connection {
                    sock: stream.input_stream()?,
                    peer_addr: peer_addr,
                });
                poll.register(
                    &connections.get_mut(index).unwrap().sock,
                    Token(index),
                    EventKind::readable() | EventKind::error(),
                )?;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Socket is not ready anymore, stop accepting
                return Ok(())
            }
            Err(e) => return Err(e.into()), // Unexpected error
        }
    }
}

fn read(connections: &mut Slab<Connection>, index: usize, poll: &Poll) -> Result<usize, Error> {
    let mut buf = [0; DEFAULT_BUF_SIZE];
    let mut tot_len = 0;
    loop {
        print!(
            "srt read from {}...",
            connections.get(index).unwrap().sock.peer_addr().unwrap()
        );
        match connections.get_mut(index).unwrap().sock.read(&mut buf) {
            Ok(0) => {
                // XXX DO NOT WORK
                // Socket is closed, remove it
                println!("connection closed");
                poll.deregister(&connections.get_mut(index).unwrap().sock)?;
                connections.remove(index);
                break;
            }
            Ok(ref len) => {
                tot_len = tot_len + len;
                println!(
                    "got message of length {} << {}",
                    len,
                    str::from_utf8(&buf[0..*len])?
                );
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Socket is not ready anymore, stop reading
                println!("not ready");
                break;
            }
            Err(e) => {
                println!("error: {}", e);
                poll.deregister(&connections.get_mut(index).unwrap().sock)?;
                connections.remove(index);
                return Err(e.into());
            }
        }
    }

    Ok(tot_len)
}
