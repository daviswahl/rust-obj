use futures;
use futures::{Future, Stream};
use tokio_io::{AsyncWrite, AsyncRead};
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor::Core;
use std::env;
use capnp_futures::serialize::*;
use futures::Sink;
use cache_capnp;
use capnp;
use capnp_futures;
use build_messages;
use new_cache;
use read_message;
use Cache;

use std::rc::Rc;
use std::cell::Cell;

pub fn server() {
    // Create the event loop that will drive this server
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // Bind the server's socket
    let addr = "127.0.0.1:12345".parse().unwrap();
    let tcp = TcpListener::bind(&addr, &handle).unwrap();
    let mut cache = new_cache();


    let messages_read = Rc::new(Cell::new(0u32));
    let messages_read1 = messages_read.clone();

    let messages_sent = Rc::new(Cell::new(0u32));
    let messages_sent1 = messages_sent.clone();

    // Iterate incoming connections
    let server = tcp.incoming().for_each(move |(socket, _)| {
        // Split up the read and write halves
        let (r, w) = socket.split();
        let (mut sender, write_queue) = capnp_futures::write_queue(w);
        let read_stream = capnp_futures::ReadStream::new(r, Default::default());

        let cache = cache.clone();

        let messages_read = messages_read.clone();

        let server = read_stream
            .for_each(move |m| {

                let resp = handler(cache.clone(), m.get_root().unwrap());
                messages_read.set(messages_read.get() + 1);
                sender.send(resp).then(|_| {
                    Ok(())
                })
            })
            .map_err(|_| ());

        let server = server.join(write_queue.map_err(|_| ())).map(|_| ());
        handle.spawn(server);
        Ok(())
    });

    // Spin up the server on the event loop
    core.run(server).unwrap();
}

fn handler(
    cache: Cache,
    m: cache_capnp::request::Reader<capnp::any_pointer::Owned>,
) -> capnp::message::Builder<capnp::message::HeapAllocator> {
    let resp = read_message(cache, m);
    let mut m = capnp::message::Builder::new_default();
    build_messages(m.init_root(), cache_capnp::Op::Set, "foo", resp);
    m
}
