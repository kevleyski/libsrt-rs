use std::io::{self, Write};
use std::process;
use std::thread;
use std::time::Duration;
use failure::{self as f, Error};

use libsrt_rs::std_srt::{Stream, Connect};

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 1 {
        return Err(f::err_msg("Usage: test-client IP:PORT"));
    }

    let addr = args[0].parse()?;
    let stream = Stream::connect(&addr)?;
    println!("connection established to {}", stream.peer_addr()?);
    let mut output = stream.output_stream()?;

    let message = "This message should be sent to the other side".to_string();
    for i in 0..100 {
        println!("write #{} {}", i, message);

        let mut sent_bytes = 0;
        loop {
            match output.write(&message.as_bytes()[sent_bytes..]) {
                Ok(len) => {
                    sent_bytes += len;
                    if sent_bytes == message.len() {
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(1));
                    continue;
                },
                Err(e) => return Err(e.into()),
            }
        }
    }

    Ok(())
}
