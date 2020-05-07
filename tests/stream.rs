use std::{
    time::Duration,
    thread,
};
use libsrt_rs::{
    net::{
        self,
        Bind,
    },
    stream::{
        self,
    },
};
use futures::stream::StreamExt;
use async_std::task;

#[test]
fn stream_echo_client() {
    let builder = stream::Builder::new().unwrap();

    let try_addr = "127.0.0.1:0".parse().unwrap();

    let server = net::Builder::new().bind(&try_addr).unwrap();
    let addr = server.local_addr().unwrap();

    let server_thread = thread::spawn(move || {
        let (_peer, _peer_addr) = server.accept().unwrap();

        thread::sleep(Duration::from_millis(500)); // XXX
    });

    let conn_fut = builder.connect(addr);
    let _res = task::block_on(conn_fut);

    assert_eq!(1, 1);

    server_thread.join().unwrap();
}

#[test]
fn stream_echo_server() {
    let builder = stream::Builder::new().unwrap();

    let try_addr = "127.0.0.1:0".parse().unwrap();
    let server = builder.bind(&try_addr).unwrap();
    let addr = server.local_addr().unwrap();

    let client_thread = thread::spawn(move || {
        let mut _client = net::Builder::new().connect(&addr).unwrap();

        thread::sleep(Duration::from_millis(500)); // XXX
    });

    let accept_fnt = builder.listen(server).take(1).collect::<Vec<_>>();
    let _res = task::block_on(accept_fnt);

    assert_eq!(1, 1);

    client_thread.join().unwrap();
}
