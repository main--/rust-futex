mod futex;
// legacy versions:
//mod rwfutex;
//mod rwfutex2;
//mod rwfutex3;
mod rwfutex4;

pub use self::futex::Futex as Mutex;
pub use self::rwfutex4::RwFutex2 as RwLock;

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use std::sync::Arc;
    use super::*;

    #[test]
    fn mutex() {
        let futex = Arc::new(Mutex::new());
        let futex2 = futex.clone();
        futex.acquire();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            futex2.release();
        }).join().unwrap();
        futex.acquire();
        futex.release();
    }

    #[test]
    fn rwlock() {
        let futex = Arc::new(RwLock::new());
        let futex2 = futex.clone();
        futex.acquire_read();
        futex.acquire_read();
        futex.acquire_read();
        thread::spawn(move || {
            futex2.acquire_read();
            futex2.release_read();
            futex2.acquire_write();
            thread::sleep(Duration::from_millis(100));
            futex2.release_write();
        });
        futex.release_read();
        futex.release_read();
        futex.release_read();
        futex.acquire_read();
        futex.release_read();
    }
}
