extern crate futures;
extern crate tokio_io;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;
extern crate bytes;

use futures::{future, Future};

use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::{Encoder, Decoder, Framed};
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_proto::{TcpClient, TcpServer};
use tokio_proto::multiplex::{RequestId, ServerProto, ClientProto, ClientService};
use tokio_service::{Service, NewService};

use bytes::{BytesMut, Buf, BufMut, BigEndian};

use std::{io, str};
use std::net::SocketAddr;

use codec;
pub fn serve<T>(addr: SocketAddr, new_service: T)
where
    T: NewService<Request = codec::Message, Response = codec::Message, Error = io::Error>
        + Send
        + Sync
        + 'static,
{
    TcpServer::new(codec::CacheProto, addr).serve(new_service)
}

pub struct CacheService;

impl Service for CacheService
{
    type Request = codec::Message;
    type Response = codec::Message;
    type Error = io::Error;
    type Future = Box<Future<Item = codec::Message, Error = io::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        println!("Request: {:?}", req);
        Box::new(self.call(req))
    }
}

impl NewService for CacheService
{
    type Request = codec::Message;
    type Response = codec::Message;
    type Error = io::Error;
    type Instance = CacheService;

    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(CacheService)
    }
}
