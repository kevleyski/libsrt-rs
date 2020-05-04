use libsrt_sys::{EventKind, Events, Poll, Socket, Token};
use std::{
    io,
    str,
    time::Duration,
    thread,
};

static MESSAGE: &str = "hello libsrt-sys";

#[test]
fn sync_echo() {
    let try_addr = "127.0.0.1:0".parse().unwrap();

    // prepare server
    let server_sock = Socket::new(&try_addr).unwrap();
    server_sock.bind(&try_addr).unwrap();
    server_sock.listen(1).unwrap();

    let addr = server_sock.socket_addr().unwrap();

    // prepare client
    let client_sock = Socket::new(&addr).unwrap();

    let server_thread = thread::spawn(move || {
        let (peer_sock, _peer_addr) = server_sock.accept().unwrap();

        let mut buf = [0; 2048];
        let nread = peer_sock.recv(&mut buf).unwrap();
        assert_eq!(nread, 16);
        assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());

        peer_sock.send(&buf[0..nread]).unwrap();
        thread::sleep(Duration::from_millis(500)); // XXX
    });

    client_sock.connect(&addr).unwrap();
    client_sock.send(MESSAGE.as_bytes()).unwrap();

    let mut buf = [0; 2048];
    let nread = client_sock.recv(&mut buf).unwrap();
    assert_eq!(nread, 16);
    assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());

    server_thread.join().unwrap();
}

#[test]
fn async_echo() {
    let try_addr = "127.0.0.1:0".parse().unwrap();

    // prepare server
    let server_sock = Socket::new(&try_addr).unwrap();
    server_sock.set_recv_nonblocking(true).unwrap();
    server_sock.set_send_nonblocking(true).unwrap();

    let server_poll = Poll::new().unwrap();

    const SERVER_TOKEN: Token = Token(0);
    server_poll.register(&server_sock, SERVER_TOKEN,
                         EventKind::readable() | EventKind::error()).unwrap();

    server_sock.bind(&try_addr).unwrap();
    server_sock.listen(1).unwrap();

    let addr = server_sock.socket_addr().unwrap();

    // prepare client
    let client_sock = Socket::new(&addr).unwrap();
    client_sock.set_recv_nonblocking(true).unwrap();
    client_sock.set_send_nonblocking(true).unwrap();

    let client_poll = Poll::new().unwrap();

    const CLIENT_TOKEN: Token = Token(1);
    client_poll.register(&client_sock, CLIENT_TOKEN,
                         EventKind::writable() | EventKind::error()).unwrap();

    let server_thread = thread::spawn(move || {
        let mut events = Events::with_capacity(2);
        let n = server_poll.poll(&mut events,
                                 Some(Duration::from_millis(1000))).unwrap();
        assert_eq!(n, 1);
        let event = events.iter().next().unwrap();
        assert_eq!(event.token(), SERVER_TOKEN);
        assert!(event.kind().is_readable());

        let (peer_sock, _peer_addr) = server_sock.accept().unwrap();

        const PEER_TOKEN: Token = Token(2);
        server_poll.reregister(&peer_sock, PEER_TOKEN,
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
                        match peer_sock.recv(&mut buf) {
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
                match peer_sock.send(&buf[nsent..nread]) {
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

        assert_eq!(nread, 16);
        assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());
    });

    client_sock.connect(&addr).unwrap();
    let mut events = Events::with_capacity(2);
    client_poll.poll(&mut events, Some(Duration::from_millis(1000))).unwrap();
    assert!(events.iter().next().is_some());

    client_poll.reregister(&client_sock, CLIENT_TOKEN,
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
                    match client_sock.recv(&mut read_buf) {
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
            match client_sock.send(&msg_bytes[nsent..]) {
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

    assert_eq!(nread, 16);
    assert_eq!(MESSAGE, str::from_utf8(&read_buf[0..nread]).unwrap());

    server_thread.join().unwrap();
}
