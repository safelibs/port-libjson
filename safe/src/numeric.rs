use crate::abi::*;
use crate::errors;
use std::ptr;

unsafe extern "C"
{
    fn __errno_location() -> *mut c_int;
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn strtoll(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_longlong;
    fn strtoull(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulonglong;
}

pub const JSON_NUMBER_CHARS_BYTES: &[u8; 16] = b"0123456789.+-eE\0";
pub const JSON_HEX_CHARS_BYTES: &[u8; 23] = b"0123456789abcdefABCDEF\0";

const EINVAL: c_int = 22;
const JSON_TYPE_NAMES: [&[u8]; 7] =
    [b"null\0", b"boolean\0", b"double\0", b"int\0", b"object\0", b"array\0", b"string\0"];

fn errno_ptr() -> *mut c_int
{
    unsafe { __errno_location() }
}

fn set_errno(value: c_int)
{
    unsafe {
        *errno_ptr() = value;
    }
}

pub(crate) unsafe fn json_parse_double_impl(buf: *const c_char, retval: *mut c_double) -> c_int
{
    if buf.is_null() || retval.is_null()
    {
        return 1;
    }

    let mut end = ptr::null_mut();
    *retval = strtod(buf, &mut end);
    if end == buf.cast_mut()
    {
        1
    }
    else
    {
        0
    }
}

pub(crate) unsafe fn json_parse_int64_impl(buf: *const c_char, retval: *mut int64_t) -> c_int
{
    if buf.is_null() || retval.is_null()
    {
        set_errno(EINVAL);
        return 1;
    }

    let mut end = ptr::null_mut();
    let errno = errno_ptr();
    *errno = 0;

    let val = strtoll(buf, &mut end, 10) as int64_t;
    if end != buf.cast_mut()
    {
        *retval = val;
    }
    if (val == 0 && *errno != 0) || end == buf.cast_mut()
    {
        *errno = EINVAL;
        return 1;
    }
    0
}

pub(crate) unsafe fn json_parse_uint64_impl(buf: *const c_char, retval: *mut uint64_t) -> c_int
{
    if buf.is_null() || retval.is_null()
    {
        set_errno(EINVAL);
        return 1;
    }

    let errno = errno_ptr();
    *errno = 0;

    let mut cursor = buf.cast::<u8>();
    while *cursor == b' '
    {
        cursor = cursor.add(1);
    }
    if *cursor == b'-'
    {
        return 1;
    }

    let mut end = ptr::null_mut();
    let val = strtoull(cursor.cast(), &mut end, 10) as uint64_t;
    if end != cursor.cast_mut().cast()
    {
        *retval = val;
    }
    if (val == 0 && *errno != 0) || end == cursor.cast_mut().cast()
    {
        *errno = EINVAL;
        return 1;
    }
    0
}

pub(crate) unsafe fn json_type_to_name_impl(o_type: json_type) -> *const c_char
{
    let idx = o_type as isize;
    if idx < 0 || idx as usize >= JSON_TYPE_NAMES.len()
    {
        errors::set_last_err_fmt(format_args!(
            "json_type_to_name: type {} is out of range [0,{}]\n",
            o_type,
            JSON_TYPE_NAMES.len()
        ));
        return ptr::null();
    }

    JSON_TYPE_NAMES[idx as usize].as_ptr().cast()
}
