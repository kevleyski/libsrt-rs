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

fn main() {
    let _ = env_logger::init();

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
    drop(builder);  // XXX cannot access stderr during shutdown

    assert_eq!(1, 1);

    client_thread.join().unwrap();
}
