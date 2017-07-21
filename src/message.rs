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
    fn from_proto(m: Self::Reader) -> Result<Box<Self>, error::Error>;
}

/// Request
#[derive(Debug, Clone)]
pub struct Request<'a, T: 'a + IntoProto + FromProto<'a> + HasTypeId> {
    key: Vec<u8>,
    op: Op,
    envelope: Option<Envelope<'a, T>>,
    _p: PhantomData<&'a T>
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> IntoProto for Request<'a, T> {
    fn into_proto(self) -> Result<AnyBuilder, error::Error> {
        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message =
                builder.init_root::<cache_capnp::message::Builder<AnyProto>>();
            message.set_op(self.op.into());
            message.set_key(self.key.as_ref());

            self.envelope.map(|envelope| message.set_value(
                envelope
                    .into_proto().unwrap()
                    .get_root::<cache_capnp::envelope::Builder<AnyProto>>()?
                    .as_reader()
            ));
        }
        Ok(builder)
    }
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> FromProto<'a> for Request<'a, T> {
    type Reader = cache_capnp::message::Reader<'a, AnyProto>;

    fn from_proto(m: Self::Reader) -> Result<Box<Self>, error::Error> {
        let env = if m.has_value() {
            Some(*Envelope::from_proto(m.get_value()?)?)
        } else {
            None
        };
        let key = m.get_key()?;
        println!("{:?}", key);
        Ok(Box::new(Self{op: Op::Set, key: key.into(), envelope: env, _p: PhantomData}))
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

/// Envelope
#[derive(Debug, Clone)]
pub struct Envelope<'a, T: 'a + IntoProto + FromProto<'a> + HasTypeId> {
    type_id: Type,
    data: T,
    _p: PhantomData<&'a T>
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> IntoProto for Envelope<'a, T> {
    fn into_proto(self) -> Result<AnyBuilder, error::Error> {
        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message =
                builder.init_root::<cache_capnp::envelope::Builder<AnyProto>>();
            message.set_type(self.type_id.into());
        }
        Ok(builder)
    }
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> FromProto<'a> for Envelope<'a, T> {
    type Reader = cache_capnp::envelope::Reader<'a, AnyProto>;

    fn from_proto(message: Self::Reader) -> Result<Box<Self>, error::Error> {
        let tpe = message.get_type()?;
        let value = message.get_data()?;
        Err(error::decoding("error decoding"))
    }
}

/// Foo
#[derive(Debug, Clone)]
pub struct Foo {
    name: String,
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

    fn from_proto(m: Self::Reader) -> Result<Box<Self>, error::Error> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn build_message(mut m: cache_capnp::message::Builder<cache_capnp::foo::Owned>) {
        m.set_op(cache_capnp::Op::Set);
        m.set_key("foo".as_bytes());
        {
            let mut value = m.init_value();
            value.set_type(cache_capnp::Type::Foo);
            {
                let mut data = value.init_data();
                data.set_name("bar")
            }
        }
    }

    #[test]
    fn test_foo() {
        let foo = Foo::new("bar".into());
        let env = Envelope {
            type_id: foo.type_id(),
            data: foo,
            _p: PhantomData
        };
        let msg = Request {
            op: Op::Set,
            key: "bar".into(),
            envelope: Some(env),
            _p: PhantomData,
        };
        msg.into_proto();

        let mut builder = capnp::message::Builder::new_default();
        build_message(builder.init_root());

        let mut root = builder.get_root::<cache_capnp::message::Builder<AnyProto>>().unwrap();
        let msg: Request<Foo> = *Request::from_proto(root.as_reader()).unwrap();
        assert_eq!(msg.key, "foo".as_bytes())
    }
}
