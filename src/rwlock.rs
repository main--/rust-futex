use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use raw::RwLock as RawRwLock;

pub struct RwLock<T> {
    rwlock: RawRwLock,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send + Sync> Send for RwLock<T> {}
unsafe impl<T: Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub fn new(t: T) -> RwLock<T> {
        RwLock {
            rwlock: RawRwLock::new(),
            data: UnsafeCell::new(t),
        }
    }

    #[inline]
    pub fn read(&self) -> RwLockReadGuard<T> {
        self.rwlock.acquire_read();
        RwLockReadGuard { rwlock: &self }
    }

    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<T> {
        self.rwlock.acquire_write();
        RwLockWriteGuard { rwlock: &self }
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(Default::default())
    }
}

#[must_use]
pub struct RwLockReadGuard<'a, T: 'a> {
    rwlock: &'a RwLock<T>
}

#[must_use]
pub struct RwLockWriteGuard<'a, T: 'a> {
    rwlock: &'a RwLock<T>
}

impl<'a, T: 'a> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.rwlock.rwlock.release_read();
    }
}

impl<'a, T: 'a> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.rwlock.rwlock.release_write();
    }
}

impl<'a, T: 'a> Deref for RwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.rwlock.data.get() }
    }
}

impl<'a, T: 'a> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.rwlock.data.get() }
    }
}

impl<'a, T: 'a> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.rwlock.data.get() }
    }
}
