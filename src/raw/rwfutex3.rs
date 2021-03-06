use std::i32;
use std::sync::atomic::{AtomicU32, Ordering};
use sys::{futex_wait_bitset, futex_wake_bitset};

pub struct RwFutex2 {
    futex: AtomicU32,
}

const M_DEATH: u32          = 0b10100000000010000000001000000000;
const F_WRITE_SHOVE: u32    = 0b01000000000000000000000000000000;
const M_WRITERS: u32        = 0b00011111111100000000000000000000;
const M_READERS_QUEUED: u32 = 0b00000000000001111111110000000000;
const M_READERS: u32        = 0b00000000000000000000000111111111;

const ONE_WRITER: u32       = 0b00000000000100000000000000000000;
const ONE_READER_QUEUED: u32 =0b00000000000000000000010000000000;
const ONE_READER: u32       = 0b00000000000000000000000000000001;

const ID_READER: i32 = 1;
const ID_WRITER: i32 = 2;

#[inline(always)]
fn safe_add(dst: &AtomicU32, val: u32, ordering: Ordering) -> u32 {
    let mut ret = dst.fetch_add(val, ordering);
    if ret & M_DEATH != 0 { die(dst) }
    ret = ret.wrapping_add(val);
    if ret & M_DEATH != 0 { die(dst) }
    ret
}

#[inline(always)]
fn safe_sub(dst: &AtomicU32, val: u32, ordering: Ordering) -> u32 {
    safe_add(dst, val.wrapping_neg(), ordering)
}

#[cold]
#[inline(never)]
fn die(dst: &AtomicU32) -> ! {
    // make it as unlikely as possible for any possible group
    // of concurrent operations to accidentally revive this
    dst.store(M_DEATH, Ordering::SeqCst);
    panic!("Spontaneous futex combustion! (overflow)");
}

impl RwFutex2 {
    pub fn new() -> RwFutex2 {
        RwFutex2 {
            futex: AtomicU32::new(0),
        }
    }

    pub fn acquire_read(&self) {
        loop {
            let mut val = safe_add(&self.futex, ONE_READER, Ordering::Acquire);
            if val & M_WRITERS == 0 {
                // got it
                break;
            }

            // writer lock - move from readers to readers_queued
            val = safe_add(&self.futex, ONE_READER_QUEUED - ONE_READER, Ordering::Acquire);
            
            if val & M_WRITERS == 0 {
                // writer unlocked in the meantime - leave queue and retry
            } else {
                if (val & M_READERS == 0) && (val & M_WRITERS != 0) {
                    // fix deadlock if our temporary new reader
                    // interleaved with release_read() calls
                    // so that we reach zero HERE => might have to wake up writers
                    futex_wake_bitset(&self.futex, 1, ID_WRITER);
                }
            
                futex_wait_bitset(&self.futex, val, ID_READER);
            }

            // no longer waiting - leave the queue
            safe_sub(&self.futex, ONE_READER_QUEUED, Ordering::Acquire);
        }
    }

    pub fn acquire_write(&self) {
        let mut have_lock = false;
        let mut val = safe_add(&self.futex, ONE_WRITER, Ordering::Acquire);
        loop {
            if have_lock {
                // I'm just waiting for readers to finish
                if val & M_READERS == 0 {
                    // got it
                    break;
                }
            } else if val & F_WRITE_SHOVE != 0 {
                // I'm one of (potentially many) waiting writers
                // (slow path)

                // hunger games: whoever manages to eat the shove flag wins
                let newval = self.futex.compare_and_swap(val, val & !F_WRITE_SHOVE, Ordering::Acquire);
                if val == newval {
                    // we won the race -> lock is ours
                    break;
                } else {
                    val = newval;
                    continue;
                }
            } else if val & M_WRITERS == ONE_WRITER {
                // I'm the only writer
                have_lock = true;
                if val & M_READERS == 0 {
                    // got it!
                    break;
                }
            } // else a writer is active right now

            // (slowest path - we wait)
            futex_wait_bitset(&self.futex, val, ID_WRITER);

            val = self.futex.load(Ordering::Acquire);
            println!("awrl{:08x} {}", val, have_lock);
        }
    }

    pub fn release_read(&self) {
        let val = safe_sub(&self.futex, ONE_READER, Ordering::Release);
        println!("r{:08x}", val);
        if (val & M_READERS == 0) && (val & M_WRITERS != 0) {
            // was 1 => now 0 => no more readers => writers queued => wake one up
            let ret = futex_wake_bitset(&self.futex, 1, ID_WRITER);
            assert_eq!(ret, 1);
        }
    }

    pub fn release_write(&self) {
        let val = safe_sub(&self.futex, ONE_WRITER, Ordering::Release);
        println!("w{:08x}", val);

        if val & M_WRITERS != 0 {
            // there are other writers waiting
            // we set the shove flag to signal that one of them may wake up now
            self.futex.fetch_or(F_WRITE_SHOVE, Ordering::Release);
            futex_wake_bitset(&self.futex, 1, ID_WRITER);
        } else {
            // no writers -> wake up readers (if any)
            if val & M_READERS_QUEUED != 0 {
                futex_wake_bitset(&self.futex, i32::MAX as u32, ID_READER);
            }
        }
    }
}

