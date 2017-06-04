#![feature(integer_atomics)]
extern crate libc;

use libc::{c_int, syscall, timespec};
use std::{ptr, io, i32};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};

const FUTEX_WAIT: c_int = 0;
const FUTEX_WAKE: c_int = 1;
const FUTEX_WAIT_BITSET: c_int = 9;
const FUTEX_WAKE_BITSET: c_int = 10;

unsafe fn do_futex(uaddr: *mut c_int, futex_op: c_int, val: c_int, timeout: *const timespec, uaddr2: *mut c_int, val3: c_int) -> c_int {
    syscall(202/*SYS_futex*/, uaddr, futex_op, val, timeout, uaddr2, val3) as i32
}

fn futex_wait(futex: &AtomicI32, val: i32) -> io::Result<()> {
    let ret = unsafe { do_futex(futex as *const _ as *mut i32,
                                FUTEX_WAIT,
                                val,
                                ptr::null(),
                                ptr::null_mut(),
                                0) };
    match ret {
        0 => Ok(()),
        -1 => Err(io::Error::last_os_error()),
        _ => unreachable!(),
    }
}

fn futex_wake(futex: &AtomicI32, count: i32) -> io::Result<i32> {
    let ret = unsafe { do_futex(futex as *const _ as *mut i32,
                                FUTEX_WAKE,
                                count,
                                ptr::null(),
                                ptr::null_mut(),
                                0) };
    if ret == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

fn futex_wait_bitset(futex: &AtomicU32, val: u32, mask: i32) -> io::Result<()> {
    let ret = unsafe { do_futex(futex as *const _ as *mut i32,
                                FUTEX_WAIT_BITSET,
                                val as i32,
                                ptr::null(),
                                ptr::null_mut(),
                                mask) };
    match ret {
        0 => Ok(()),
        -1 => Err(io::Error::last_os_error()),
        _ => unreachable!(),
    }
}

fn futex_wake_bitset(futex: &AtomicU32, count: u32, mask: i32) -> io::Result<i32> {
    let ret = unsafe { do_futex(futex as *const _ as *mut i32,
                                FUTEX_WAKE_BITSET,
                                count as i32,
                                ptr::null(),
                                ptr::null_mut(),
                                mask) };
    if ret == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

pub struct RwFutex2 {
    futex: AtomicU32,
    writers_wakeup: AtomicI32,
}

//const WRITER_LOCKED: i32 = i32::MIN;
//const WRITER_LOCKED_READERS_QUEUED: i32 = i32::MIN + 1;

//const F_WRITER_LOCK: u32    = 0b10000000000000000000000000000000;
//const F_WRITER_QUEUED: u32  = 0b01000000000000000000000000000000;
const M_WRITERS: u32        = 0b11000000000000000000000000000000;
const M_READERS_QUEUED: u32 = 0b00111111110000000000000000000000;
const M_READERS: u32        = 0b00000000001111111111111111111111;
const F_WRITE: u32 = M_WRITERS; //F_WRITER_LOCK | F_WRITER_QUEUED;
const E_WRITERS_FREE: u32   = 0b00000000000000000000000000000000;
const E_WRITERS_LOCK: u32   = 0b01000000000000000000000000000000;
const E_WRITERS_OVER: u32   = 0b11000000000000000000000000000000;
const E_WRITERS_RESV: u32   = 0b10000000000000000000000000000000;

const ONE_WRITER: u32 = M_READERS_QUEUED + ONE_READER_QUEUED;
const ONE_READER_QUEUED: u32 = M_READERS + ONE_READER;
const ONE_READER: u32 = 1;

const ID_READER: i32 = 1;
const ID_WRITER: i32 = 2;

impl RwFutex2 {
    pub fn new() -> RwFutex2 {
        RwFutex2 {
            futex: AtomicU32::new(0),
            writers_wakeup: AtomicI32::new(0),
        }
    }

    pub fn acquire_read(&self) {
        loop {
            let mut val = self.futex.fetch_add(ONE_READER, Ordering::Acquire);
            if val & F_WRITE == 0 {
                // got it
                break;
            }

            // writer lock - move from readers to readers_queued
            val = self.futex.fetch_add(M_READERS, Ordering::Acquire); // clever trick
            val += M_READERS;
            
            if val & F_WRITE == 0 {
                // writer unlocked in the meantime - leave queue and retry
                // FIXME: we fucked up by moving to queue
                // however, right now someone else might acquire a new write lock
                // right now, we just don't care
                // worst case: next release_write calls FUTEX_WAKE up for a read that
                //             doesn't exist - aka 1 wasted syscall
                continue;
            } else if (val & M_READERS == 0) && (val & E_WRITERS_LOCK != 0) {
                // fix deadlock if our temporary new reader
                // interleaved with release_read() calls
                // so that we reach zero HERE => might have to wake up writers
                futex_wake_bitset(&self.futex, 1, ID_WRITER).unwrap();
            }
            
            if let Err(e) = futex_wait_bitset(&self.futex, val, ID_READER) {
                match e.kind() {
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::Interrupted => (), // ok
                    _ => panic!("{}", e),
                }
            }
        }
    }

    pub fn acquire_write(&self) -> bool {
        let mut have_lock = false;
        loop {
            let mut val = self.futex.fetch_or(E_WRITERS_LOCK, Ordering::Acquire);
            if !have_lock && (val & E_WRITERS_LOCK != 0) {
                // other writer(s)
            } else if val & M_WRITERS == E_WRITERS_RESV {
                // got it
                //break;
                return true;
            } else if val & M_READERS != 0 {
                // got the writelock, but readers exist
                have_lock = true;
            } else {
                // got it
                break;
            }

            val = self.futex.fetch_or(E_WRITERS_OVER, Ordering::Acquire);
            if val & E_WRITERS_LOCK == 0 {
                // got it accidentally
                break;
            }
            
            if let Err(e) = futex_wait_bitset(&self.futex, val, ID_WRITER) {
                match e.kind() {
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::Interrupted => (), // ok
                    _ => panic!("{}", e),
                }
            }
        }
        false
    }

    pub fn release_read(&self) {
        let val = self.futex.fetch_sub(ONE_READER, Ordering::Release);
        println!("r{:08x}", val);
        if (val & M_READERS == ONE_READER) && (val & E_WRITERS_LOCK != 0) {
            // was 1 => now 0 => no more readers => writers queued => wake one up
            let ret = futex_wake_bitset(&self.futex, 1, ID_WRITER).unwrap();
            assert_eq!(ret, 1);
        }
    }

    pub fn release_write(&self, resv: bool) {
        // clear resv and lock
        let val = self.futex.fetch_and(!E_WRITERS_LOCK, Ordering::Release);
        println!("w{:08x}", val);

        if (val & M_WRITERS == E_WRITERS_OVER) || resv {
            // there are /probably/ writers waiting
            // (the resv=true case is a little more complicated:
            //  we acquired while the lock was reserved, thus the OVER/RESV
            //  flag might be garbled, so we have to try to wake up here either way)
            if futex_wake_bitset(&self.futex, 1, ID_WRITER).unwrap() == 1 {
                // woke up a writer -> get out
                return;
            }
        }

        // no writers waiting -> clear RESV and wake up queued readers
        let val = self.futex.fetch_and(M_READERS | E_WRITERS_LOCK, Ordering::Release);
        if val & E_WRITERS_LOCK != 0 {
            // we lost the race: someone else has already re-acquired a write lock
            // they might even be overcommitted - we really have no way to tell

            // don't wake up readers for no reason - instead, re-queue them
            let readers = val & M_READERS_QUEUED;
            if readers != 0 {
                if self.futex.fetch_add(readers, Ordering::Relaxed) & E_WRITERS_LOCK != 0 {
                    // we can only return if it was locked
                    // else there might be no-one to wake those readers
                    return;
                }
            }
        }
        
        if val & M_READERS_QUEUED != 0 {
            let ret = futex_wake_bitset(&self.futex, i32::MAX as u32, ID_READER).unwrap() as u32;
            //assert_eq!(ret * ONE_READER_QUEUED, val & M_READERS_QUEUED); // FIXME: this is wrong (racy)
        }
    }
}

pub struct RwFutex {
    readers: AtomicI32,
    writers_queued: AtomicI32,
    writers_wakeup: AtomicI32,
}

const WRITER_LOCKED: i32 = i32::MIN;
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
            futex2.acquire_write();
            thread::sleep_ms(1000);
            futex2.release_write();
        }).join().unwrap();
        futex.release_read();
        futex.release_read();
        futex.release_read();
        futex.acquire_read();
        futex.release_read();
    }
}
