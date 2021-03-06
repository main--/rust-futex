#![allow(deprecated)]
extern crate libc;
extern crate futex;

use futex::raw::RwLock;
use std::thread;
use std::sync::Arc;

fn main() {
    println!("lul");
    let futex = Arc::new(RwLock::new());
    let futex2 = futex.clone();
    futex.acquire_read();
    futex.acquire_read();
    futex.acquire_read();
    let thread = thread::spawn(move || {
        futex2.acquire_read();
        println!("thread reader");
        thread::sleep_ms(100);
        futex2.release_read();
        futex2.acquire_write();
        println!("thread writer");
        thread::sleep_ms(100);
        futex2.release_write();
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
    futex.acquire_write();
    println!("main writer");
    thread::sleep_ms(100);
    futex.release_write();
    thread.join().unwrap();
    println!("done");
}
