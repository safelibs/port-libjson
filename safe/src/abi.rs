#![allow(non_camel_case_types)]

pub use std::os::raw::{c_char, c_double, c_int, c_longlong, c_uint, c_ulong, c_ulonglong, c_void};

pub type size_t = usize;
pub type ssize_t = isize;
pub type int32_t = i32;
pub type int64_t = i64;
pub type uint32_t = u32;
pub type uint64_t = u64;
pub type json_bool = c_int;
pub type json_type = c_int;
pub type json_tokener_error = c_int;
pub type json_tokener_state = c_int;

#[repr(C)]
pub struct array_list {
    pub array: *mut *mut c_void,
    pub length: size_t,
    pub size: size_t,
    pub free_fn: Option<array_list_free_fn>,
}

#[repr(C)]
pub struct lh_entry {
    pub k: *const c_void,
    pub k_is_constant: c_int,
    pub v: *const c_void,
    pub next: *mut lh_entry,
    pub prev: *mut lh_entry,
}

#[repr(C)]
pub struct lh_table {
    pub size: c_int,
    pub count: c_int,
    pub head: *mut lh_entry,
    pub tail: *mut lh_entry,
    pub table: *mut lh_entry,
    pub free_fn: Option<lh_entry_free_fn>,
    pub hash_fn: Option<lh_hash_fn>,
    pub equal_fn: Option<lh_equal_fn>,
}

#[repr(C)]
pub struct printbuf {
    pub buf: *mut c_char,
    pub bpos: c_int,
    pub size: c_int,
}

#[repr(C)]
pub struct json_object {
    _private: [u8; 0],
}

#[repr(C)]
pub struct json_tokener {
    pub str: *mut c_char,
    pub pb: *mut printbuf,
    pub max_depth: c_int,
    pub depth: c_int,
    pub is_double: c_int,
    pub st_pos: c_int,
    pub char_offset: c_int,
    pub err: json_tokener_error,
    pub ucs_char: c_uint,
    pub high_surrogate: c_uint,
    pub quote_char: c_char,
    pub stack: *mut json_tokener_srec,
    pub flags: c_int,
}

#[repr(C)]
pub struct json_tokener_srec {
    pub state: c_int,
    pub saved_state: c_int,
    pub obj: *mut json_object,
    pub current: *mut json_object,
    pub obj_field_name: *mut c_char,
}

#[repr(C)]
pub struct json_object_iter {
    pub key: *mut c_char,
    pub val: *mut json_object,
    pub entry: *mut lh_entry,
}

#[repr(C)]
pub struct json_object_iterator {
    pub opaque_: *const c_void,
}

#[repr(C)]
pub struct json_patch_error {
    pub errno_code: c_int,
    pub patch_failure_idx: size_t,
    pub errmsg: *const c_char,
}

pub type array_list_free_fn = unsafe extern "C" fn(*mut c_void);
pub type comparison_fn = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;
pub type json_object_delete_fn = unsafe extern "C" fn(*mut json_object, *mut c_void);
pub type json_object_to_json_string_fn =
    unsafe extern "C" fn(*mut json_object, *mut printbuf, c_int, c_int) -> c_int;
pub type json_c_shallow_copy_fn = unsafe extern "C" fn(
    *mut json_object,
    *mut json_object,
    *const c_char,
    size_t,
    *mut *mut json_object,
) -> c_int;
pub type json_c_visit_userfunc = unsafe extern "C" fn(
    *mut json_object,
    c_int,
    *mut json_object,
    *const c_char,
    *mut size_t,
    *mut c_void,
) -> c_int;
pub type lh_entry_free_fn = unsafe extern "C" fn(*mut lh_entry);
pub type lh_hash_fn = unsafe extern "C" fn(*const c_void) -> c_ulong;
pub type lh_equal_fn = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

const _: [(); 32] = [(); core::mem::size_of::<array_list>()];
const _: [(); 40] = [(); core::mem::size_of::<lh_entry>()];
const _: [(); 56] = [(); core::mem::size_of::<lh_table>()];
const _: [(); 16] = [(); core::mem::size_of::<printbuf>()];
const _: [(); 24] = [(); core::mem::size_of::<json_object_iter>()];
const _: [(); 8] = [(); core::mem::size_of::<json_object_iterator>()];
const _: [(); 72] = [(); core::mem::size_of::<json_tokener>()];
const _: [(); 32] = [(); core::mem::size_of::<json_tokener_srec>()];
const _: [(); 24] = [(); core::mem::size_of::<json_patch_error>()];

const _: [(); 16] = [(); core::mem::offset_of!(json_object_iter, entry)];
const _: [(); 16] = [(); core::mem::offset_of!(json_patch_error, errmsg)];
const _: [(); 32] = [(); core::mem::offset_of!(json_tokener, char_offset)];
const _: [(); 48] = [(); core::mem::offset_of!(json_tokener, quote_char)];
const _: [(); 56] = [(); core::mem::offset_of!(json_tokener, stack)];
const _: [(); 64] = [(); core::mem::offset_of!(json_tokener, flags)];
const _: [(); 24] = [(); core::mem::offset_of!(json_tokener_srec, obj_field_name)];
