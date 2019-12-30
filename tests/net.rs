use std::io::{self, Read, Write};
use std::str;
use std::time::Duration;
use std::thread;

use libsrt_rs::net::Builder;
use libsrt_rs::net::{Bind, Stream};
use libsrt_rs::net::{Poll, Token, Events, EventKind};

static MESSAGE: &str = "foo bar baz";

#[test]
fn net_sync_echo() {
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
fn net_async_echo() {
    let try_addr = "127.0.0.1:0".parse().unwrap();
    let server = Builder::new().nonblocking(true).bind(&try_addr).unwrap();
    let addr = server.local_addr().unwrap();

    let client = thread::spawn(move || {
        const TOKEN: Token = Token(0);

        let poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(2);

        let mut stream = Builder::new().nonblocking(true).connect(&addr)
            .unwrap();
        poll.register(&stream, TOKEN, EventKind::writable()).unwrap();
        poll.poll(&mut events, Some(Duration::from_millis(1000))).unwrap();
        assert!(events.iter().next().is_some());

        poll.reregister(&stream, TOKEN,
                        EventKind::readable() | EventKind::error())
            .unwrap();
        let msg_bytes = MESSAGE.as_bytes();
        let mut read_buf = [0; 2048];
        let mut nread = 0;
        let mut nsent = 0;
        'outer: while nread < msg_bytes.len() {
            events.clear();
            poll.poll(&mut events, Some(Duration::from_millis(100))).unwrap();

            for event in &events {
                assert_eq!(event.token(), TOKEN);
                assert!(! event.kind().is_error());

                if event.kind().is_readable() {
                    while nread < nsent {
                        match stream.read(&mut read_buf) {
                            Ok(0) => {
                                break 'outer;
                            }
                            Ok(ref len) => {
                                nread = nread + len;
                            }
                            Err(ref e)
                                if e.kind() == io::ErrorKind::WouldBlock =>
                            {
                                break;
                            }
                            Err(e) => panic!("{} failed with {:?}",
                                             stringify!($e), e),
                        }
                    }
                }
            }

            while nsent < msg_bytes.len() {
                match stream.write(&msg_bytes[nsent..]) {
                    Ok(len) => {
                        nsent += len;
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
                }
            }
        }

        assert_eq!(nread, 11);
        assert_eq!(MESSAGE, str::from_utf8(&read_buf[0..nread]).unwrap());
    });

    const SERVER_TOKEN: Token = Token(0);
    const CLIENT_TOKEN: Token = Token(1);

    let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(1);

    poll.register(&server, SERVER_TOKEN, EventKind::readable()).unwrap();
    let mut _stream: Option<Stream> = None;
    loop {
        events.clear();
        poll.poll(&mut events, None).unwrap();
        let event = events.iter().next().unwrap();
        assert_eq!(event.token(), SERVER_TOKEN);
        assert!(event.kind().is_readable());
        match server.accept() {
            Ok((s, _a)) => {
                _stream = Some(Builder::new()
                               .nonblocking(true)
                               .accept(s).unwrap());
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
        }
    }

    let mut stream = _stream.unwrap();
    poll.reregister(&stream, CLIENT_TOKEN,
                    EventKind::readable() | EventKind::error())
        .unwrap();

    let msg_len = MESSAGE.as_bytes().len();
    let mut buf = [0; 2048];
    let mut nread = 0;
    let mut nsent = 0;

    'outer: while nsent < msg_len {
        events.clear();
        poll.poll(&mut events, Some(Duration::from_millis(100))).unwrap();

        for event in &events {
            assert_eq!(event.token(), CLIENT_TOKEN);
            assert!(! event.kind().is_error());

            if event.kind().is_readable() {
                while nread < msg_len {
                    match stream.read(&mut buf) {
                        Ok(0) => {
                            break 'outer;
                        }
                        Ok(ref len) => {
                            nread = nread + len;
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => panic!("{} failed with {:?}",
                                         stringify!($e), e),
                    }
                }
            }
        }

        while nsent < nread {
            match stream.write(&buf[nsent..nread]) {
                Ok(len) => {
                    nsent += len;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
            }
        }
    }

    assert_eq!(nread, 11);
    assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());

    client.join().unwrap();
}
