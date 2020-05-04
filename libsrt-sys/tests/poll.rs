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
    poll.register(&sock, TOKEN,
                  EventKind::writable() | EventKind::error()).unwrap();

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
    poll.register(&sock, TOKEN,
                  EventKind::writable() | EventKind::error()).unwrap();
    poll.deregister(&sock).unwrap();

    let mut events = Events::with_capacity(2);
    let n = poll.poll(&mut events, Some(Duration::from_millis(1))).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn notify_connection_break() {
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
    conn_poll.register(&conn_sock, CONN_TOKEN,
                       EventKind::readable() | EventKind::error()).unwrap();

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
