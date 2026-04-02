use crate::abi::*;
use std::cell::RefCell;
use std::ffi::CStr;
use std::fmt;
use std::ptr;

thread_local! {
    static LAST_ERR: RefCell<[c_char; 256]> = const { RefCell::new([0; 256]) };
}

fn copy_into_last_err(bytes: &[u8]) {
    LAST_ERR.with(|buf| {
        let mut buf = buf.borrow_mut();
        let len = bytes.len().min(buf.len() - 1);

        for (dst, src) in buf.iter_mut().zip(bytes.iter()).take(len) {
            *dst = *src as c_char;
        }
        buf[len] = 0;
    });
}

pub(crate) fn clear_last_err() {
    LAST_ERR.with(|buf| {
        buf.borrow_mut()[0] = 0;
    });
}

pub(crate) fn set_last_err_bytes(bytes: &[u8]) {
    copy_into_last_err(bytes);
}

pub(crate) unsafe fn set_last_err_cstr(text: *const c_char) {
    if text.is_null() {
        clear_last_err();
        return;
    }

    set_last_err_bytes(CStr::from_ptr(text).to_bytes());
}

pub(crate) fn set_last_err_fmt(args: fmt::Arguments<'_>) {
    let rendered = fmt::format(args);
    set_last_err_bytes(rendered.as_bytes());
}

pub(crate) fn json_util_get_last_err_impl() -> *const c_char {
    LAST_ERR.with(|buf| {
        let buf = buf.borrow();
        if buf[0] == 0 {
            ptr::null()
        } else {
            buf.as_ptr()
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn __json_c_set_last_err_text(text: *const c_char) {
    set_last_err_cstr(text);
}
