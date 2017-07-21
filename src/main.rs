#![feature(test)]
extern crate test;
extern crate objcache;

use std::env;
use objcache::client::client;
use objcache::server::server;


fn main() {
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "server" => server(),
            "client" => client(),
            _ => (),
        }
    }
}

struct CapnProto;


#[cfg(test)]
mod tests {

    use super::*;
    use test::Bencher;
    #[bench]
    fn bench1(b: &mut Bencher) {
        b.iter(|| client())
    }
}
