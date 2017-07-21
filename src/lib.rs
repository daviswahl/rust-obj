extern crate capnp;
extern crate capnp_futures;
extern crate futures;
extern crate mio_uds;
extern crate tokio_core;
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::Arc;
use std::marker::PhantomData;

pub mod cache_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema/cache_capnp.rs"));
}
use cache_capnp::{Type, foo, message as msg, envelope, Op, messages as msgs};

pub use cache_capnp::message;

pub type Cache = Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>;

pub fn new_cache() -> Cache {
    Arc::new(RwLock::new(HashMap::new()))
}

pub fn build_messages(builder: msgs::Builder) {
    let mut messages = builder.init_messages(2);
    {
        let mut message = messages.borrow().get(0);
        message.set_op(Op::Set);
        message.set_key("foo".as_bytes());

        {
            let mut value = message.borrow().init_value();
            let mut data: foo::Builder = value.init_data().get_as().unwrap();

            data.set_name("bar");
        }

    }

    {
        let mut message = messages.borrow().get(1);
        message.set_op(Op::Get);
        message.set_key("foo".as_bytes());
    }
}

pub fn print_message(reader: capnp::message::Reader<capnp_futures::serialize::OwnedSegments>) {
    let message = reader.get_root::<msg::Reader<capnp::any_pointer::Owned>>().unwrap();
    let op = message.get_op().unwrap();
    let key = message.get_key().unwrap();
    let value = message.get_value().unwrap();
    let data = value.get_data().unwrap();
    println!("OP: {:?} KEY: {:?}", op as u16, key);
}

pub fn read_value(cache: Cache, key: &[u8]) -> Vec<u8> {
    let cache = cache.read().unwrap();
    cache.get(key).unwrap().to_owned()
}

pub fn set_value(cache: Cache, key: &[u8], value: envelope::Reader<capnp::any_pointer::Owned>) {
    let mut cache = cache.write().unwrap();
    use std::io::BufRead;
    let mut buf: Vec<u8> = vec![];
    let mut data = value.get_data().unwrap();

    let mut builder = capnp::message::Builder::new_default();
    {
        let mut message = builder.init_root::<envelope::Builder<capnp::any_pointer::Owned>>();
        message.set_data(data);
    }

    capnp::serialize_packed::write_message(&mut buf, &builder);
    cache.insert(Vec::from(key), buf);
}

pub fn read_message(cache: Cache, reader: msg::Reader<capnp::any_pointer::Owned>) -> Vec<u8> {
    match reader.get_op() {
        Ok(op) => {
            match op {
                Op::Get => {
                    println!("get!");
                    read_value(cache, reader.get_key().expect("get key"))
                }
                Op::Set => {
                    println!("SET!");
                    use std::borrow::Borrow;
                    let value = reader.get_value().unwrap();
                    let key = reader.get_key().unwrap();
                    set_value(cache, key, value);
                    vec![]
                }
                Op::Del => vec![],
            }
        }
        Err(e) => {
            println!("Error: {}", e);
            vec![]
        }
    }
}

pub fn wrap_result(data: Vec<u8>, mut builder: msg::Builder<capnp::any_pointer::Owned>) {
    use std::io::BufRead;
    let msg = capnp::serialize_packed::read_message(
        &mut data.as_ref(),
        capnp::message::ReaderOptions::default(),
    );
    builder.set_value(msg.unwrap().get_root().unwrap());
}

