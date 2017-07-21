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

pub fn server() {
    // Create the event loop that will drive this server
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // Bind the server's socket
    let addr = "127.0.0.1:12345".parse().unwrap();
    let tcp = TcpListener::bind(&addr, &handle).unwrap();
    let mut cache = new_cache();


    // Iterate incoming connections
    let server = tcp.incoming().for_each(move |(tcp, _)| {
        // Split up the read and write halves
        let (writer, reader) = capnp_futures::serialize::Transport::new(tcp, Default::default())
            .split();
        let c2 = cache.clone();
        let responses = reader.and_then(move |m| {
            let message =
                m.get_root::<cache_capnp::message::Reader<capnp::any_pointer::Owned>>()
                    .unwrap();
            let resp = read_message(c2.clone(), message);
            let mut m = capnp::message::Builder::new_default();

            build_messages(m.init_root(), cache_capnp::Op::Set, "foo", resp);

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
