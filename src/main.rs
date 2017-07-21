extern crate objcache;

extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate capnp;
extern crate capnp_futures;
extern crate bytes;

use futures::{Future, Stream};
use tokio_io::{io, AsyncRead};
use tokio_io::codec::{Decoder, Encoder};
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor::Core;
use bytes::BytesMut;
use std::env;


fn main() {
   for arg in env::args().skip(1) {
       match arg.as_str() {
           "server" => server(),
           "client" => client(),
           _ => ()
       }
   }
}

fn client() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let addr = "127.0.0.1:12345".parse().unwrap();

    let socket = TcpStream::connect(&addr, &handle);

    let request = socket.and_then(|socket| {
        let (reader, writer) = socket.split();
        let (mut sender, write_queue) = capnp_futures::write_queue(writer);

        let read_stream = capnp_futures::ReadStream::new(reader, Default::default());

        let response = read_stream.for_each(|m| {
            objcache::print_message(m);
            Ok(())
        });

        let mut msg = capnp::message::Builder::new_default();
        objcache::build_messages(msg.init_root());

        let request = sender.send(msg).map(|_| { println!("sent"); ()}).map_err(|_| println!("fuck!!"));

        handle.spawn(sent);

        let io = response.join(write_queue).map(|_| ()).map_err(|_| ());

        println!("{:?}", handle.spawn(io));
        Ok(())
    });

    let result = core.run(request).unwrap();
    println!("{:?}", result);
}

fn server() {
    // Create the event loop that will drive this server
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // Bind the server's socket
    let addr = "127.0.0.1:12345".parse().unwrap();
    let tcp = TcpListener::bind(&addr, &handle).unwrap();
    let mut cache = objcache::new_cache();

    // Iterate incoming connections
    let server = tcp.incoming().for_each(move|(tcp, _)| {
        println!("here");
        // Split up the read and write halves
        let (reader, writer) = tcp.split();
        let (mut sender, write_queue) = capnp_futures::write_queue(writer);
        let read_stream = capnp_futures::ReadStream::new(reader, Default::default());
        let c2 = cache.clone();
        let done_reading = read_stream.and_then(move|m| {
            println!("here 2");
            let message = m.get_root::<objcache::cache_capnp::message::Reader<capnp::any_pointer::Owned>>().unwrap();
            let resp = objcache::read_message(c2.clone(), message);
            let mut m = capnp::message::Builder::new_default();
            objcache::wrap_result(resp, m.init_root());
            Ok(m)
        });
        let io = done_reading.for_each(move|m| {
            sender.send(m);
            Ok(())
        }).join(write_queue.map(|_| ())).map_err(|_| ()).map(|_| ());
        handle.spawn(io);

        Ok(())
    });

    // Spin up the server on the event loop
    core.run(server).unwrap();
}
