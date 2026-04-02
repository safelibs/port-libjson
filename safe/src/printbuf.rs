use crate::abi::*;
use std::cmp;
use std::mem::size_of;
use std::ptr;

extern "C"
{
    fn __errno_location() -> *mut c_int;
    fn calloc(nmemb: size_t, size: size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn malloc(size: size_t) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
}

const EFBIG: c_int = 27;

fn set_errno(value: c_int)
{
    unsafe {
        *__errno_location() = value;
    }
}

pub(crate) unsafe fn printbuf_new_impl() -> *mut printbuf
{
    let p = calloc(1, size_of::<printbuf>()).cast::<printbuf>();
    if p.is_null()
    {
        return ptr::null_mut();
    }

    (*p).size = 32;
    (*p).bpos = 0;
    (*p).buf = malloc((*p).size as usize).cast();
    if (*p).buf.is_null()
    {
        free(p.cast());
        return ptr::null_mut();
    }
    *(*p).buf = 0;
    p
}

pub(crate) unsafe fn printbuf_extend_impl(p: *mut printbuf, min_size: c_int) -> c_int
{
    if p.is_null()
    {
        return -1;
    }

    if (*p).size >= min_size
    {
        return 0;
    }
    if min_size > c_int::MAX - 8
    {
        set_errno(EFBIG);
        return -1;
    }

    let mut new_size = if (*p).size > c_int::MAX / 2
    {
        min_size + 8
    }
    else
    {
        cmp::max((*p).size * 2, min_size + 8)
    };
    if new_size <= 0
    {
        new_size = min_size + 8;
    }

    let new_buf = realloc((*p).buf.cast(), new_size as usize).cast::<c_char>();
    if new_buf.is_null()
    {
        return -1;
    }

    (*p).buf = new_buf;
    (*p).size = new_size;
    0
}

pub(crate) unsafe fn printbuf_memappend_impl(p: *mut printbuf, buf: *const c_char, size: c_int) -> c_int
{
    if p.is_null() || buf.is_null()
    {
        return -1;
    }
    if size < 0 || size > c_int::MAX - (*p).bpos - 1
    {
        set_errno(EFBIG);
        return -1;
    }
    if (*p).size <= (*p).bpos + size + 1 && printbuf_extend_impl(p, (*p).bpos + size + 1) < 0
    {
        return -1;
    }

    memcpy((*p).buf.add((*p).bpos as usize).cast(), buf.cast(), size as usize);
    (*p).bpos += size;
    *(*p).buf.add((*p).bpos as usize) = 0;
    size
}

pub(crate) unsafe fn printbuf_memset_impl(pb: *mut printbuf, offset: c_int, charvalue: c_int, len: c_int) -> c_int
{
    if pb.is_null()
    {
        return -1;
    }

    let mut offset = offset;
    if offset == -1
    {
        offset = (*pb).bpos;
    }
    if len < 0 || offset < -1 || len > c_int::MAX - offset
    {
        set_errno(EFBIG);
        return -1;
    }

    let size_needed = offset + len;
    if (*pb).size < size_needed && printbuf_extend_impl(pb, size_needed) < 0
    {
        return -1;
    }

    if (*pb).bpos < offset
    {
        memset(
            (*pb).buf.add((*pb).bpos as usize).cast(),
            0,
            (offset - (*pb).bpos) as usize,
        );
    }
    memset((*pb).buf.add(offset as usize).cast(), charvalue, len as usize);
    if (*pb).bpos < size_needed
    {
        (*pb).bpos = size_needed;
    }
    0
}

pub(crate) unsafe fn printbuf_reset_impl(p: *mut printbuf)
{
    if p.is_null() || (*p).buf.is_null()
    {
        return;
    }

    *(*p).buf = 0;
    (*p).bpos = 0;
}

pub(crate) unsafe fn printbuf_free_impl(p: *mut printbuf)
{
    if p.is_null()
    {
        return;
    }

    free((*p).buf.cast());
    free(p.cast());
}
