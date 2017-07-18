@0x970aa9edf26e100e;

enum Op {
    get @0;
    set @1;
    del @2;
}

struct Message(Value) {
    op @0 :Op;
    key @1 :Data;
    value @2 :Value;
}


