#![feature(test)]
extern crate test;
extern crate objcache;

use std::env;
use objcache::client::client;
use objcache::server::server;
use std::thread;


fn main() {
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "server" => server(),
            "client" => {
                loop {
                    let mut threads = vec![];
                    for _ in 0..100 {
                        threads.push(thread::spawn(|| client()));
                    }

                    for child in threads {
                        let _ = child.join();
                    }
                }
            }
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
