use std::io::{self, Write};
use std::process;
use std::time::Duration;
use failure::{self as f, Error};

use libsrt_rs::std_srt::{Stream, Connect};
use libsrt_rs::std_srt::{Poll, Token, EventKind, Events};

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    const TOKEN: Token = Token(0);

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 1 {
        return Err(f::err_msg("Usage: test-client IP:PORT"));
    }

    let poll = Poll::new()?;
    let mut events = Events::with_capacity(2);

    let addr = args[0].parse()?;
    let stream = Stream::connect(&addr)?;

    poll.register(&stream, TOKEN, EventKind::writable())?;
    poll.poll(&mut events, Some(Duration::from_millis(1000)))?;
    if events.iter().next().is_none() {
        return Err(io::Error::new(io::ErrorKind::TimedOut, "connection timeout").into());
    }
    poll.deregister(&stream)?;

    println!("connection established to {}", stream.peer_addr()?);
    let mut output = stream.output_stream()?;

    poll.reregister(&output, TOKEN, EventKind::writable() | EventKind::error())?;

    let message = "This message should be sent to the other side".to_string();
    'outer: for i in 0..100 {
        println!("write #{} {}", i, message);

        let mut _nsent = 0;
        loop {
            events.clear();

            poll.poll(&mut events, None)?;

            for event in &events {
                match event.token() {
                    TOKEN => {
                        if event.kind().is_error() {
                            println!("connection closed");
                            break 'outer;
                        }
                    }
                    _ => unreachable!(),
                }
            }

            match output.write(&message.as_bytes()[_nsent..]) {
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
    }

    // XXX To avoid the error message:
    // SRT:RcvQ:worker!!FATAL!!:SRT.c: CChannel reported ERROR DURING TRANSMISSION - IPE. INTERRUPTING worker anyway.
    poll.reregister(&output, TOKEN, EventKind::error())?;
    events.clear();
    poll.poll(&mut events, Some(Duration::from_millis(1000)))?;

    poll.deregister(&output)?;

    Ok(())
}
