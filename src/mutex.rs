use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use raw::Mutex as RawMutex;

pub struct Mutex<T> {
    mutex: RawMutex,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> { }
unsafe impl<T: Send> Sync for Mutex<T> { }

impl<T> Mutex<T> {
    pub fn new(t: T) -> Mutex<T> {
        Mutex {
            mutex: RawMutex::new(),
            data: UnsafeCell::new(t),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        self.mutex.acquire();
        MutexGuard { mutex: &self }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        if self.mutex.try_acquire() {
            Some(MutexGuard { mutex: &self })
        } else {
            None
        }
    }
}

impl<T: Default> Default for Mutex<T> {
    /// Creates a `Mutex<T>`, with the `Default` value for T.
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

#[must_use]
pub struct MutexGuard<'a, T: 'a> {
    mutex: &'a Mutex<T>
}

impl<'a, T: 'a> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.mutex.release();
    }
}

impl<'a, T: 'a> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T: 'a> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}
