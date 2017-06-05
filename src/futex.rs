use std::{io, i32};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::atomic::{AtomicI32, Ordering};
use sys::{futex_wait, futex_wake};

pub struct Futex {
    futex: AtomicI32
}

impl Futex {
    /// Creates a new `Futex`.
    pub fn new() -> Futex {
        Futex { futex: AtomicI32::new(1) }
    }

    // TOOD: review memory orderings
    pub fn acquire(&self) {
        loop {
            match self.futex.fetch_sub(1, Ordering::Acquire) {
                1 => return, // jobs done - we got the lock
                _ => {
                    // lock is contended :(
                    self.futex.store(-1, Ordering::Relaxed);
                    match futex_wait(&self.futex, -1) {
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => (),
                        Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
                        Ok(_) => (),
                        _ => unreachable!(),
                    }
                }
            }
        }
    }

    pub fn release(&self) {
        match self.futex.fetch_add(1, Ordering::Release) {
            0 => return, // jobs done - no waiters
            _ => {
                // wake them up
                self.futex.store(1, Ordering::Release);
                futex_wake(&self.futex, i32::MAX).unwrap();
            }
        }
    }
}

impl Debug for Futex {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "Futex@{:p} (={})", &self.futex as *const _, self.futex.load(Ordering::SeqCst))
    }
}
