use std::io::Write;
use std::process;
use std::thread;
use std::time::Duration;
use failure::{self as f, Error};

use libsrt_rs::SrtStream;

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
    let mut sock = SrtStream::connect(&addr)?;
    println!("connection established to {}", sock.peer_addr()?);

    let message = "This message should be sent to the other side".to_string();
    for i in 0..100 {
        println!("write #{} {}", i, message);

        sock.write(message.as_bytes())?;

        thread::sleep(Duration::from_millis(1));
    }

    Ok(())
}
