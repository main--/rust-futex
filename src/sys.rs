use libc::{c_int, syscall, timespec};
use std::{ptr, io, i32};
use std::sync::atomic::{AtomicI32, AtomicU32};

const FUTEX_WAIT: c_int = 0;
const FUTEX_WAKE: c_int = 1;
const FUTEX_WAIT_BITSET: c_int = 9;
const FUTEX_WAKE_BITSET: c_int = 10;

#[inline(always)]
unsafe fn do_futex(uaddr: *mut c_int, futex_op: c_int, val: c_int, timeout: *const timespec, uaddr2: *mut c_int, val3: c_int) -> c_int {
    syscall(202/*SYS_futex*/, uaddr, futex_op, val, timeout, uaddr2, val3) as i32
}

#[inline(never)]
pub fn futex_wait(futex: &AtomicI32, val: i32) -> io::Result<()> {
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

#[inline(never)]
pub fn futex_wake(futex: &AtomicI32, count: i32) -> io::Result<i32> {
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

#[inline]
pub fn futex_wait_bitset(futex: &AtomicU32, val: u32, mask: i32) -> io::Result<()> {
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

#[inline]
pub fn futex_wake_bitset(futex: &AtomicU32, count: u32, mask: i32) -> io::Result<i32> {
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
