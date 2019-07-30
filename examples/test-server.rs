use std::io::{self, Read};
use std::process;
use std::str;
use failure::{self as f, Error};

use libsrt_rs::std_srt::{Listener, Bind, InputStream};
use libsrt_rs::std_srt::{Poll, Token, EventKind, Events};

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    const LISTEN_TOKEN: Token = Token(0);
    const STREAM_TOKEN: Token = Token(1);

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 1 {
        return Err(f::err_msg("Usage: test-server IP:PORT"));
    }

    let addr = args[0].parse()?;
    let listen = Listener::bind(&addr)?;
    println!("listening on {}", listen.local_addr()?);

    let poll = Poll::new()?;

    // Register the listener
    poll.register(&listen, LISTEN_TOKEN, EventKind::READABLE | EventKind::ERROR)?;

    // Create storage for events
    let mut events = Events::with_capacity(2);

    let mut input_stream: Option<InputStream> = None;
    let mut i = 0;

    // The main event loop
    'outer: loop {
        events.clear();

        // Wait for events
        poll.poll(&mut events, None)?;

        for event in &events {
            match event.token() {
                LISTEN_TOKEN => {
                    println!("listen");
                    // Perform operations in a loop until `WouldBlock` is
                    // encountered.
                    loop {
                        match listen.accept() {
                            Ok((stream, peer_addr)) => {
                                println!("connection established from {}", peer_addr);
                                poll.deregister(&listen)?;

                                let is = stream.input_stream()?;
                                poll.register(&is, STREAM_TOKEN, EventKind::READABLE | EventKind::ERROR)?;
                                input_stream = Some(is);
                                break;
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                // Socket is not ready anymore, stop accepting
                                break;
                            }
                            Err(e) => return Err(e.into()), // Unexpected error
                        }
                    }
                }
                STREAM_TOKEN => {
                    let kind = event.kind();

                    if kind.is_error() {
                        println!("connection closed");
                        poll.deregister(input_stream.as_ref().unwrap())?;
                        break 'outer;
                    } else if kind.is_readable() {
                        let mut buf = [0; DEFAULT_BUF_SIZE];
                        loop {
                            print!("srt read {}...", i);
                            match input_stream.as_ref().unwrap().read(&mut buf) {
                                Ok(0) => {
                                    // XXX DO NOT WORK
                                    // Socket is closed, remove it
                                    println!("connection closed");
                                    poll.deregister(input_stream.as_ref().unwrap())?;
                                    break 'outer;
                                }
                                Ok(ref len) => {
                                    println!("Got message of length {} << {}",
                                             len,
                                             str::from_utf8(&buf[0..*len])?);
                                    i += 1;
                                }
                                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    // Socket is not ready anymore, stop reading
                                    println!("");
                                    break;

                                }
                                Err(e) => return Err(e.into()), // Unexpected error
                            }
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}
