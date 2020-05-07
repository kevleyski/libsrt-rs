use futures::{
    channel,
    stream,
};
use slab::Slab;
use std::{
    future::Future,
    io::{self, Read, Write},
    net::SocketAddr,
    pin::Pin,
    sync::{
        self,
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{
        self,
        Context,
    },
    thread,
};
use crate::net::{
    self,
    TRANSTYPE,
    Bind,
    EventKind,
    Events,
    Listener,
    Poll,
    Stream,
    Token,
};

const MSG_PLSIZE: usize = 32;

////////////////////////////////////////////////////////////////////////////////
// Builder
////////////////////////////////////////////////////////////////////////////////
pub struct Builder {
    tx: Sender<Message>,
    cnt: Arc<AtomicUsize>,
}

enum Message {
    Connecting(SocketAddr, channel::oneshot::Sender<Stream>),
    Listening(Listener, channel::mpsc::Sender<Stream>),
    Done,
}

struct Sender<T> {
    tx: sync::mpsc::Sender<T>,
    inner: Arc<Channel>,
}

struct Receiver<T> {
    rx: sync::mpsc::Receiver<T>,
    inner: Arc<Channel>,
}

struct Channel {
    ready: AtomicBool,
    tx: Stream,
    rx: Stream,
}

enum Task {
    Message(),
    Connecting(Stream, channel::oneshot::Sender<Stream>),
    Listening(Listener, channel::mpsc::Sender<Stream>),
}

impl Builder {
    pub fn new() -> Result<Builder, io::Error> {
        const LISTEN_TOKEN: Token = Token(0);
        const CONN1_TOKEN: Token = Token(1);

        let poll = Poll::new()?;
        let mut events = Events::with_capacity(2);

        let addr = "127.0.0.1:0".parse().unwrap();
        let listener = net::Builder::new()
            .trans_type(TRANSTYPE::SRTT_FILE)
            .payload_size(MSG_PLSIZE)
            .nonblocking(true)
            .bind(&addr)?;
        poll.register(&listener, LISTEN_TOKEN, EventKind::readable())?;
        let addr = listener.local_addr()?;

        let conn1 = net::Builder::new()
            .trans_type(TRANSTYPE::SRTT_FILE)
            .payload_size(MSG_PLSIZE)
            .nonblocking(true)
            .connect(&addr)?;
        poll.register(&conn1, CONN1_TOKEN, EventKind::writable())?;

        // XXX infinite waiting
        let mut accepted = None;
        let mut connected = false;
        let conn2 = loop {
            events.clear();
            poll.poll(&mut events, None)?;
            for event in &events {
                match event.token() {
                    LISTEN_TOKEN => accepted = Some(listener.accept()?.0),
                    CONN1_TOKEN => connected = true,
                    _ => {}
                }
            }

            if connected && accepted.is_some() {
                break accepted.unwrap();
            }
        };
        
        drop(listener);

        let inner = Arc::new(Channel {
            ready: AtomicBool::new(false),
            tx: conn1,
            rx: conn2,
        });
        let (tx, rx) = sync::mpsc::channel();

        let tx = Sender { tx: tx, inner: inner.clone() };
        let tx2 = tx.clone();
        let rx = Receiver { rx: rx, inner: inner };

        thread::spawn(|| {
            run(tx2, rx);
        });

        Ok(Builder {
            tx: tx,
            cnt: Arc::new(AtomicUsize::new(1)),
        })
    }

    pub fn connect(&self, addr: SocketAddr) -> Connecting {
        let (tx, rx) = channel::oneshot::channel();
        self.tx.send(Message::Connecting(addr, tx));
        Connecting { inner: rx }
    }

    pub fn bind(&self, addr: &SocketAddr) -> io::Result<Listener> {
        net::Builder::new().nonblocking(true).bind(addr)
    }

    pub fn listen(&self, listener: Listener) -> Incoming {
        let (tx, rx) = channel::mpsc::channel(1000); // XXX
        self.tx.send(Message::Listening(listener, tx));
        Incoming { inner: rx }
    }
}

impl Clone for Builder {
    fn clone(&self) -> Builder {
        self.cnt.fetch_add(1, Ordering::SeqCst);
        Builder {
            tx: self.tx.clone(),
            cnt: self.cnt.clone(),
        }
    }
}

impl Drop for Builder {
    fn drop(&mut self) {
        if self.cnt.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.tx.send(Message::Done);
        }
    }
}

