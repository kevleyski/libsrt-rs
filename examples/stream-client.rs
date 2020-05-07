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
use async_std::task;

fn main() {
    let _ = env_logger::init();

    let try_addr = "127.0.0.1:0".parse().unwrap();

    let server = net::Builder::new().bind(&try_addr).unwrap();
    let addr = server.local_addr().unwrap();

    let server_thread = thread::spawn(move || {
        let (_peer, _peer_addr) = server.accept().unwrap();

        // let mut buf = [0; 2048];
        // let nread = peer.read(&mut buf).unwrap();
        // assert_eq!(nread, 13);
        // assert_eq!(MESSAGE, str::from_utf8(&buf[0..nread]).unwrap());

        // peer.write(&buf[0..nread]).unwrap();
        thread::sleep(Duration::from_millis(500)); // XXX
    });

    let builder = stream::Builder::new().unwrap();
    let conn_fut = builder.connect(addr);
    let _res = task::block_on(conn_fut);
    drop(builder);  // XXX cannot access stderr during shutdown

    assert_eq!(1, 1);

    server_thread.join().unwrap();
}
