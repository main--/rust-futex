#![feature(integer_atomics)]
extern crate libc;

mod sys;
pub mod futex;
pub mod rwfutex;
pub mod rwfutex2;
pub mod rwfutex3;
pub mod rwfutex4;

pub type Futex = futex::Futex;
pub type RwFutex = rwfutex3::RwFutex2;

#[cfg(test)]
mod tests {
    use std::thread;
    use std::sync::Arc;
    use super::*;
    
    #[test]
    fn mutex() {
        let futex = Arc::new(Futex::new());
        let futex2 = futex.clone();
        futex.acquire();
        thread::spawn(move || {
            thread::sleep_ms(1000);
            futex2.release();
        }).join().unwrap();
        futex.acquire();
        futex.release();
    }

    #[test]
    fn rwlock() {
        println!("lul");
        let futex = Arc::new(RwFutex::new());
        let futex2 = futex.clone();
        futex.acquire_read();
        futex.acquire_read();
        futex.acquire_read();
        thread::spawn(move || {
            futex2.acquire_read();
            futex2.release_read();
            let resv = futex2.acquire_write();
            thread::sleep_ms(1000);
            futex2.release_write(resv);
        }); //.join().unwrap();
        futex.release_read();
        futex.release_read();
        futex.release_read();
        futex.acquire_read();
        futex.release_read();
    }
}
