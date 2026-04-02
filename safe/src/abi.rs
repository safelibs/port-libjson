#![allow(non_camel_case_types)]

pub use std::os::raw::{c_char, c_double, c_int, c_uint, c_ulong, c_void};

pub type size_t = usize;
pub type int32_t = i32;
pub type int64_t = i64;
pub type uint64_t = u64;
pub type json_bool = c_int;
pub type json_type = c_int;
pub type json_tokener_error = c_int;

#[repr(C)]
pub struct array_list
{
    _private: [u8; 0],
}

#[repr(C)]
pub struct lh_entry
{
    _private: [u8; 0],
}

#[repr(C)]
pub struct lh_table
{
    _private: [u8; 0],
}

#[repr(C)]
pub struct printbuf
{
    _private: [u8; 0],
}

#[repr(C)]
pub struct json_object
{
    _private: [u8; 0],
}

#[repr(C)]
pub struct json_tokener
{
    _private: [u8; 0],
}

#[repr(C)]
pub struct json_tokener_srec
{
    _private: [u8; 0],
}

#[repr(C)]
pub struct json_object_iter
{
    pub key: *mut c_char,
    pub val: *mut json_object,
    pub entry: *mut lh_entry,
}

#[repr(C)]
pub struct json_object_iterator
{
    pub opaque_: *const c_void,
}

#[repr(C)]
pub struct json_patch_error
{
    pub errno_code: c_int,
    pub patch_failure_idx: size_t,
    pub errmsg: *const c_char,
}

pub type array_list_free_fn = unsafe extern "C" fn(*mut c_void);
pub type comparison_fn = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;
pub type json_object_delete_fn = unsafe extern "C" fn(*mut json_object, *mut c_void);
pub type json_object_to_json_string_fn =
    unsafe extern "C" fn(*mut json_object, *mut printbuf, c_int, c_int) -> c_int;
pub type json_c_shallow_copy_fn =
    unsafe extern "C" fn(*mut json_object, *mut json_object, *const c_char, size_t, *mut *mut json_object)
        -> c_int;
pub type json_c_visit_userfunc =
    unsafe extern "C" fn(*mut json_object, c_int, *mut json_object, *const c_char, *mut size_t, *mut c_void)
        -> c_int;
pub type lh_entry_free_fn = unsafe extern "C" fn(*mut lh_entry);
pub type lh_hash_fn = unsafe extern "C" fn(*const c_void) -> c_ulong;
pub type lh_equal_fn = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;
