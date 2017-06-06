#![feature(integer_atomics, core_intrinsics)]
extern crate libc;

mod sys;
pub mod raw;
mod mutex;
mod rwlock;

pub use mutex::{Mutex, MutexGuard};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};
