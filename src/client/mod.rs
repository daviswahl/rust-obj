use futures;
use futures::{Future, Stream};
use tokio_io::AsyncRead;
use message;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;
use cache_capnp;
use capnp;
use error;
use capnp_futures;
use build_messages;
use message::{FromProto, IntoProto};

pub fn client() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let addr = "127.0.0.1:12345".parse().unwrap();

    let socket = TcpStream::connect(&addr, &handle);

    let request = socket.and_then(|socket| {
        let (r, w) = socket.split();
        let (mut sender, write_queue) = capnp_futures::write_queue(w);
        let read_stream = capnp_futures::ReadStream::new(r, Default::default());

        let mut futs = vec![];

        for _ in 0..100 {
            let m = message::RequestBuilder::new()
                .set_key("foo")
                .set_op(message::Op::Set)
                .set_payload(message::Foo { name: format!("bar") })
                .finish()
                .unwrap();
            futs.push(sender.send(m.into_proto().unwrap()));
        }

        let futs = futures::future::join_all(futs);

        let requests = futs.join(write_queue);

        requests.and_then(|_| {
            read_stream.for_each(|m| {
                let msg: message::Request<message::Foo> = *message::Request::from_proto(m.get_root().unwrap()).unwrap();
                println!("{:?}", msg);
                futures::future::ok(())
            }).map_err(|_| capnp::Error::failed("fuck".into()))
        }).map_err(|_| error::decoding("fuck").into())
    });

    core.run(request).unwrap();
}
