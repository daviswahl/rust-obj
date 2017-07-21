extern crate capnp;


trait WriteMessage {
    fn write_message(&self, mut m: capnp::message::Builder<capnp::message::HeapAllocator>);
}

trait ReadMessage {
    fn read_message(m: capnp::any_pointer::Reader) -> Self;
}

enum Op {

}

struct Message<T: WriteMessage + ReadMessage> {
    key: String,
    envelope: Envelope<T>
}

struct Envelope<T: WriteMessage + ReadMessage> {
    data: T
}
