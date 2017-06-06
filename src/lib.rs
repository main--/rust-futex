#![feature(integer_atomics, core_intrinsics)]
extern crate libc;

mod sys;
mod futex;
// legacy versions:
//mod rwfutex;
//mod rwfutex2;
//mod rwfutex3;
mod rwfutex4;

pub use futex::Futex as Mutex;
pub use rwfutex4::RwFutex2 as RwLock;

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::thread;
    use std::sync::Arc;
    use super::*;
    
    #[test]
    fn mutex() {
        let futex = Arc::new(Mutex::new());
        let futex2 = futex.clone();
        futex.acquire();
        thread::spawn(move || {
            thread::sleep_ms(100);
            futex2.release();
        }).join().unwrap();
        futex.acquire();
        futex.release();
    }

    #[test]
    fn rwlock() {
        println!("lul");
        let futex = Arc::new(RwLock::new());
        let futex2 = futex.clone();
        futex.acquire_read();
        futex.acquire_read();
        futex.acquire_read();
        thread::spawn(move || {
            futex2.acquire_read();
            futex2.release_read();
            futex2.acquire_write();
            thread::sleep_ms(100);
            futex2.release_write();
        });
        futex.release_read();
        futex.release_read();
        futex.release_read();
        futex.acquire_read();
        futex.release_read();
    }
}
