#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

extern crate futures;
extern crate tokio_io;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;
extern crate capnp;
extern crate bytes;

use futures::{Future, Stream, Poll};
use futures::sync::mpsc;

use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::{Encoder, Decoder, Framed};
use tokio_core::reactor::Handle;
use tokio_proto::{TcpClient, TcpServer};
use tokio_proto::streaming::{Body, Message};
use tokio_proto::streaming::pipeline::{Frame, ServerProto, ClientProto};
use tokio_proto::util::client_proxy::ClientProxy;
use tokio_service::{Service, NewService};

use bytes::{BytesMut, BufMut};

use std::{io, str};
use std::net::SocketAddr;

mod cache_capnp;

/// Line-based client handle
///
/// This type just wraps the inner service. This is done to encapsulate the
/// details of how the inner service is structured. Specifically, we don't want
/// the type signature of our client to be:
///
///   ClientTypeMap<ClientProxy<LineMessage, LineMessage, io::Error>>
///
/// This also allows adding higher level API functions that are protocol
/// specific. For example, our line client has a `ping()` function, which sends
/// a "ping" request.
pub struct Client {
    inner: ClientTypeMap<ClientProxy<LineMessage, LineMessage, io::Error>>,
}

/// The request and response type for the streaming line-based service.
///
/// A message is either "oneshot" and includes the full line, or it is streaming
/// and the line is broken up into chunks.
#[derive(Debug)]
pub enum Line {
    /// The full line
    Once(String),
    /// A stream of line chunks
    Stream(LineStream),
}

/// A stream of line chunks.
///
/// We defined a custom type that wraps `tokio_proto::streaming::Body` in order
/// to keep tokio-proto as an implementation detail.
#[derive(Debug)]
pub struct LineStream {
    inner: Body<String, io::Error>,
}

impl LineStream {
    /// Returns a `LineStream` with its sender half.
    pub fn pair() -> (mpsc::Sender<Result<String, io::Error>>, LineStream) {
        let (tx, rx) = Body::pair();
        (tx, LineStream { inner: rx })
    }
}

impl Stream for LineStream {
    type Item = String;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<String>, io::Error> {
        self.inner.poll()
    }
}

/// Message type used to communicate with tokio-proto. The library should hide
/// this and instead expose a custom message type
type LineMessage = Message<String, Body<String, io::Error>>;

/// Maps types between Line <-> LineMessage for the server service
struct ServerTypeMap<T> {
    inner: T,
}

/// Maps types between Line <-> LineMessage for the client service
struct ClientTypeMap<T> {
    inner: T,
}

/// Our line-based codec
///
/// In this version of the `LineCodec`, some state is required. We need to track
/// if we are currently decoding a message "head" or the streaming body.
pub struct LineCodec {
    decoding_head: bool,
}

/// Protocol definition
struct LineProto;

/// Start a server, listening for connections on `addr`.
///
/// For each new connection, `new_service` will be used to build a `Service`
/// instance to process requests received on the new connection.
///
/// This function will block as long as the server is running.
pub fn serve<T>(addr: SocketAddr, new_service: T)
    where T: NewService<Request = Line, Response = Line, Error = io::Error> + Send + Sync + 'static,
{
    let new_service = ServerTypeMap { inner: new_service };

    // Use the tokio-proto TCP server builder, this will handle creating a
    // reactor instance and other details needed to run a server.
    TcpServer::new(LineProto, addr)
        .serve(new_service);
}

impl Client {
    /// Establish a connection to a line-based server at the provided `addr`.
    pub fn connect(addr: &SocketAddr, handle: &Handle) -> Box<Future<Item = Client, Error = io::Error>> {
        let ret = TcpClient::new(LineProto)
            .connect(addr, handle)
            .map(|client_proxy| {
                // Wrap the returned client handle with our `ClientTypeMap`
                // service middleware
                let type_map = ClientTypeMap { inner: client_proxy };
                Client { inner: type_map }
            });

        Box::new(ret)
    }
}

impl Service for Client {
    type Request = Line;
    type Response = Line;
    type Error = io::Error;
    // For simplicity, box the future.
    type Future = Box<Future<Item = Line, Error = io::Error>>;

    fn call(&self, req: Line) -> Self::Future {
        self.inner.call(req)
    }
}

/*
 *
 * ===== impl Line =====
 *
 */

impl From<LineMessage> for Line {
    fn from(src: LineMessage) -> Line {
        match src {
            Message::WithoutBody(line) => Line::Once(line),
            Message::WithBody(head, body) => {
                assert_eq!(head, "");
                Line::Stream(LineStream { inner: body })
            }
        }
    }
}

impl From<Line> for Message<String, Body<String, io::Error>> {
    fn from(src: Line) -> Self {
        match src {
            Line::Once(line) => Message::WithoutBody(line),
            Line::Stream(body) => {
                let LineStream { inner } = body;
                Message::WithBody("".to_string(), inner)
            }
        }
    }
}

