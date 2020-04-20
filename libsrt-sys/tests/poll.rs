use libsrt_sys::{EventKind, Events, Poll, Socket, Token};
use std::{
    time::Duration,
    thread,
};

#[test]
fn wait_no_sockets_in_poll() {
    let poll = Poll::new().unwrap();
    let mut events = Events::with_capacity(2);
    let n = poll.poll(&mut events, Some(Duration::from_millis(1))).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn wait_empty_call() {
    let addr = "127.0.0.1:0".parse().unwrap();
    let sock = Socket::new(&addr).unwrap();
    sock.set_recv_nonblocking(true).unwrap();
    sock.set_send_nonblocking(true).unwrap();

    let poll = Poll::new().unwrap();

    const TOKEN: Token = Token(0);
    let event_kind = EventKind::writable() | EventKind::error();
    poll.register(&sock, TOKEN, event_kind).unwrap();

    let mut events = Events::with_capacity(2);
    let n = poll.poll(&mut events, Some(Duration::from_millis(1))).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn wait_all_sockets_in_poll_released() {
    let addr = "127.0.0.1:0".parse().unwrap();
    let sock = Socket::new(&addr).unwrap();
    sock.set_recv_nonblocking(true).unwrap();
    sock.set_send_nonblocking(true).unwrap();
    sock.set_sender(true).unwrap();
    sock.set_tsbpd_mode(true).unwrap();

    let poll = Poll::new().unwrap();

    const TOKEN: Token = Token(0);
    let event_kind = EventKind::writable() | EventKind::error();
    poll.register(&sock, TOKEN, event_kind).unwrap();
    poll.deregister(&sock).unwrap();

    let mut events = Events::with_capacity(2);
    let n = poll.poll(&mut events, Some(Duration::from_millis(1))).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn notify_connection_break() {
    let addr = "127.0.0.1:5555".parse().unwrap();

    // prepare client
    let client_sock = Socket::new(&addr).unwrap();
    client_sock.set_recv_nonblocking(true).unwrap();
    client_sock.set_send_nonblocking(true).unwrap();

    let client_poll = Poll::new().unwrap();

    const CLIENT_TOKEN: Token = Token(0);
    let event_kind = EventKind::writable() | EventKind::error();
    client_poll.register(&client_sock, CLIENT_TOKEN, event_kind).unwrap();

    // prepare server
    let server_sock = Socket::new(&addr).unwrap();
    client_sock.set_recv_nonblocking(true).unwrap();
    client_sock.set_send_nonblocking(true).unwrap();

    let server_poll = Poll::new().unwrap();

    const SERVER_TOKEN: Token = Token(1);
    let event_kind = EventKind::readable() | EventKind::error();
    server_poll.register(&server_sock, SERVER_TOKEN, event_kind).unwrap();

    server_sock.bind(&addr).unwrap();
    server_sock.listen(1).unwrap();

    let connect_thread = thread::spawn(move || {
        client_sock.connect(&addr).unwrap();

        let mut events = Events::with_capacity(2);
        let _ = client_poll.poll(&mut events,
                                 Some(Duration::from_millis(1000))).unwrap();
        client_sock
    });

    let mut events = Events::with_capacity(2);
    let n = server_poll.poll(&mut events,
                             Some(Duration::from_millis(5000))).unwrap();
    assert_eq!(n, 1);

    let (conn_sock, _) = server_sock.accept().unwrap();

    let conn_poll = Poll::new().unwrap();
    const CONN_TOKEN: Token = Token(3);
    let event_kind = EventKind::readable() | EventKind::error();
    conn_poll.register(&conn_sock, CONN_TOKEN, event_kind).unwrap();

    let client_sock = connect_thread.join().unwrap();

    let close_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(1));
        drop(client_sock);
    });

    let mut events = Events::with_capacity(2);
    let n = conn_poll.poll(&mut events, None).unwrap();
    assert_eq!(n, 1);
    assert_eq!(events.len(), 1);
    let event = events.iter().next().unwrap();
    assert_eq!(event.token(), CONN_TOKEN);

    assert!(conn_sock.is_broken().unwrap()
            || conn_sock.is_closing().unwrap()
            || conn_sock.is_closed().unwrap());

    let _ = close_thread.join().unwrap();
}
