use std::io::{Read, Write};
use std::str;
use std::time::Duration;
use std::thread;

use libsrt_rs::net::Builder;
use libsrt_rs::net::Bind;
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
