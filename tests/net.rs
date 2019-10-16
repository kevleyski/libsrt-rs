use std::io::{self, Read, Write};
use std::str;
use std::time::Duration;
use std::thread;

use libsrt_rs::net::Builder;
use libsrt_rs::net::{Bind, Stream};
use libsrt_rs::net::{Poll, Token, Events, EventKind};

static MESSAGE: &str = "foo bar baz";

#[test]
fn sync_echo() {
    let try_addr = "127.0.0.1:0".parse().unwrap();
    let server = Builder::new().bind(&try_addr).unwrap();
    let addr = server.local_addr().unwrap();

    let client = thread::spawn(move || {
        let mut stream = Builder::new()
            .connect(&addr).unwrap();
        stream.write(MESSAGE.as_bytes()).unwrap();
        thread::sleep(Duration::from_millis(200));

        let poll = Poll::new().unwrap();
        const TOKEN: Token = Token(0);
        poll.register(&stream, TOKEN, EventKind::readable()).unwrap();

        let mut events = Events::with_capacity(1);
        let mut buf = [0; 2048];
        poll.poll(&mut events, None).unwrap();
        let event = events.iter().next().unwrap();
        assert_eq!(event.token(), TOKEN);
        assert!(event.kind().is_readable());

        let nread = stream.read(&mut buf).unwrap();
        assert_eq!(nread, 11);
        assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());
    });

    let (mut stream, _peer_addr) = server.accept().unwrap();

    let poll = Poll::new().unwrap();
    const TOKEN: Token = Token(0);
    poll.register(&stream, TOKEN, EventKind::readable()).unwrap();

    let mut events = Events::with_capacity(1);
    let mut buf = [0; 2048];
    poll.poll(&mut events, None).unwrap();
    let event = events.iter().next().unwrap();
    assert_eq!(event.token(), TOKEN);
    assert!(event.kind().is_readable());

    let nread = stream.read(&mut buf).unwrap();
    assert_eq!(nread, 11);
    assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());

    stream.write(&buf[0..nread]).unwrap();

    client.join().unwrap();
}

#[test]
fn async_echo() {
    let try_addr = "127.0.0.1:0".parse().unwrap();
    let server = Builder::new().nonblocking(true).bind(&try_addr).unwrap();
    let addr = server.local_addr().unwrap();

    let client = thread::spawn(move || {
        const TOKEN: Token = Token(0);

        let poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(1);

        let mut stream = Builder::new().nonblocking(true).connect(&addr).unwrap();
        poll.register(&stream, TOKEN, EventKind::writable()).unwrap();
        poll.poll(&mut events, Some(Duration::from_millis(1000))).unwrap();
        assert!(events.iter().next().is_some());

        poll.reregister(&stream, TOKEN, EventKind::writable() | EventKind::error()).unwrap();
        let msg_bytes = MESSAGE.as_bytes();
        let mut nsent = 0;
        loop {
            events.clear();
            poll.poll(&mut events, None).unwrap();
            let event = events.iter().next().unwrap();
            assert_eq!(event.token(), TOKEN);
            assert!(! event.kind().is_error());

            match stream.write(&msg_bytes[nsent..]) {
                Ok(len) => {
                    nsent += len;
                    if nsent == msg_bytes.len() {
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
            }
        }

        let mut buf = [0; 2048];
        let mut nread = 0;

        poll.reregister(&stream, TOKEN, EventKind::readable()).unwrap();
        loop {
            events.clear();
            poll.poll(&mut events, None).unwrap();
            let event = events.iter().next().unwrap();
            assert_eq!(event.token(), TOKEN);
            assert!(event.kind().is_readable());
            match stream.read(&mut buf) {
                Ok(0) => {
                    break;
                }
                Ok(ref len) => {
                    nread = nread + len;
                    if nread == msg_bytes.len() {
                        break;
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
            }
        }

        assert_eq!(nread, 11);
        assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());
    });

    const SERVER_TOKEN: Token = Token(0);
    const CLIENT_TOKEN: Token = Token(1);

    let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1);

    poll.register(&server, SERVER_TOKEN, EventKind::readable()).unwrap();
    let mut stream: Option<Stream> = None;
    loop {
        events.clear();
        poll.poll(&mut events, None).unwrap();
        let event = events.iter().next().unwrap();
        assert_eq!(event.token(), SERVER_TOKEN);
        assert!(event.kind().is_readable());
        match server.accept() {
            Ok((s, _a)) => {
                stream = Some(Builder::new().nonblocking(true).accept(s).unwrap());
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
        }
    }

    let mut stream = stream.unwrap();
    let mut buf = [0; 2048];
    let mut nread = 0;

    poll.register(&stream, CLIENT_TOKEN, EventKind::readable()).unwrap();
    loop {
        events.clear();
        poll.poll(&mut events, None).unwrap();
        let event = events.iter().next().unwrap();
        assert_eq!(event.token(), CLIENT_TOKEN);
        assert!(event.kind().is_readable());
        match stream.read(&mut buf) {
            Ok(0) => {
                break;
            }
            Ok(ref len) => {
                nread = nread + len;
                if nread == MESSAGE.as_bytes().len() {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
        }
    }

    assert_eq!(nread, 11);
    assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());

    poll.reregister(&stream, CLIENT_TOKEN, EventKind::writable() | EventKind::error()).unwrap();
    let mut nsent = 0;
    loop {
        events.clear();
        poll.poll(&mut events, None).unwrap();
        let event = events.iter().next().unwrap();
        assert_eq!(event.token(), CLIENT_TOKEN);
        assert!(! event.kind().is_error());

        match stream.write(&buf[nsent..nread]) {
            Ok(len) => {
                nsent += len;
                if nsent == nread {
                    break;
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
        }
    }

    client.join().unwrap();
}
