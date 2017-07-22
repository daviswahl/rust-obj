use futures;
use futures::Future;
use tokio_io::AsyncRead;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;
use cache_capnp;
use capnp;
use error;
use capnp_futures;
use build_messages;

pub fn client() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let addr = "127.0.0.1:12345".parse().unwrap();

    let socket = TcpStream::connect(&addr, &handle);

    let request = socket.and_then(|socket| {
        let (r, w) = socket.split();
        let (mut sender, write_queue) = capnp_futures::write_queue(w);
        let read_stream = capnp_futures::ReadStream::new(r, Default::default());

        let mut m = capnp::message::Builder::new_default();

        let mut m_data = capnp::message::Builder::new_default();
        {
            let mut foo = m_data.init_root::<cache_capnp::foo::Builder>();
            foo.set_name("bar");
        }

        let mut buf = vec![];
        capnp::serialize_packed::write_message(&mut buf, &m_data).unwrap();

        build_messages(m.init_root(), cache_capnp::Op::Set, "foo", buf);

        sender.send(m);
        let mut futs = vec![];
        for _ in 0..100 {
            let mut m2 = capnp::message::Builder::new_default();
            build_messages(m2.init_root(), cache_capnp::Op::Get, "foo", vec![]);
            futs.push(sender.send(m2));
        }
        let futs = futures::future::join_all(futs);

        futs.join(write_queue).map_err(|e| error::decoding(e.description.as_ref()).into())
    });

    core.run(request).unwrap();
}
