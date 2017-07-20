extern crate objcache;

extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate capnp;
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

    let mut buf: Vec<u8> = vec![];
    let request = socket.and_then(|socket| {
        let mut msg = capnp::message::Builder::new_default();
        objcache::build_messages(msg.init_root());
        capnp::serialize_packed::write_message(&mut buf, &msg);

        tokio_io::io::write_all(socket, buf)
    });

    let response = request.and_then(|(socket, _request)| {
        tokio_io::io::read_to_end(socket, Vec::new())
    });
    let (_socket, data) = core.run(response).unwrap();

    println!("{}", String::from_utf8_lossy(&data));
}

fn server() {
    // Create the event loop that will drive this server
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // Bind the server's socket
    let addr = "127.0.0.1:12345".parse().unwrap();
    let tcp = TcpListener::bind(&addr, &handle).unwrap();

    // Iterate incoming connections
    let server = tcp.incoming().for_each(|(tcp, _)| {
        // Split up the read and write halves
        let (reader, writer) = tcp.split();
        let responses = io::read_to_end(reader, Vec::new()).and_then(|(r, v)| futures::future::ok((r, v))).then(|_| Ok(()));
        // Spawn the future as a concurrent task
        handle.spawn(responses);
        Ok(())
    });

    // Spin up the server on the event loop
    core.run(server).unwrap();
}
