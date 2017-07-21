extern crate capnp;

use cache_capnp;
use std::marker::PhantomData;


pub trait HasTypeId {
    fn type_id(&self) -> Type;
}

pub trait IntoProto {
    fn into_proto(self) -> capnp::message::Builder<capnp::message::HeapAllocator>;
}

pub trait FromProto<'a> {
    type From;
    fn from_proto(m: Self::From) -> Result<Box<Self>, capnp::Error>;
}

/// Message
#[derive(Debug, Clone)]
pub struct Message<'a, T: 'a + IntoProto + FromProto<'a> + HasTypeId> {
    key: String,
    op: Op,
    envelope: Envelope<'a, T>,
    _p: PhantomData<&'a T>
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> IntoProto for Message<'a, T> {
    fn into_proto(self) -> capnp::message::Builder<capnp::message::HeapAllocator> {
        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message =
                builder.init_root::<cache_capnp::message::Builder<capnp::any_pointer::Owned>>();
            message.set_op(self.op.into());
            message.set_key(self.key.as_bytes());

            message.set_value(
                self.envelope
                    .into_proto()
                    .get_root::<cache_capnp::envelope::Builder<capnp::any_pointer::Owned>>()
                    .unwrap()
                    .as_reader(),
            );
        }
        builder
    }
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> FromProto<'a> for Message<'a, T> {
    type From = cache_capnp::message::Reader<'a, capnp::any_pointer::Owned>;

    fn from_proto(m: Self::From) -> Result<Box<Self>, capnp::Error> {
        let env = m.get_value()?;
        Ok(Box::new(Self{op: Op::Set, key: "foo".into(), envelope: *Envelope::from_proto(env)?, _p: PhantomData}))
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
    fn into_proto(self) -> capnp::message::Builder<capnp::message::HeapAllocator> {
        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message =
                builder.init_root::<cache_capnp::envelope::Builder<capnp::any_pointer::Owned>>();
            message.set_type(self.type_id.into());
        }
        builder
    }
}

impl<'a, T: IntoProto + FromProto<'a> + HasTypeId> FromProto<'a> for Envelope<'a, T> {
    type From = cache_capnp::envelope::Reader<'a, capnp::any_pointer::Owned>;

    fn from_proto(message: Self::From) -> Result<Box<Self>, capnp::Error> {
        let tpe = message.get_type().unwrap();
        let value = message.get_data().unwrap();
        Err(capnp::Error{kind: capnp::ErrorKind::Failed, description: "Fuck".into()})
    }
}
/// Foo
#[derive(Debug, Clone)]
pub struct Foo {
    name: String,
}

impl IntoProto for Foo {
    fn into_proto(self) -> capnp::message::Builder<capnp::message::HeapAllocator> {
        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message = builder.init_root::<cache_capnp::foo::Builder>();
            message.set_name(self.name.as_str())
        }
        builder
    }
}

impl <'a>FromProto<'a> for Foo {
    type From = cache_capnp::foo::Reader<'a>;

    fn from_proto(m: Self::From) -> Result<Box<Self>, capnp::Error> {
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
        Self { name }
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foo() {
        let foo = Foo::new("bar".into());
        let env = Envelope {
            type_id: foo.type_id(),
            data: foo,
            _p: PhantomData
        };
        let msg = Message {
            op: Op::Set,
            key: "bar".into(),
            envelope: env,
            _p: PhantomData,
        };
        msg.into_proto();
    }
}
