use std::{io, i32};
use std::sync::atomic::{AtomicI32, Ordering};
use sys::{futex_wait, futex_wake};

pub struct RwFutex {
    readers: AtomicI32,
    writers_queued: AtomicI32,
    writers_wakeup: AtomicI32,
}

//rconst WRITER_LOCKED: i32 = i32::MIN;
const WRITER_LOCKED_READERS_QUEUED: i32 = i32::MIN + 1;

impl RwFutex {
    pub fn new() -> RwFutex {
        RwFutex {
            readers: AtomicI32::new(0),
            writers_queued: AtomicI32::new(0),
            writers_wakeup: AtomicI32::new(0),
        }
    }

    pub fn acquire_read(&self) {
        loop {
            // only try to acquire if no one is waiting to write
            let mut val;
            if self.writers_queued.load(Ordering::Relaxed) == 0 {
                val = self.readers.fetch_add(1, Ordering::Acquire);
                if val >= 0 {
                    // got it
                    break;
                }
                
                // try to undo the damage (if any)
                val = val + 1;
                while val > WRITER_LOCKED_READERS_QUEUED && val < 0 {
                    val = self.readers.compare_and_swap(val, WRITER_LOCKED_READERS_QUEUED, Ordering::Relaxed);
                }

                if val >= 0 {
                    // unlock happened, try again to acquire
                    continue;
                }
            } else {
                val = self.readers.load(Ordering::Relaxed);
            }
            
            // writer is active
            if let Err(e) = futex_wait(&self.readers, val) {
                match e.kind() {
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::Interrupted => (), // ok
                    _ => panic!("{}", e),
                }
            }
        }
    }

    pub fn acquire_write(&self) {
        self.writers_queued.fetch_add(1, Ordering::Acquire);
        loop {
            let val = self.readers.compare_and_swap(0, i32::MIN, Ordering::Acquire);
            if val == 0 {
                // got it!
                break;
            } else {
                // load
                if let Err(e) = futex_wait(&self.writers_wakeup, 0) {
                    match e.kind() {
                        io::ErrorKind::WouldBlock
                            | io::ErrorKind::Interrupted => (), // ok
                        _ => panic!("{}", e),
                    }
                }
            }
        }
        self.writers_queued.fetch_sub(1, Ordering::Release);
    }

    pub fn release_read(&self) {
        let val = self.readers.fetch_sub(1, Ordering::Release);
        if val == 1 {
            // now 0 => no more readers => wake up a writer
            if self.writers_queued.load(Ordering::Relaxed) > 0 {
                futex_wake(&self.writers_wakeup, 1).unwrap();
            }
        }
    }

    pub fn release_write(&self) {
        self.readers.swap(0, Ordering::Release);
        if self.writers_queued.load(Ordering::Relaxed) > 0 {
            // store
            if futex_wake(&self.writers_wakeup, 1).unwrap() == 1 {
                // woke up a writer - don't wake up the readers
                return;
            }
        }

        futex_wake(&self.readers, i32::MAX).unwrap();
    }
}
