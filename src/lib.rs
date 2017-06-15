#![cfg_attr(feature = "nightly", feature(core_intrinsics))]

extern crate libc;
extern crate integer_atomics;

mod sys;
pub mod raw;
mod mutex;
mod rwlock;

pub use mutex::{Mutex, MutexGuard};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
