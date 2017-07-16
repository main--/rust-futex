use std::{io, i32};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::atomic::Ordering;
use integer_atomics::AtomicI32;
use lock_wrappers::raw::Mutex;
use sys::{futex_wait, futex_wake};

/// A simple mutual exclusion lock (mutex).
///
/// This is not designed for direct use but as a building block for locks.
///
/// Thus, it is not reentrant and it may misbehave if used incorrectly
/// (i.e. you can release even if someone else is holding it).
/// It's also not fair.
pub struct Futex {
    futex: AtomicI32
}

impl Mutex for Futex {
    type LockState = ();

    // TOOD: review memory orderings
    /// Acquires the lock.
    ///
    /// This blocks until the lock is ours.
    fn lock(&self) {
        loop {
            match self.futex.fetch_sub(1, Ordering::Acquire) {
                1 => return, // jobs done - we got the lock
                _ => {
                    // lock is contended :(
                    self.futex.store(-1, Ordering::Relaxed);
                    // FIXME: deadlock if release stores the 1, we overwrite with -1 here
                    //        but they wake us up before we wait
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

    /// Attempts to acquire the lock without blocking.
    ///
    /// Returns `true` if the lock was acquired, `false` otherwise.
    fn try_lock(&self) -> Option<()> {
        if self.futex.compare_and_swap(1, 0, Ordering::Acquire) == 1 {
            Some(())
        } else {
            None
        }
    }

    /// Releases the lock.
    fn unlock(&self, _: ()) {
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

impl Default for Futex {
    /// Creates a new instance.
    fn default() -> Futex {
        Futex { futex: AtomicI32::new(1) }
    }
}

impl Debug for Futex {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "Futex@{:p} (={})", &self.futex as *const _, self.futex.load(Ordering::SeqCst))
    }
}
