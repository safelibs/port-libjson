use crate::abi::*;
use core::sync::atomic::{AtomicI8, Ordering};
use std::cell::RefCell;

extern "C" {
    fn strerror(errnum: c_int) -> *mut c_char;
}

static ENABLE_STATE: AtomicI8 = AtomicI8::new(0);
thread_local! {
    static ERRNO_BUF: RefCell<[c_char; 128]> = const { RefCell::new([0; 128]) };
}

const EPERM: c_int = 1;
const ENOENT: c_int = 2;
const ESRCH: c_int = 3;
const EINTR: c_int = 4;
const EIO: c_int = 5;
const ENXIO: c_int = 6;
const E2BIG: c_int = 7;
const ENOEXEC: c_int = 8;
const EBADF: c_int = 9;
const ECHILD: c_int = 10;
const EAGAIN: c_int = 11;
const ENOMEM: c_int = 12;
const EACCES: c_int = 13;
const EFAULT: c_int = 14;
const ENOTBLK: c_int = 15;
const EBUSY: c_int = 16;
const EEXIST: c_int = 17;
const EXDEV: c_int = 18;
const ENODEV: c_int = 19;
const ENOTDIR: c_int = 20;
const EISDIR: c_int = 21;
const EINVAL: c_int = 22;
const ENFILE: c_int = 23;
const EMFILE: c_int = 24;
const ENOTTY: c_int = 25;
const ETXTBSY: c_int = 26;
const EFBIG: c_int = 27;
const ENOSPC: c_int = 28;
const ESPIPE: c_int = 29;
const EROFS: c_int = 30;
const EMLINK: c_int = 31;
const EPIPE: c_int = 32;
const EDOM: c_int = 33;
const ERANGE: c_int = 34;
const EDEADLK: c_int = 35;

fn errno_name(errno_in: c_int) -> Option<&'static str> {
    match errno_in {
        EPERM => Some("EPERM"),
        ENOENT => Some("ENOENT"),
        ESRCH => Some("ESRCH"),
        EINTR => Some("EINTR"),
        EIO => Some("EIO"),
        ENXIO => Some("ENXIO"),
        E2BIG => Some("E2BIG"),
        ENOEXEC => Some("ENOEXEC"),
        EBADF => Some("EBADF"),
        ECHILD => Some("ECHILD"),
        EDEADLK => Some("EDEADLK"),
        ENOMEM => Some("ENOMEM"),
        EACCES => Some("EACCES"),
        EFAULT => Some("EFAULT"),
        ENOTBLK => Some("ENOTBLK"),
        EBUSY => Some("EBUSY"),
        EEXIST => Some("EEXIST"),
        EXDEV => Some("EXDEV"),
        ENODEV => Some("ENODEV"),
        ENOTDIR => Some("ENOTDIR"),
        EISDIR => Some("EISDIR"),
        EINVAL => Some("EINVAL"),
        ENFILE => Some("ENFILE"),
        EMFILE => Some("EMFILE"),
        ENOTTY => Some("ENOTTY"),
        ETXTBSY => Some("ETXTBSY"),
        EFBIG => Some("EFBIG"),
        ENOSPC => Some("ENOSPC"),
        ESPIPE => Some("ESPIPE"),
        EROFS => Some("EROFS"),
        EMLINK => Some("EMLINK"),
        EPIPE => Some("EPIPE"),
        EDOM => Some("EDOM"),
        ERANGE => Some("ERANGE"),
        EAGAIN => Some("EAGAIN"),
        _ => None,
    }
}

fn strerror_mode() -> i8 {
    let state = ENABLE_STATE.load(Ordering::Acquire);
    if state != 0 {
        return state;
    }

    let discovered = if std::env::var_os("_JSON_C_STRERROR_ENABLE").is_some() {
        1
    } else {
        -1
    };
    let _ = ENABLE_STATE.compare_exchange(0, discovered, Ordering::AcqRel, Ordering::Acquire);
    ENABLE_STATE.load(Ordering::Acquire)
}

fn set_errno_buf(message: &str) -> *mut c_char {
    ERRNO_BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        let bytes = message.as_bytes();
        let len = bytes.len().min(buf.len() - 1);

        for (dst, src) in buf.iter_mut().zip(bytes.iter()).take(len) {
            *dst = *src as c_char;
        }
        buf[len] = 0;
        buf.as_mut_ptr()
    })
}

pub(crate) unsafe fn _json_c_strerror_impl(errno_in: c_int) -> *mut c_char {
    if strerror_mode() == -1 {
        return strerror(errno_in);
    }

    if let Some(name) = errno_name(errno_in) {
        return set_errno_buf(&format!("ERRNO={name}"));
    }

    set_errno_buf(&format!("ERRNO={errno_in}"))
}
