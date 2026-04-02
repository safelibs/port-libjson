use crate::abi::*;
use core::cell::UnsafeCell;
use std::ffi::CStr;
use std::fmt;
use std::ptr;

pub(crate) struct StaticCell<T>(pub UnsafeCell<T>);

unsafe impl<T> Sync for StaticCell<T> {}

static LAST_ERR: StaticCell<[c_char; 256]> = StaticCell(UnsafeCell::new([0; 256]));

fn last_err_buf() -> &'static mut [c_char; 256]
{
    unsafe { &mut *LAST_ERR.0.get() }
}

fn copy_into_last_err(bytes: &[u8])
{
    let buf = last_err_buf();
    let len = bytes.len().min(buf.len() - 1);

    for (dst, src) in buf.iter_mut().zip(bytes.iter()).take(len)
    {
        *dst = *src as c_char;
    }
    buf[len] = 0;
}

pub(crate) fn clear_last_err()
{
    last_err_buf()[0] = 0;
}

pub(crate) fn set_last_err_bytes(bytes: &[u8])
{
    copy_into_last_err(bytes);
}

pub(crate) fn set_last_err_fmt(args: fmt::Arguments<'_>)
{
    let rendered = fmt::format(args);
    set_last_err_bytes(rendered.as_bytes());
}

pub(crate) fn json_util_get_last_err_impl() -> *const c_char
{
    let buf = unsafe { &*LAST_ERR.0.get() };
    if buf[0] == 0
    {
        ptr::null()
    }
    else
    {
        buf.as_ptr()
    }
}

#[no_mangle]
pub unsafe extern "C" fn __json_c_set_last_err_text(text: *const c_char)
{
    if text.is_null()
    {
        clear_last_err();
        return;
    }

    set_last_err_bytes(CStr::from_ptr(text).to_bytes());
}