/*
 *
 * ===== ServerTypeMap =====
 *
 */

impl<T> Service for ServerTypeMap<T>
    where T: Service<Request = Line, Response = Line, Error = io::Error>,
          T::Future: 'static
{
    type Request = LineMessage;
    type Response = LineMessage;
    type Error = io::Error;
    type Future = Box<Future<Item = LineMessage, Error = io::Error>>;

    fn call(&self, req: LineMessage) -> Self::Future {
        Box::new(self.inner.call(req.into())
            .map(LineMessage::from))
    }
}

impl<T> NewService for ServerTypeMap<T>
    where T: NewService<Request = Line, Response = Line, Error = io::Error>,
          <T::Instance as Service>::Future: 'static
{
    type Request = LineMessage;
    type Response = LineMessage;
    type Error = io::Error;
    type Instance = ServerTypeMap<T::Instance>;

    fn new_service(&self) -> io::Result<Self::Instance> {
        let inner = try!(self.inner.new_service());
        Ok(ServerTypeMap { inner: inner })
    }
}

/*
 *
 * ===== ClientTypeMap =====
 *
 */

impl<T> Service for ClientTypeMap<T>
    where T: Service<Request = LineMessage, Response = LineMessage, Error = io::Error>,
          T::Future: 'static
{
    type Request = Line;
    type Response = Line;
    type Error = io::Error;
    type Future = Box<Future<Item = Line, Error = io::Error>>;

    fn call(&self, req: Line) -> Self::Future {
        Box::new(self.inner.call(req.into())
            .map(Line::from))
    }
}

/// Implementation of the simple line-based protocol.
///
/// Frames consist of a UTF-8 encoded string, terminated by a '\n' character.
impl Decoder for LineCodec {
    type Item = Frame<String, String, io::Error>;
    type Error = io::Error;


    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        // Check to see if the frame contains a new line
        if let Some(n) = buf.as_ref().iter().position(|b| *b == b'\n') {
            // remove the serialized frame from the buffer.
            let line = buf.split_to(n);

            // Also remove the '\n'
            buf.split_to(1);

            // Turn this data into a UTF string and return it in a Frame.
            return match str::from_utf8(&line.as_ref()) {
                Ok(s) => {
                    // Got an empty line, which means that the state should be
                    // toggled.
                    if s == "" {
                        let decoding_head = self.decoding_head;
                        // Toggle the state
                        self.decoding_head = !decoding_head;

                        if decoding_head {
                            Ok(Some(Frame::Message {
                                // The message head is an empty line
                                message: s.to_string(),
                                // We will be streaming a body after this
                                body: true,
                            }))
                        } else {
                            // We parsed the streaming body "termination" frame,
                            // which is represented as `None`.
                            Ok(Some(Frame::Body {
                                chunk: None
                            }))
                        }
                    } else {
                        if self.decoding_head {
                            // This is a "oneshot" message with no streaming
                            // body
                            Ok(Some(Frame::Message {
                                message: s.to_string(),
                                body: false,
                            }))
                        } else {
                            // This line is a chunk in a streaming body
                            Ok(Some(Frame::Body {
                                chunk: Some(s.to_string()),
                            }))
                        }
                    }
                }
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "invalid string")),
            }
        }

        Ok(None)
    }
}

impl Encoder for LineCodec {
    type Item = Frame<String, String, io::Error>;
    type Error = io::Error;


    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        match msg {
            Frame::Message { message, body } => {
                // Our protocol dictates that a message head that includes a
                // streaming body is an empty string.
                assert!(message.is_empty() == body);

                buf.reserve(message.len());
                buf.extend(message.as_bytes());
            }
            Frame::Body { chunk } => {
                if let Some(chunk) = chunk {
                    buf.reserve(chunk.len());
                    buf.extend(chunk.as_bytes());
                }
            }
            Frame::Error { error } => {
                // Our protocol does not support error frames, so this results
                // in a connection level error, which will terminate the socket.
                return Err(error);
            }
        }

        // Push the new line
        buf.put_u8(b'\n');

        Ok(())
    }
}

impl<T: AsyncRead + AsyncWrite + 'static> ClientProto<T> for LineProto {
    type Request = String;
    type RequestBody = String;
    type Response = String;
    type ResponseBody = String;
    type Error = io::Error;

    /// `Framed<T, LineCodec>` is the return value of `io.framed(LineCodec)`
    type Transport = Framed<T, LineCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        let codec = LineCodec {
            decoding_head: true,
        };

        Ok(io.framed(codec))
    }
}

impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for LineProto {
    type Request = String;
    type RequestBody = String;
    type Response = String;
    type ResponseBody = String;
    type Error = io::Error;

    /// `Framed<T, LineCodec>` is the return value of `io.framed(LineCodec)`
    type Transport = Framed<T, LineCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        let codec = LineCodec {
            decoding_head: true,
        };

        Ok(io.framed(codec))
    }
}