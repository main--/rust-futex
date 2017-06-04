#![feature(integer_atomics)]
extern crate libc;

mod lib;
use lib::*;
use std::sync::Arc;
use std::thread;

/*
A: readers == locked
R: readers = 0
R: queued == 0
R: readers.go()
A: starve
*/

fn main() {
    println!("lul");
    let futex = Arc::new(RwFutex2::new());
    let futex2 = futex.clone();
    futex.acquire_read();
    futex.acquire_read();
    futex.acquire_read();
    let thread = thread::spawn(move || {
        futex2.acquire_read();
        println!("thread reader");
        thread::sleep_ms(100);
        futex2.release_read();
        let resv = futex2.acquire_write();
        println!("thread writer");
        thread::sleep_ms(100);
        futex2.release_write(resv);
        thread::sleep_ms(100);
        futex2.acquire_read();
        println!("thread reader 2");
        thread::sleep_ms(100);
        futex2.release_read();
    });
    thread::sleep_ms(100);
    futex.release_read();
    futex.release_read();
    println!("last reader going down");
    thread::sleep_ms(100);
    futex.release_read();
    thread::sleep_ms(100);
    let resv = futex.acquire_write();
    println!("main writer");
    thread::sleep_ms(100);
    futex.release_write(resv);
    thread.join().unwrap();
    println!("done");
}
