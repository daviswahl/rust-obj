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

pub fn print_message(buf: &mut &[u8]) {
    let reader =
        capnp::serialize_packed::read_message(buf, capnp::message::ReaderOptions::default())
            .unwrap();
    let message = reader.get_root::<msg::Reader<foo::Owned>>().unwrap();
    let op = message.get_op().unwrap();
    let key = message.get_key().unwrap();
    let value = message.get_value().unwrap();
    let data = value.get_data().unwrap();
    let foo = data.get_name().unwrap();
    println!("OP: {:?} KEY: {:?} Foo: {}", op as u16, key, foo);
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

fn read_message(cache: Cache, reader: msg::Reader<capnp::any_pointer::Owned>) -> Vec<u8> {
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

fn wrap_result(data: Vec<u8>, mut builder: msg::Builder<capnp::any_pointer::Owned>) {
    use std::io::BufRead;
    let msg = capnp::serialize_packed::read_message(
        &mut data.as_ref(),
        capnp::message::ReaderOptions::default(),
    );
    builder.set_value(msg.unwrap().get_root().unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar() {}
    #[test]
    fn foo() {
        use tokio_core::reactor;
        use mio_uds::UnixStream;
        use capnp;
        use capnp_futures;
        use futures::future::Future;
        use futures::stream::Stream;

        use std::cell::Cell;
        use std::rc::Rc;

        let mut l = reactor::Core::new().unwrap();
        let handle = l.handle();
        let (s1, s2) = UnixStream::pair().unwrap();

        let s1 = reactor::PollEvented::new(s1, &handle).unwrap();
        let s2 = reactor::PollEvented::new(s2, &handle).unwrap();

        let cache: Cache = Arc::new(RwLock::new(HashMap::new()));
        {
            let (mut sender, write_queue) = capnp_futures::write_queue(s1);

            let read_stream = capnp_futures::ReadStream::new(s2, Default::default());

            let done_reading = read_stream.for_each(|m2| {
                let messages = m2.get_root::<msgs::Reader>().unwrap();
                {
                    for message in messages.get_messages().unwrap().iter() {
                        let result = read_message(cache.clone(), message);
                        let mut resp = capnp::message::Builder::new_default();
                        wrap_result(result, resp.init_root());
                    }
                }
                Ok(())
            });

            let io = done_reading.join(write_queue.map(|_| ()));

            let mut m = capnp::message::Builder::new_default();
            build_messages(m.init_root());
            handle.spawn(sender.send(m).map_err(|_| panic!("cancelled")).map(|_| {
                println!("SENT");
                ()
            }));
            drop(sender);
            l.run(io).expect("running");
        }
        println!("{:?}", cache);
        assert!(false)
    }
}
