extern crate capnp;

use cache_capnp;
use error;
use std::marker::PhantomData;

type AnyProto = capnp::any_pointer::Owned;
type AnyBuilder = capnp::message::Builder<capnp::message::HeapAllocator>;

pub trait HasTypeId {
    fn type_id(&self) -> Type;
}

pub trait IntoProto {
    fn into_proto(self) -> Result<AnyBuilder, error::Error>;
}

pub trait FromProto<'a> {
    type Reader;
    fn from_proto(m: &Self::Reader) -> Result<Box<Self>, error::Error>;
}

/// Request
#[derive(Debug, Clone)]
pub struct Request<'a, T: 'a + IntoProto + FromProto<'a> + HasTypeId> {
    key: Vec<u8>,
    op: Op,
    payload: Option<Payload<'a, T>>,
    _p: PhantomData<&'a T>
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> IntoProto for Request<'a, T> {
    fn into_proto(self) -> Result<AnyBuilder, error::Error> {
        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message =
                builder.init_root::<cache_capnp::request::Builder<AnyProto>>();
            message.set_op(self.op.into());
            message.set_key(self.key.as_ref());

            self.payload.map(|payload| message.set_payload(
                payload
                    .into_proto().unwrap()
                    .get_root::<cache_capnp::payload::Builder<AnyProto>>()?
                    .as_reader()
            ));
        }
        Ok(builder)
    }
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> FromProto<'a> for Request<'a, T> {
    type Reader = cache_capnp::request::Reader<'a, AnyProto>;

    fn from_proto(m: &Self::Reader) -> Result<Box<Self>, error::Error> {
        unimplemented!();
    }
}

/// Request Builder
pub struct RequestBuilder {
    op: Option<Op>,
    key: Option<Vec<u8>>,
}

impl RequestBuilder {
    pub fn new() -> Self {
        RequestBuilder{
            op: None,
            key: None,
        }
    }

    pub fn set_op(mut self, op: Op) -> Self {
        self.op = Some(op);
        self
    }

    pub fn set_key(mut self, key: &str) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn set_payload<'a, T>(self, payload: T) -> TypedRequestBuilder<'a, T>
        where T: 'a + IntoProto + FromProto<'a> + HasTypeId {
        TypedRequestBuilder{
            op: self.op,
            key: self.key,
            payload: payload,
            _p: PhantomData
        }
    }
}

/// Typed Request Builder
pub struct TypedRequestBuilder<'a, T: 'a + IntoProto + FromProto<'a> + HasTypeId> {
    op: Option<Op>,
    key: Option<Vec<u8>>,
    payload: T,
    _p: PhantomData<&'a T>
}

impl <'a, T> TypedRequestBuilder<'a, T> where T: 'a + IntoProto + FromProto<'a> + HasTypeId {
   pub fn finish(self) -> Result<Request<'a, T>, &'static str> {
       let op = self.op.ok_or("No op specified")?;
       let key = self.key.ok_or("No key specified")?;

       let payload = Payload{data: self.payload, _p: PhantomData};


       Ok(Request{op: op, key: key, payload: Some(payload), _p: PhantomData})
   }
}

/// Response
#[derive(Debug, Clone)]
pub struct Response<'a, T: 'a + IntoProto + FromProto<'a> + HasTypeId> {
    request_id: String,
    code: Code,
    payload: Option<Payload<'a, T>>,
    _p: PhantomData<&'a T>
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> IntoProto for Response<'a, T> {
    fn into_proto(self) -> Result<AnyBuilder, error::Error> {
        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message =
                builder.init_root::<cache_capnp::response::Builder<AnyProto>>();
            message.set_request_id(self.request_id.as_str());
            message.set_code(self.code.into());

            self.payload.map(|payload| message.set_payload(
                payload
                    .into_proto().unwrap()
                    .get_root::<cache_capnp::payload::Builder<AnyProto>>()?
                    .as_reader()
            ));
        }
        Ok(builder)
    }
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> FromProto<'a> for Response<'a, T> {
    type Reader = cache_capnp::response::Reader<'a, AnyProto>;

    fn from_proto(m: &Self::Reader) -> Result<Box<Self>, error::Error> {
        unimplemented!();
    }
}

/// Request Builder
pub struct ResponseBuilder {
    code: Option<Code>,
    request_id: Option<String>
}

impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder{
            code: None,
            request_id: None,
        }
    }

    pub fn set_code(mut self, code: Code) -> Self {
        self.code = Some(code);
        self
    }

    pub fn set_request_id(mut self, id: &str) -> Self {
        self.request_id = Some(id.into());
        self
    }

    pub fn set_payload<'a, T>(self, payload: T) -> TypedResponseBuilder<'a, T>
        where T: 'a + IntoProto + FromProto<'a> + HasTypeId {
        TypedResponseBuilder{
            request_id: self.request_id,
            code: self.code,
            payload: payload,
            _p: PhantomData
        }
    }
}

/// Typed Request Builder
pub struct TypedResponseBuilder<'a, T: 'a + IntoProto + FromProto<'a> + HasTypeId> {
    request_id: Option<String>,
    code: Option<Code>,
    payload: T,
    _p: PhantomData<&'a T>
}

impl <'a, T> TypedResponseBuilder<'a, T> where T: 'a + IntoProto + FromProto<'a> + HasTypeId {
   pub fn finish(self) -> Result<Response<'a, T>, &'static str> {
       let request_id = self.request_id.ok_or("No request id specified")?;
       let code = self.code.ok_or("No code specified")?;

       let payload = Payload{data: self.payload, _p: PhantomData};
       Ok(Response{code: code.into(), request_id: request_id.into(), payload: Some(payload), _p: PhantomData})
   }
}

/// Type
#[derive(Debug, Copy, Clone)]
pub enum Type {
    Foo,
}

impl From<cache_capnp::Type> for Type {
    fn from(tpe: cache_capnp::Type) -> Self {
        match tpe {
            cache_capnp::Type::Foo => Type::Foo,
        }
    }
}

impl From<Type> for cache_capnp::Type {
    fn from(tpe: Type) -> Self {
        match tpe {
            Type::Foo => cache_capnp::Type::Foo,
        }
    }
}

/// Op
#[derive(Debug, Copy, Clone)]
pub enum Op {
    Set,
    Get,
    Del,
}

impl From<cache_capnp::Op> for Op {
    fn from(op: cache_capnp::Op) -> Self {
        match op {
            cache_capnp::Op::Del => Op::Del,
            cache_capnp::Op::Set => Op::Set,
            cache_capnp::Op::Get => Op::Get,
        }
    }
}

impl From<Op> for cache_capnp::Op {
    fn from(op: Op) -> Self {
        match op {
            Op::Del => cache_capnp::Op::Del,
            Op::Set => cache_capnp::Op::Set,
            Op::Get => cache_capnp::Op::Get,
        }
    }
}

/// Code
#[derive(Debug, Copy, Clone)]
pub enum Code {
    Success,
    Failure,
}

impl From<cache_capnp::Code> for Code {
    fn from(code: cache_capnp::Code) -> Self {
        match code {
            cache_capnp::Code::Success => Code::Success,
            cache_capnp::Code::Failure => Code::Failure,
        }
    }
}

impl From<Code> for cache_capnp::Code {
    fn from(code: Code) -> Self {
        match code {
            Code::Success => cache_capnp::Code::Success,
            Code::Failure => cache_capnp::Code::Failure,
        }
    }
}

/// Payload
#[derive(Debug, Clone)]
pub struct Payload<'a, T: 'a + IntoProto + FromProto<'a> + HasTypeId> {
    data: T,
    _p: PhantomData<&'a T>
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> IntoProto for Payload<'a, T> {
    fn into_proto(self) -> Result<AnyBuilder, error::Error> {
        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message =
                builder.init_root::<cache_capnp::payload::Builder<AnyProto>>();
        }
        Ok(builder)
    }
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> FromProto<'a> for Payload<'a, T> {
    type Reader = cache_capnp::payload::Reader<'a, AnyProto>;

    fn from_proto(message: &Self::Reader) -> Result<Box<Self>, error::Error> {
        let tpe = message.get_type()?;
        let value = message.get_data()?;
        Err(error::decoding("error decoding"))
    }
}

/// Foo
#[derive(Debug, Clone)]
pub struct Foo {
    pub name: String,
}

impl IntoProto for Foo {
    fn into_proto(self) -> Result<AnyBuilder, error::Error> {
        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message = builder.init_root::<cache_capnp::foo::Builder>();
            message.set_name(self.name.as_str())
        }
        Ok(builder)
    }
}

impl <'a>FromProto<'a> for Foo {
    type Reader = cache_capnp::foo::Reader<'a>;

    fn from_proto(m: &Self::Reader) -> Result<Box<Self>, error::Error> {
        let name = m.get_name()?;
        Ok(Box::new(Self { name: name.into() }))
    }
}

impl HasTypeId for Foo {
    fn type_id(&self) -> Type {
        Type::Foo
    }
}

impl Foo {
    fn new(name: String) -> Self {
        Self { name: name }
    }
}