fn enqueue(
    rx: &Receiver<Message>,
    poll: &Poll,
    tasks: &mut Slab<Task>,
    done: &mut bool
) {
    if !rx.drain() {
        trace!("no messages available");
        return
    }
    trace!("looking for some messages");
    while let Some(msg) = rx.recv() {
        match msg {
            Message::Done => {
                debug!("done");
                *done = true;
            }
            Message::Connecting(addr, complete) => {
                let stream = net::Builder::new()
                    .nonblocking(true)
                    .connect(&addr).unwrap(); // XXX
                // XXX check (tasks.len() == tasks.capacity())
                let task = Task::Connecting(stream, complete);
                let index = tasks.insert(task);
                match tasks.get(index).unwrap() { // XXX
                    Task::Connecting(stream, _complete) => {
                        poll.register(&stream, Token(index),
                                      EventKind::writable()).unwrap(); // XXX
                    }
                    _ => {}
                }
            }
            Message::Listening(listener, accept) => {
                let task = Task::Listening(listener, accept);
                let index = tasks.insert(task);
                match tasks.get(index).unwrap() { // XXX
                    Task::Listening(listener, _accept) => {
                        poll.register(&listener, Token(index),
                                      EventKind::readable()).unwrap(); // XXX
                    }
                    _ => {}
                }
            }
        }
    }
}

fn run(_tx: Sender<Message>, rx: Receiver<Message>) {
    let poll = Poll::new().expect("srt poll creation error");
    let mut events = Events::with_capacity(1000); // XXX
    let mut tasks = Slab::with_capacity(1000); // XXX
    let mut rx_done = false;

    let msg_index = tasks.insert(Task::Message());
    poll.register(&rx.inner.rx, Token(msg_index),
                  EventKind::readable()).unwrap(); // XXX

    loop {
        trace!("turn of the loop");
        if !rx_done {
            enqueue(&rx, &poll, &mut tasks, &mut rx_done);
        }
        if rx_done && tasks.len() == 0 {
            break
        }

        events.clear();
        // Wait for events
        poll.poll(&mut events, None).expect("srt poll error");
        for event in &events {
            match event.token() {
                Token(index) => {
                    let _kind = event.kind();

                    let task = tasks.get_mut(index).unwrap(); // XXX
                    match task {
                        Task::Message() => {
                            // Do nothing
                        }
                        Task::Connecting(stream, _complete) => {
                            poll.deregister(stream).unwrap();
                            let t = tasks.remove(index);
                            match t {
                                Task::Connecting(stream, complete) => {
                                    let _t = complete.send(stream); // XXX
                                }
                                _ => {}
                            }
                        }
                        Task::Listening(listener, accept) => {
                            loop {
                                match listener.accept() {
                                    Ok((stream, _peer_addr)) => {
                                        let stream = net::Builder::new()
                                            .nonblocking(true)
                                            .accept(stream).unwrap();
                                        let _t = accept.try_send(stream); // XXX
                                    }
                                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                        break;
                                    }
                                    Err(_e) => {
                                        break; // XXX
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<T> Sender<T> {
    fn send(&self, t: T) {
        self.tx.send(t).unwrap();
        self.inner.notify();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        Sender {
            tx: self.tx.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<T> Receiver<T> {
    fn recv(&self) -> Option<T> {
        self.rx.try_recv().ok()
    }

    /// Returns whether there are messages to look at
    fn drain(&self) -> bool {
        if !self.inner.ready.swap(false, Ordering::SeqCst) {
            return false
        }
        loop {
            match (&self.inner.rx).read(&mut [0; MSG_PLSIZE]) {
                Ok(_) => {}
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => panic!("I/O error: {}", e),
            }
        }
        return true
    }
}

impl Channel {
    fn notify(&self) {
        if !self.ready.swap(true, Ordering::SeqCst) {
            drop((&self.tx).write(&[1]));
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Connecting
////////////////////////////////////////////////////////////////////////////////

pub struct Connecting {
    inner: channel::oneshot::Receiver<Stream>,
}

impl Future for Connecting {
    type Output = Result<Stream, io::Error>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> task::Poll<Self::Output> {
        match Pin::new(&mut self.inner).poll(cx) {
            task::Poll::Ready(Ok(res)) => task::Poll::Ready(Ok(res)),
            task::Poll::Ready(Err(_e)) => { // XXX
                task::Poll::Ready(Err(io::Error::new(io::ErrorKind::Other,
                                                     "canceled")))
            }
            task::Poll::Pending => task::Poll::Pending,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Incoming
////////////////////////////////////////////////////////////////////////////////

pub struct Incoming {
    inner: channel::mpsc::Receiver<Stream>,
}

impl stream::Stream for Incoming {
    type Item = Stream;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> task::Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}
