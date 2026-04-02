use crate::abi::*;
use std::cmp;
use std::mem::size_of;
use std::ptr;

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn calloc(nmemb: size_t, size: size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn malloc(size: size_t) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
}

const EFBIG: c_int = 27;

fn set_errno(value: c_int) {
    unsafe {
        *__errno_location() = value;
    }
}

fn printbuf_mut<'a>(pb: *mut printbuf) -> Option<&'a mut printbuf> {
    unsafe { pb.as_mut() }
}

fn buf_at(pb: &printbuf, offset: c_int) -> *mut c_char {
    unsafe { pb.buf.add(offset as usize) }
}

fn copy_bytes(dst: *mut c_char, src: *const c_char, len: usize) {
    unsafe {
        memcpy(dst.cast(), src.cast(), len);
    }
}

fn fill_bytes(dst: *mut c_char, value: c_int, len: usize) {
    unsafe {
        memset(dst.cast(), value, len);
    }
}

pub(crate) fn printbuf_new_impl() -> *mut printbuf {
    let p = unsafe { calloc(1, size_of::<printbuf>()).cast::<printbuf>() };
    if p.is_null() {
        return ptr::null_mut();
    }

    let pb = printbuf_mut(p).expect("fresh calloc printbuf pointer");
    pb.size = 32;
    pb.bpos = 0;
    pb.buf = unsafe { malloc(pb.size as usize).cast() };
    if pb.buf.is_null() {
        unsafe {
            free(p.cast());
        }
        return ptr::null_mut();
    }
    unsafe {
        *pb.buf = 0;
    }
    p
}

pub(crate) fn printbuf_extend_impl(p: *mut printbuf, min_size: c_int) -> c_int {
    let Some(pb) = printbuf_mut(p) else {
        return -1;
    };

    if pb.size >= min_size {
        return 0;
    }
    if min_size > c_int::MAX - 8 {
        set_errno(EFBIG);
        return -1;
    }

    let mut new_size = if pb.size > c_int::MAX / 2 {
        min_size + 8
    } else {
        cmp::max(pb.size * 2, min_size + 8)
    };
    if new_size <= 0 {
        new_size = min_size + 8;
    }

    let new_buf = unsafe { realloc(pb.buf.cast(), new_size as usize).cast::<c_char>() };
    if new_buf.is_null() {
        return -1;
    }

    pb.buf = new_buf;
    pb.size = new_size;
    0
}

pub(crate) fn printbuf_memappend_impl(p: *mut printbuf, buf: *const c_char, size: c_int) -> c_int {
    let Some(pb) = printbuf_mut(p) else {
        return -1;
    };
    if buf.is_null() {
        return -1;
    }
    if size < 0 || size > c_int::MAX - pb.bpos - 1 {
        set_errno(EFBIG);
        return -1;
    }
    if pb.size <= pb.bpos + size + 1 && printbuf_extend_impl(p, pb.bpos + size + 1) < 0 {
        return -1;
    }

    copy_bytes(buf_at(pb, pb.bpos), buf, size as usize);
    pb.bpos += size;
    unsafe {
        *buf_at(pb, pb.bpos) = 0;
    }
    size
}

pub(crate) fn printbuf_memset_impl(
    pb: *mut printbuf,
    offset: c_int,
    charvalue: c_int,
    len: c_int,
) -> c_int {
    let Some(pb) = printbuf_mut(pb) else {
        return -1;
    };

    let mut offset = offset;
    if offset == -1 {
        offset = pb.bpos;
    }
    if len < 0 || offset < -1 || len > c_int::MAX - offset {
        set_errno(EFBIG);
        return -1;
    }

    let size_needed = offset + len;
    if pb.size < size_needed && printbuf_extend_impl(pb, size_needed) < 0 {
        return -1;
    }

    if pb.bpos < offset {
        fill_bytes(buf_at(pb, pb.bpos), 0, (offset - pb.bpos) as usize);
    }
    fill_bytes(buf_at(pb, offset), charvalue, len as usize);
    if pb.bpos < size_needed {
        pb.bpos = size_needed;
    }
    0
}

pub(crate) fn printbuf_reset_impl(p: *mut printbuf) {
    let Some(pb) = printbuf_mut(p) else {
        return;
    };
    if pb.buf.is_null() {
        return;
    }

    unsafe {
        *pb.buf = 0;
    }
    pb.bpos = 0;
}

pub(crate) fn printbuf_free_impl(p: *mut printbuf) {
    if p.is_null() {
        return;
    }

    let buf = unsafe { (*p).buf };
    unsafe {
        free(buf.cast());
        free(p.cast());
    }
}
