#![feature(test)]
extern crate test;
extern crate objcache;

extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_proto;
extern crate capnp;
extern crate capnp_futures;
extern crate bytes;

use futures::{Future, Stream};
use tokio_io::{io, AsyncRead, AsyncWrite};
use tokio_io::codec::{Decoder, Encoder};
use tokio_proto::streaming::pipeline::ServerProto;
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor::Core;
use bytes::BytesMut;
use std::env;
use capnp_futures::serialize::*;
use futures::Sink;


fn main() {
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "server" => server(),
            "client" => client(),
            _ => (),
        }
    }
}

struct CapnProto;

fn client() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let addr = "127.0.0.1:12345".parse().unwrap();

    let socket = TcpStream::connect(&addr, &handle);
    let transport = socket.and_then(|socket| {
        let transport = capnp_futures::serialize::Transport::new(socket, Default::default());
        futures::future::ok(transport)
    });

    let request = transport.and_then(|socket| {
        let (writer, reader) = socket.split();
        let mut m = capnp::message::Builder::new_default();
        objcache::build_messages(m.init_root(), objcache::Op::Get, "foo", vec![]);


        writer
            .send(m)
            .and_then(|_| {
                reader
                    .map(|m| objcache::print_message(m))
                    .into_future()
                    .map_err(|_| capnp::Error::failed(format!("foo")))
            })
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });

    core.run(request).unwrap();
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
    let server = tcp.incoming().for_each(move |(tcp, _)| {
        // Split up the read and write halves
        let (writer, reader) = capnp_futures::serialize::Transport::new(tcp, Default::default())
            .split();
        let c2 = cache.clone();
        let responses = reader.and_then(move |m| {
            let message =
                m.get_root::<objcache::cache_capnp::message::Reader<capnp::any_pointer::Owned>>()
                    .unwrap();
            let resp = objcache::read_message(c2.clone(), message);
            let mut m = capnp::message::Builder::new_default();

            objcache::build_messages(m.init_root(), objcache::Op::Set, "foo", resp);

            Ok(m)
        });
        use futures::Sink;
        let server = writer.send_all(responses).then(|_| Ok(()));
        handle.spawn(server);

        Ok(())
    });

    // Spin up the server on the event loop
    core.run(server).unwrap();
}

#[cfg(test)]
mod tests {

    use super::*;
    use test::Bencher;
    #[bench]
    fn bench1(b: &mut Bencher) {
        b.iter(||  client())
    }
}
