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
use cache_capnp::{Type, foo};
type Cache = Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>;

#[cfg(test)]
mod tests {
    use super::*;
    use cache_capnp::{message, messages, Op, foo, wrapper};
    fn build_messages(builder: messages::Builder) {
        let mut messages = builder.init_messages(1);
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
            let mut message = messages.borrow().get(0);
            message.set_op(Op::Set);
            message.set_key("foo".as_bytes());

            {
                let mut value = message.borrow().init_value();
                let mut data: foo::Builder = value.init_data().get_as().unwrap();

                data.set_name("bar");
            }

        }
    }

    fn read_value(cache: Cache, key: &[u8]) -> Vec<u8>
//where T: capnp::traits::SetPointerBuilder<<T as capnp::traits::Owned<'a>>:uBuilder> {
    {
        let cache = cache.read().unwrap();
        let data = cache.get(key).unwrap();
        let mut buf: Vec<u8> = vec![];
        let mut reader = capnp::message::Builder::new_default();
        {
            let mut message = reader.init_root::<message::Builder<capnp::any_pointer::Owned>>();
            message.set_key(key);
            message.set_op(Op::Get);

            let buf = capnp::serialize_packed::read_message(&mut data.clone().as_ref(), capnp::message::ReaderOptions::default()).unwrap();
            message.set_value(buf.get_root::<wrapper::Reader<_>>().unwrap());
        }

        capnp::serialize_packed::write_message(&mut buf, &reader);
        buf
    }

    fn set_value(cache: Cache, key: &[u8], value: wrapper::Reader<capnp::any_pointer::Owned>)
//where T: capnp::traits::SetPointerBuilder<<T as capnp::traits::Owned<'a>>::Builder> {
    {
        let mut cache = cache.write().unwrap();
        use std::io::BufRead;
        let mut buf: Vec<u8> = vec![];
        let mut data = value.get_data().unwrap();

        let mut builder = capnp::message::Builder::new_default();
        {
            let mut message = builder.init_root::<wrapper::Builder<capnp::any_pointer::Owned>>();
            message.set_data(data);
        }

        capnp::serialize_packed::write_message(&mut buf, &builder);
        cache.insert(Vec::from(key), buf);
    }

    fn read_message(cache: Cache, reader: message::Reader<capnp::any_pointer::Owned>) -> Vec<u8>
// where T: capnp::traits::SetPointerBuilder<<T as capnp::traits::Owned<'a>>::Builder> {
    {
        let reader = reader.clone();
        match reader.get_op() {
            Ok(op) => {
                match op {
                    Op::Get => read_value(cache, reader.get_key().expect("get key")),
                    Op::Set => {
                        println!("SET!");
                        use std::borrow::Borrow;
                        let value = reader.clone().get_value().unwrap();
                        set_value(cache, reader.get_key().unwrap(), value);
                        vec![]
                    }
                    Op::Del => {
                        println!("DEL!");
                        vec![]
                    }
                }
            }
            Err(e) => {
                println!("Error! {}", e);
                vec![]
            }
        }
    }

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
                let messages = m2.get_root::<messages::Reader>().unwrap();
                {
                    for message in messages.get_messages().unwrap().iter() {
                        read_message(cache.clone(), message.clone());
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
