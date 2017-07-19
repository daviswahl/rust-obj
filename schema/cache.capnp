@0x970aa9edf26e100e;

enum Type {
    foo @0;
}

enum Op {
    get @0;
    set @1;
    del @2;
}

struct Message(Value) {
    op @0 :Op;
    key @1 :Data;
    value @2 :Wrapper(Value);
}

struct Messages {
    messages @0 :List(Message);
}

struct Wrapper(Value){
    type @0 :Type;
    data @1 :Value;
}

struct Foo {
    name @0 :Text;
}
