use futures;
use futures::{Future, Stream};
use tokio_io::{AsyncRead};
use tokio_io;
use message;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;
use cache_capnp;
use capnp;
use error;
use capnp_futures;
use std::net;
use build_messages;
use message::{FromProto, IntoProto};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::io;

type Message = capnp::message::Builder<capnp::message::HeapAllocator>;
type WriteQueue = capnp_futures::WriteQueue<tokio_io::io::WriteHalf<TcpStream>, Message>;
type ReadStream = capnp_futures::ReadStream<tokio_io::io::ReadHalf<TcpStream>>;
type Sender = capnp_futures::Sender<Message>;

/// Client
pub struct Client {
    core: Core,
    addr: net::SocketAddr,
    connection: Arc<RwLock<Option<Connection>>>,
    queue: VecDeque<Message>
}

impl Client {
    pub fn new(addr: &str) -> Result<Self, error::Error> {
        let core = Core::new()?;
        let addr = addr.parse()?;
        Ok(Self{core: core, addr: addr, connection: Arc::new(RwLock::new(None)), queue: VecDeque::new()})
    }

    pub fn connect(&mut self) -> Result<(), io::Error> {
        let mut lock = self.connection.clone();
        let connect = TcpStream::connect(&self.addr, &self.core.handle()).and_then(move|socket| {
            println!("getting lock");
            let mut lock = lock.try_write().unwrap();
            if lock.is_some() { return futures::future::ok(()) }

            let (r, w) = socket.split();
            let (mut sender, write_queue) = capnp_futures::write_queue(w);
            let read_stream = capnp_futures::ReadStream::new(r, Default::default());
            let connection = Connection{read_stream, write_queue, sender};
            lock.get_or_insert(connection);
            futures::future::ok(())
        }).map(|_| ()).map_err(|_| ());

        self.core.run(connect).unwrap();
        Ok(())
    }
}

/// Connection
pub struct Connection {
    write_queue: WriteQueue,
    read_stream: ReadStream,
    sender: Sender,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_1() {
        let mut client = Client::new("127.0.0.1:12345").unwrap();
        client.connect();
        assert!(client.connection.read().unwrap().is_some())

    }
}
