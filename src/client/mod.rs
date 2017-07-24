
use futures;
use futures::{Future, BoxFuture, Stream, future};
use tokio_io::AsyncRead;
use tokio_io;
use message;
use tokio_core::net::TcpStream;
use tokio_core::reactor::{Core, Handle};
use tokio_proto::{TcpClient};
use tokio_proto::multiplex::ClientService;
use tokio_service::Service;
use cache_capnp;
use capnp;
use error;
use capnp_futures;
use std::net::{self, SocketAddr};
use build_messages;
use message::{FromProto, IntoProto};
use std::rc::Rc;
use std::sync::{Mutex, Arc};
use std::io;
use codec;
use service;

pub struct Client {
    inner: service::LogService<ClientService<TcpStream, codec::CacheProto>>
}

impl Client {
    pub fn connect(addr: &SocketAddr, handle: &Handle) -> Box<Future<Item = Client, Error = io::Error>>{
        Box::new(TcpClient::new(codec::CacheProto).connect(addr, handle).map(|client_service|
            Client { inner: service::LogService{ inner: client_service } }
        ))
    }
}

impl Service for Client {
    type Request = codec::Message;
    type Response = codec::Message;
    type Error = io::Error;
    type Future = Box<Future<Item = codec::Message, Error = io::Error>>;

    fn call(&self, req: codec::Message) -> Self::Future {
        self.inner.call(req)
    }
}


