use std::i32;
use std::sync::atomic::{AtomicU32, Ordering};
use sys::{futex_wait_bitset, futex_wake_bitset};

pub struct RwFutex2 {
    futex: AtomicU32,
}

//const WRITER_LOCKED: i32 = i32::MIN;
//const WRITER_LOCKED_READERS_QUEUED: i32 = i32::MIN + 1;

//const F_WRITER_LOCK: u32    = 0b10000000000000000000000000000000;
//const F_WRITER_QUEUED: u32  = 0b01000000000000000000000000000000;
const M_WRITERS: u32        = 0b11000000000000000000000000000000;
const M_READERS_QUEUED: u32 = 0b00111111110000000000000000000000;
const M_READERS: u32        = 0b00000000001111111111111111111111;
const F_WRITE: u32 = M_WRITERS; //F_WRITER_LOCK | F_WRITER_QUEUED;
//const E_WRITERS_FREE: u32   = 0b00000000000000000000000000000000;
const E_WRITERS_LOCK: u32   = 0b01000000000000000000000000000000;
const E_WRITERS_OVER: u32   = 0b11000000000000000000000000000000;
const E_WRITERS_RESV: u32   = 0b10000000000000000000000000000000;

//const ONE_WRITER: u32 = M_READERS_QUEUED + ONE_READER_QUEUED;
//const ONE_READER_QUEUED: u32 = M_READERS + ONE_READER;
const ONE_READER: u32 = 1;

const ID_READER: i32 = 1;
const ID_WRITER: i32 = 2;

impl RwFutex2 {
    pub fn new() -> RwFutex2 {
        RwFutex2 {
            futex: AtomicU32::new(0),
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
                futex_wake_bitset(&self.futex, 1, ID_WRITER);
            }
            
            futex_wait_bitset(&self.futex, val, ID_READER);
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
            
            futex_wait_bitset(&self.futex, val, ID_WRITER);
        }
        false
    }

    pub fn release_read(&self) {
        let val = self.futex.fetch_sub(ONE_READER, Ordering::Release);
        println!("r{:08x}", val);
        if (val & M_READERS == ONE_READER) && (val & E_WRITERS_LOCK != 0) {
            // was 1 => now 0 => no more readers => writers queued => wake one up
            let ret = futex_wake_bitset(&self.futex, 1, ID_WRITER);
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
            if futex_wake_bitset(&self.futex, 1, ID_WRITER) == 1 {
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
                let val = self.futex.fetch_add(readers, Ordering::Relaxed);
                if val & E_WRITERS_LOCK != 0 {
                    // we can only return if it was locked
                    // else there might be no-one to wake those readers
                    return;
                }
            }
        }
        
        if val & M_READERS_QUEUED != 0 {
            let ret = futex_wake_bitset(&self.futex, i32::MAX as u32, ID_READER) as u32;
            //assert_eq!(ret * ONE_READER_QUEUED, val & M_READERS_QUEUED); // FIXME: this is wrong (racy)
            let _ = ret;
        }
    }
}

