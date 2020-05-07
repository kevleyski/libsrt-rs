use std::{
    io::{self, Read, Write},
    str,
    time::Duration,
    thread,
};
use libsrt_rs::net::{
    Builder,
    Bind,
    Poll, Token, Events, EventKind,
};

static MESSAGE: &str = "hello srt-net";

#[test]
fn net_sync_echo() {
    let try_addr = "127.0.0.1:0".parse().unwrap();

    let server = Builder::new().bind(&try_addr).unwrap();
    let addr = server.local_addr().unwrap();

    let mut client = Builder::new().connect(&addr).unwrap();

    let server_thread = thread::spawn(move || {
        let (mut peer, _peer_addr) = server.accept().unwrap();

        let mut buf = [0; 2048];
        let nread = peer.read(&mut buf).unwrap();
        assert_eq!(nread, 13);
        assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());

        peer.write(&buf[0..nread]).unwrap();
        thread::sleep(Duration::from_millis(500)); // XXX
    });

    client.write(MESSAGE.as_bytes()).unwrap();

    let mut buf = [0; 2048];
    let nread = client.read(&mut buf).unwrap();
    assert_eq!(nread, 13);
    assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());

    server_thread.join().unwrap();
}

#[test]
fn net_async_echo() {
    let try_addr = "127.0.0.1:0".parse().unwrap();

    let server = Builder::new().nonblocking(true).bind(&try_addr).unwrap();
    let server_poll = Poll::new().unwrap();

    const SERVER_TOKEN: Token = Token(0);
    server_poll.register(&server, SERVER_TOKEN,
                         EventKind::readable() | EventKind::error()).unwrap();

    let addr = server.local_addr().unwrap();

    let server_thread = thread::spawn(move || {
        let mut events = Events::with_capacity(2);
        let n = server_poll.poll(&mut events,
                                 Some(Duration::from_millis(1000))).unwrap();
        assert_eq!(n, 1);
        let event = events.iter().next().unwrap();
        assert_eq!(event.token(), SERVER_TOKEN);
        assert!(event.kind().is_readable());

        let mut peer = Builder::new()
            .nonblocking(true)
            .accept(server.accept().unwrap().0)
            .unwrap();
        const PEER_TOKEN: Token = Token(1);
        server_poll.reregister(&peer, PEER_TOKEN,
                               EventKind::readable() | EventKind::error())
            .unwrap();

        let msg_len = MESSAGE.as_bytes().len();
        let mut buf = [0; 2048];
        let mut nread = 0;
        let mut nsent = 0;

        'outer: while nsent < msg_len {
            events.clear();
            server_poll.poll(&mut events,
                             Some(Duration::from_millis(100))).unwrap();

            for event in &events {
                assert_eq!(event.token(), PEER_TOKEN);
                assert!(! event.kind().is_error());

                if event.kind().is_readable() {
                    while nread < msg_len {
                        match peer.read(&mut buf) {
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

            while nsent < nread {
                match peer.write(&buf[nsent..nread]) {
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

        assert_eq!(nread, 13);
        assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());
    });


    let mut client = Builder::new().nonblocking(true).connect(&addr).unwrap();
    let client_poll = Poll::new().unwrap();

    const CLIENT_TOKEN: Token = Token(2);
    client_poll.register(&client, CLIENT_TOKEN,
                         EventKind::writable() | EventKind::error()).unwrap();

    let mut events = Events::with_capacity(2);
    client_poll.poll(&mut events, Some(Duration::from_millis(1000))).unwrap();
    assert!(events.iter().next().is_some());

    client_poll.reregister(&client, CLIENT_TOKEN,
                           EventKind::readable() | EventKind::error()).unwrap();

    let msg_bytes = MESSAGE.as_bytes();
    let mut read_buf = [0; 2048];
    let mut nread = 0;
    let mut nsent = 0;
    'outer: while nread < msg_bytes.len() {
        events.clear();
        client_poll.poll(&mut events, Some(Duration::from_millis(100))).unwrap();

        for event in &events {
            assert_eq!(event.token(), CLIENT_TOKEN);
            assert!(! event.kind().is_error());

            if event.kind().is_readable() {
                while nread < nsent {
                    match client.read(&mut read_buf) {
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
            match client.write(&msg_bytes[nsent..]) {
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

    assert_eq!(nread, 13);
    assert_eq!(MESSAGE, str::from_utf8(&read_buf[0..nread]).unwrap());

    server_thread.join().unwrap();
}
