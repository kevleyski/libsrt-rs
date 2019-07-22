use std::io::Read;
use std::process;
use std::str;
use failure::{self as f, Error};

use libsrt_rs::{SrtInListener, SrtCommon};

const DEFAULT_BUF_SIZE: usize = 8 * 1024;

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 1 {
        return Err(f::err_msg("Usage: test-server IP:PORT"));
    }

    let addr = args[0].parse()?;
    let sock = SrtInListener::bind(&addr)?;
    println!("listening on {}", sock.local_addr()?);

    let (mut peer_sock, peer_addr) = sock.accept()?;
    println!("connection established from {}", peer_addr);

    let mut buf = [0; DEFAULT_BUF_SIZE];
    for i in 0..100 {
        print!("read #{}... ", i);

        let len = peer_sock.read(&mut buf)?;

        println!("Got message of length {} << {}",
                 len,
                 str::from_utf8(&buf[0..len])?);
    }

    Ok(())
}
