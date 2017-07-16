#![cfg_attr(feature = "nightly", feature(integer_atomics, core_intrinsics))]

extern crate libc;
extern crate integer_atomics;
extern crate lock_wrappers;

mod sys;
pub mod raw;

pub use lock_wrappers::raw::{Mutex as RawMutex, RwLock as RawRwLock};

pub type Mutex<T> = lock_wrappers::Mutex<raw::Mutex, T>;
pub type MutexGuard<'a, T> = lock_wrappers::MutexGuard<'a, raw::Mutex, T>;
pub type RwLock<T> = lock_wrappers::RwLock<raw::RwLock, T>;
pub type RwLockReadGuard<'a, T> = lock_wrappers::RwLockReadGuard<'a, raw::RwLock, T>;
pub type RwLockWriteGuard<'a, T> = lock_wrappers::RwLockWriteGuard<'a, raw::RwLock, T>;
