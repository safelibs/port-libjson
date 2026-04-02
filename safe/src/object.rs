use crate::abi::*;
use crate::{arraylist, errors, linkhash, printbuf as printbuf_impl, serializer};
use core::mem::size_of;
use core::sync::atomic::{AtomicU32, Ordering};
use std::ffi::CStr;
use std::ptr;

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn abort() -> !;
    fn free(ptr: *mut c_void);
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
}

pub(crate) const JSON_C_TO_STRING_SPACED: c_int = 1 << 0;
pub(crate) const JSON_C_TO_STRING_PRETTY: c_int = 1 << 1;
pub(crate) const JSON_C_TO_STRING_NOZERO: c_int = 1 << 2;
pub(crate) const JSON_C_TO_STRING_PRETTY_TAB: c_int = 1 << 3;
pub(crate) const JSON_C_TO_STRING_NOSLASHESCAPE: c_int = 1 << 4;
pub(crate) const JSON_C_OBJECT_ADD_KEY_IS_NEW: c_uint = 1 << 1;
pub(crate) const JSON_C_OBJECT_ADD_CONSTANT_KEY: c_uint = 1 << 2;
pub(crate) const JSON_OBJECT_DEF_HASH_ENTRIES: c_int = 16;
pub(crate) const ARRAY_LIST_DEFAULT_SIZE: c_int = 32;
pub(crate) const JSON_C_OPTION_GLOBAL: c_int = 0;
pub(crate) const JSON_C_OPTION_THREAD: c_int = 1;

const EINVAL: c_int = 22;
const ENOMEM: c_int = 12;

#[derive(Clone, Copy, Debug)]
pub(crate) enum JsonInt {
    Int64(int64_t),
    UInt64(uint64_t),
}

#[derive(Debug)]
pub(crate) enum JsonData {
    Object { table: *mut lh_table },
    Array { list: *mut array_list },
    Boolean(json_bool),
    Double(c_double),
    Int(JsonInt),
    String(Vec<u8>),
}

#[repr(C)]
struct JsonObjectBaseLayout {
    o_type: json_type,
    ref_count: uint32_t,
    to_json_string: Option<json_object_to_json_string_fn>,
    pb: *mut printbuf,
    user_delete: Option<json_object_delete_fn>,
    userdata: *mut c_void,
}

const _: [(); 40] = [(); size_of::<JsonObjectBaseLayout>()];

#[repr(C)]
pub(crate) struct JsonObjectBox {
    pub o_type: json_type,
    pub ref_count: AtomicU32,
    pub to_json_string: Option<json_object_to_json_string_fn>,
    pub pb: *mut printbuf,
    pub user_delete: Option<json_object_delete_fn>,
    pub userdata: *mut c_void,
    pub data: JsonData,
}

fn errno_ptr() -> *mut c_int {
    unsafe { __errno_location() }
}

pub(crate) fn set_errno(value: c_int) {
    unsafe {
        *errno_ptr() = value;
    }
}

pub(crate) fn as_json_box<'a>(obj: *const json_object) -> Option<&'a JsonObjectBox> {
    unsafe { obj.cast::<JsonObjectBox>().as_ref() }
}

pub(crate) fn as_json_box_mut<'a>(obj: *mut json_object) -> Option<&'a mut JsonObjectBox> {
    unsafe { obj.cast::<JsonObjectBox>().as_mut() }
}

pub(crate) fn string_bytes<'a>(obj: *const json_object) -> Option<&'a [u8]> {
    let inner = as_json_box(obj)?;
    match &inner.data {
        JsonData::String(bytes) => Some(&bytes[..bytes.len().saturating_sub(1)]),
        _ => None,
    }
}

pub(crate) fn object_table(obj: *const json_object) -> *mut lh_table {
    match as_json_box(obj) {
        Some(inner) => match inner.data {
            JsonData::Object { table } => table,
            _ => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

pub(crate) fn array_list_ptr(obj: *const json_object) -> *mut array_list {
    match as_json_box(obj) {
        Some(inner) => match inner.data {
            JsonData::Array { list } => list,
            _ => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

pub(crate) fn default_serializer_for_type(
    o_type: json_type,
) -> Option<json_object_to_json_string_fn> {
    match o_type {
        1 => Some(serializer::json_object_boolean_to_json_string_impl),
        2 => Some(serializer::json_object_double_to_json_string_default_impl),
        3 => Some(serializer::json_object_int_to_json_string_impl),
        4 => Some(serializer::json_object_object_to_json_string_impl),
        5 => Some(serializer::json_object_array_to_json_string_impl),
        6 => Some(serializer::json_object_string_to_json_string_impl),
        _ => None,
    }
}

fn new_json_object(o_type: json_type, data: JsonData) -> *mut json_object {
    Box::into_raw(Box::new(JsonObjectBox {
        o_type,
        ref_count: AtomicU32::new(1),
        to_json_string: default_serializer_for_type(o_type),
        pb: ptr::null_mut(),
        user_delete: None,
        userdata: ptr::null_mut(),
        data,
    }))
    .cast()
}

fn alloc_key_copy(key: *const c_char) -> *mut c_char {
    unsafe { strdup(key) }
}

fn c_string_bytes<'a>(value: *const c_char) -> &'a [u8] {
    unsafe { CStr::from_ptr(value).to_bytes() }
}

fn raw_string_bytes<'a>(value: *const c_char, len: c_int) -> &'a [u8] {
    if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(value.cast::<u8>(), len as usize) }
    }
}

fn table_hash_fn(table: *mut lh_table) -> Option<lh_hash_fn> {
    unsafe { (*table).hash_fn }
}

fn table_head(table: *mut lh_table) -> *mut lh_entry {
    unsafe { (*table).head }
}

fn entry_key(entry: *mut lh_entry) -> *const c_void {
    unsafe { (*entry).k }
}

fn entry_value(entry: *mut lh_entry) -> *mut json_object {
    unsafe { (*entry).v.cast_mut().cast() }
}

fn set_entry_value(entry: *mut lh_entry, value: *mut json_object) {
    unsafe {
        (*entry).v = value.cast();
    }
}

fn entry_next(entry: *mut lh_entry) -> *mut lh_entry {
    unsafe { (*entry).next }
}

fn write_object_out(out: *mut *mut json_object, value: *mut json_object) {
    unsafe {
        *out = value;
    }
}

fn object_out_is_null(out: *mut *mut json_object) -> bool {
    unsafe { (*out).is_null() }
}

fn call_user_delete(
    delete_fn: json_object_delete_fn,
    obj: *mut json_object,
    userdata: *mut c_void,
) {
    unsafe {
        delete_fn(obj, userdata);
    }
}

fn call_abort() -> ! {
    unsafe { abort() }
}

fn call_strtod(input: *const c_char, end_ptr: &mut *mut c_char) -> c_double {
    unsafe { strtod(input, end_ptr) }
}

fn c_char_value(ptr: *const c_char) -> c_char {
    unsafe { *ptr }
}

fn bsearch_result_object(result: *mut c_void) -> *mut json_object {
    unsafe { *result.cast::<*mut json_object>() }
}

fn call_hash_fn(hash_fn: lh_hash_fn, key: *const c_void) -> c_ulong {
    unsafe { hash_fn(key) }
}

fn linkhash_kchar_table_new(size: c_int, free_fn: Option<lh_entry_free_fn>) -> *mut lh_table {
    unsafe { linkhash::lh_kchar_table_new_impl(size, free_fn) }
}

fn linkhash_table_free(table: *mut lh_table) {
    unsafe {
        linkhash::lh_table_free_impl(table);
    }
}

fn linkhash_table_length(table: *mut lh_table) -> c_int {
    unsafe { linkhash::lh_table_length_impl(table) }
}

fn linkhash_lookup_entry_w_hash(
    table: *mut lh_table,
    key: *const c_void,
    hash: c_ulong,
) -> *mut lh_entry {
    unsafe { linkhash::lh_table_lookup_entry_w_hash_impl(table, key, hash) }
}

fn linkhash_insert_w_hash(
    table: *mut lh_table,
    key: *const c_void,
    value: *mut c_void,
    hash: c_ulong,
    opts: c_uint,
) -> c_int {
    unsafe { linkhash::lh_table_insert_w_hash_impl(table, key, value, hash, opts) }
}

fn linkhash_lookup_ex(
    table: *mut lh_table,
    key: *const c_void,
    value_out: *mut *mut c_void,
) -> json_bool {
    unsafe { linkhash::lh_table_lookup_ex_impl(table, key, value_out) }
}

fn linkhash_delete(table: *mut lh_table, key: *const c_void) {
    unsafe {
        linkhash::lh_table_delete_impl(table, key);
    }
}

fn parse_int64_c(buf: *const c_char, retval: *mut int64_t) -> c_int {
    unsafe { crate::numeric::json_parse_int64_impl(buf, retval) }
}

fn parse_uint64_c(buf: *const c_char, retval: *mut uint64_t) -> c_int {
    unsafe { crate::numeric::json_parse_uint64_impl(buf, retval) }
}

fn errno_get() -> c_int {
    unsafe { *errno_ptr() }
}

fn errno_set(value: c_int) {
    unsafe {
        *errno_ptr() = value;
    }
}

fn shallow_copy_call(
    shallow_copy: json_c_shallow_copy_fn,
    src: *mut json_object,
    parent: *mut json_object,
    key_in_parent: *const c_char,
    index_in_parent: size_t,
    dst: *mut *mut json_object,
) -> c_int {
    unsafe { shallow_copy(src, parent, key_in_parent, index_in_parent, dst) }
}

fn read_object_out(out: *mut *mut json_object) -> *mut json_object {
    unsafe { *out }
}

unsafe extern "C" fn json_object_array_entry_free(data: *mut c_void) {
    json_object_put_impl(data.cast());
}

unsafe extern "C" fn json_object_lh_entry_free(entry: *mut lh_entry) {
    if entry.is_null() {
        return;
    }

    if unsafe { (*entry).k_is_constant } == 0 {
        unsafe {
            free(entry_key(entry).cast_mut());
        }
    }
    json_object_put_impl(entry_value(entry));
}

pub(crate) fn json_object_get_impl(obj: *mut json_object) -> *mut json_object {
    let Some(inner) = as_json_box(obj) else {
        return obj;
    };

    let current = inner.ref_count.load(Ordering::Relaxed);
    assert!(current < u32::MAX);
    inner.ref_count.fetch_add(1, Ordering::SeqCst);
    obj
}

fn serializer_matches(
    current: Option<json_object_to_json_string_fn>,
    expected: json_object_to_json_string_fn,
) -> bool {
    current.is_some_and(|func| func as usize == expected as usize)
}

fn is_userdata_serializer(current: Option<json_object_to_json_string_fn>) -> bool {
    serializer_matches(
        current,
        serializer::json_object_userdata_to_json_string_impl,
    ) || serializer_matches(
        current,
        serializer::json_object_userdata_to_json_string_wrapper_impl,
    ) || serializer_matches(current, crate::exports::json_object_userdata_to_json_string)
}

pub(crate) fn json_object_put_impl(obj: *mut json_object) -> c_int {
    if obj.is_null() {
        return 0;
    }

    let inner = as_json_box(obj).expect("json_object pointer must stay valid");
    let current = inner.ref_count.load(Ordering::Relaxed);
    assert!(current > 0);
    if inner.ref_count.fetch_sub(1, Ordering::SeqCst) > 1 {
        return 0;
    }

    let boxed = unsafe { Box::from_raw(obj.cast::<JsonObjectBox>()) };
    if let Some(delete_fn) = boxed.user_delete {
        call_user_delete(delete_fn, obj, boxed.userdata);
    }

    match &boxed.data {
        JsonData::Object { table } => {
            linkhash_table_free(*table);
        }
        JsonData::Array { list } => {
            arraylist::array_list_free_impl(*list);
        }
        JsonData::Boolean(_) | JsonData::Double(_) | JsonData::Int(_) | JsonData::String(_) => {}
    }

    if !boxed.pb.is_null() {
        printbuf_impl::printbuf_free_impl(boxed.pb);
    }

    1
}

pub(crate) fn json_object_is_type_impl(obj: *const json_object, o_type: json_type) -> c_int {
    if obj.is_null() {
        return (o_type == 0) as c_int;
    }
    (as_json_box(obj).expect("valid json_object").o_type == o_type) as c_int
}

pub(crate) fn json_object_get_type_impl(obj: *const json_object) -> json_type {
    as_json_box(obj).map(|inner| inner.o_type).unwrap_or(0)
}

pub(crate) fn json_object_new_object_impl() -> *mut json_object {
    let table = linkhash_kchar_table_new(
        JSON_OBJECT_DEF_HASH_ENTRIES,
        Some(json_object_lh_entry_free),
    );
    if table.is_null() {
        set_errno(ENOMEM);
        return ptr::null_mut();
    }

    new_json_object(4, JsonData::Object { table })
}

pub(crate) fn json_object_get_object_impl(obj: *const json_object) -> *mut lh_table {
    object_table(obj)
}

pub(crate) fn json_object_object_length_impl(obj: *const json_object) -> c_int {
    let table = object_table(obj);
    if table.is_null() {
        return 0;
    }
    linkhash_table_length(table)
}

pub(crate) fn json_c_object_sizeof_impl() -> size_t {
    size_of::<JsonObjectBaseLayout>()
}

pub(crate) fn json_object_object_add_impl(
    obj: *mut json_object,
    key: *const c_char,
    value: *mut json_object,
) -> c_int {
    json_object_object_add_ex_impl(obj, key, value, 0)
}

pub(crate) fn json_object_object_add_ex_impl(
    obj: *mut json_object,
    key: *const c_char,
    value: *mut json_object,
    opts: c_uint,
) -> c_int {
    if obj.is_null() || key.is_null() || json_object_get_type_impl(obj) != 4 {
        return -1;
    }
    if obj == value {
        return -1;
    }

    let table = object_table(obj);
    if table.is_null() {
        return -1;
    }

    let hash_fn = table_hash_fn(table).expect("object table hash function");
    let hash = call_hash_fn(hash_fn, key.cast());
    let existing_entry = if (opts & JSON_C_OBJECT_ADD_KEY_IS_NEW) != 0 {
        ptr::null_mut()
    } else {
        linkhash_lookup_entry_w_hash(table, key.cast(), hash)
    };

    if existing_entry.is_null() {
        let inserted_key = if (opts & JSON_C_OBJECT_ADD_CONSTANT_KEY) != 0 {
            key.cast_mut()
        } else {
            alloc_key_copy(key)
        };
        if inserted_key.is_null() {
            set_errno(ENOMEM);
            return -1;
        }

        return linkhash_insert_w_hash(table, inserted_key.cast(), value.cast(), hash, opts);
    }

    let existing_value = entry_value(existing_entry);
    if !existing_value.is_null() {
        json_object_put_impl(existing_value);
    }
    set_entry_value(existing_entry, value);
    0
}

pub(crate) fn json_object_object_get_impl(
    obj: *const json_object,
    key: *const c_char,
) -> *mut json_object {
    let mut result = ptr::null_mut();
    json_object_object_get_ex_impl(obj, key, &mut result);
    result
}

pub(crate) fn json_object_object_get_ex_impl(
    obj: *const json_object,
    key: *const c_char,
    value_out: *mut *mut json_object,
) -> json_bool {
    if !value_out.is_null() {
        write_object_out(value_out, ptr::null_mut());
    }
    if obj.is_null() || key.is_null() {
        return 0;
    }

    let table = object_table(obj);
    if table.is_null() {
        return 0;
    }

    linkhash_lookup_ex(table, key.cast(), value_out.cast())
}

pub(crate) fn json_object_object_del_impl(obj: *mut json_object, key: *const c_char) {
    if obj.is_null() || key.is_null() {
        return;
    }

    let table = object_table(obj);
    if table.is_null() {
        return;
    }
    linkhash_delete(table, key.cast());
}

pub(crate) fn json_object_new_boolean_impl(value: json_bool) -> *mut json_object {
    new_json_object(1, JsonData::Boolean(value))
}

pub(crate) fn json_object_get_boolean_impl(obj: *const json_object) -> json_bool {
    let Some(inner) = as_json_box(obj) else {
        return 0;
    };

    match &inner.data {
        JsonData::Boolean(value) => *value,
        JsonData::Int(JsonInt::Int64(value)) => (*value != 0) as c_int,
        JsonData::Int(JsonInt::UInt64(value)) => (*value != 0) as c_int,
        JsonData::Double(value) => (*value != 0.0) as c_int,
        JsonData::String(bytes) => (bytes.len() > 1) as c_int,
        JsonData::Object { .. } | JsonData::Array { .. } => 0,
    }
}

pub(crate) fn json_object_set_boolean_impl(obj: *mut json_object, value: json_bool) -> c_int {
    let Some(inner) = as_json_box_mut(obj) else {
        return 0;
    };
    match &mut inner.data {
        JsonData::Boolean(slot) => {
            *slot = value;
            1
        }
        _ => 0,
    }
}

pub(crate) fn json_object_new_int_impl(value: int32_t) -> *mut json_object {
    json_object_new_int64_impl(value as int64_t)
}

pub(crate) fn json_object_new_int64_impl(value: int64_t) -> *mut json_object {
    new_json_object(3, JsonData::Int(JsonInt::Int64(value)))
}

pub(crate) fn json_object_new_uint64_impl(value: uint64_t) -> *mut json_object {
    new_json_object(3, JsonData::Int(JsonInt::UInt64(value)))
}

fn saturating_int32(value: int64_t) -> int32_t {
    if value <= i32::MIN as i64 {
        i32::MIN
    } else if value >= i32::MAX as i64 {
        i32::MAX
    } else {
        value as i32
    }
}

pub(crate) fn json_object_get_int_impl(obj: *const json_object) -> int32_t {
    let Some(inner) = as_json_box(obj) else {
        return 0;
    };

    match &inner.data {
        JsonData::Int(JsonInt::Int64(value)) => saturating_int32(*value),
        JsonData::Int(JsonInt::UInt64(value)) => {
            let capped = if *value >= i64::MAX as u64 {
                i64::MAX
            } else {
                *value as i64
            };
            saturating_int32(capped)
        }
        JsonData::Double(value) => {
            if value.is_nan() {
                0
            } else if *value <= i32::MIN as f64 {
                i32::MIN
            } else if *value >= i32::MAX as f64 {
                i32::MAX
            } else {
                *value as i32
            }
        }
        JsonData::Boolean(value) => *value,
        JsonData::String(_) => {
            let mut parsed = 0_i64;
            if parse_int64_c(json_object_get_string_impl(obj.cast_mut()), &mut parsed) != 0 {
                return 0;
            }
            saturating_int32(parsed)
        }
        JsonData::Object { .. } | JsonData::Array { .. } => 0,
    }
}

pub(crate) fn json_object_get_int64_impl(obj: *const json_object) -> int64_t {
    let Some(inner) = as_json_box(obj) else {
        return 0;
    };

    match &inner.data {
        JsonData::Int(JsonInt::Int64(value)) => *value,
        JsonData::Int(JsonInt::UInt64(value)) => {
            if *value >= i64::MAX as u64 {
                i64::MAX
            } else {
                *value as i64
            }
        }
        JsonData::Double(value) => {
            if value.is_nan() {
                0
            } else if *value >= i64::MAX as f64 {
                i64::MAX
            } else if *value <= i64::MIN as f64 {
                i64::MIN
            } else {
                *value as i64
            }
        }
        JsonData::Boolean(value) => (*value).into(),
        JsonData::String(_) => {
            let mut parsed = 0_i64;
            if parse_int64_c(json_object_get_string_impl(obj.cast_mut()), &mut parsed) != 0 {
                return 0;
            }
            parsed
        }
        JsonData::Object { .. } | JsonData::Array { .. } => 0,
    }
}

pub(crate) fn json_object_get_uint64_impl(obj: *const json_object) -> uint64_t {
    let Some(inner) = as_json_box(obj) else {
        return 0;
    };

    match &inner.data {
        JsonData::Int(JsonInt::Int64(value)) => {
            if *value < 0 {
                0
            } else {
                *value as u64
            }
        }
        JsonData::Int(JsonInt::UInt64(value)) => *value,
        JsonData::Double(value) => {
            if value.is_nan() || *value < 0.0 {
                0
            } else if *value >= u64::MAX as f64 {
                u64::MAX
            } else {
                *value as u64
            }
        }
        JsonData::Boolean(value) => (*value != 0) as u64,
        JsonData::String(_) => {
            let mut parsed = 0_u64;
            if parse_uint64_c(json_object_get_string_impl(obj.cast_mut()), &mut parsed) != 0 {
                return 0;
            }
            parsed
        }
        JsonData::Object { .. } | JsonData::Array { .. } => 0,
    }
}

pub(crate) fn json_object_set_int_impl(obj: *mut json_object, value: c_int) -> c_int {
    json_object_set_int64_impl(obj, value as int64_t)
}

pub(crate) fn json_object_set_int64_impl(obj: *mut json_object, value: int64_t) -> c_int {
    let Some(inner) = as_json_box_mut(obj) else {
        return 0;
    };
    match &mut inner.data {
        JsonData::Int(slot) => {
            *slot = JsonInt::Int64(value);
            1
        }
        _ => 0,
    }
}

pub(crate) fn json_object_set_uint64_impl(obj: *mut json_object, value: uint64_t) -> c_int {
    let Some(inner) = as_json_box_mut(obj) else {
        return 0;
    };
    match &mut inner.data {
        JsonData::Int(slot) => {
            *slot = JsonInt::UInt64(value);
            1
        }
        _ => 0,
    }
}

pub(crate) fn json_object_int_inc_impl(obj: *mut json_object, value: int64_t) -> c_int {
    let Some(inner) = as_json_box_mut(obj) else {
        return 0;
    };
    let JsonData::Int(slot) = &mut inner.data else {
        return 0;
    };

    match slot {
        JsonInt::Int64(current) => {
            if value > 0 && *current > i64::MAX - value {
                *slot = JsonInt::UInt64((*current as u64).wrapping_add(value as u64));
            } else if value < 0 && *current < i64::MIN - value {
                *current = i64::MIN;
            } else {
                *current += value;
            }
        }
        JsonInt::UInt64(current) => {
            if value > 0 && *current > u64::MAX - value as u64 {
                *current = u64::MAX;
            } else if value < 0 && *current < (-value) as u64 {
                *slot = JsonInt::Int64(*current as i64 + value);
            } else if value < 0 {
                *current -= (-value) as u64;
            } else {
                *current += value as u64;
            }
        }
    }

    1
}

pub(crate) fn json_object_new_double_impl(value: c_double) -> *mut json_object {
    new_json_object(2, JsonData::Double(value))
}

pub(crate) fn json_object_new_double_s_impl(
    value: c_double,
    serialized: *const c_char,
) -> *mut json_object {
    if serialized.is_null() {
        return ptr::null_mut();
    }

    let obj = json_object_new_double_impl(value);
    if obj.is_null() {
        return ptr::null_mut();
    }

    let duplicated = unsafe { strdup(serialized) };
    if duplicated.is_null() {
        json_object_put_impl(obj);
        set_errno(ENOMEM);
        return ptr::null_mut();
    }

    serializer::json_object_set_serializer_impl(
        obj,
        Some(serializer::json_object_userdata_to_json_string_wrapper_impl),
        duplicated.cast(),
        Some(serializer::json_object_free_userdata_impl),
    );
    obj
}

pub(crate) fn json_object_get_double_impl(obj: *const json_object) -> c_double {
    let Some(inner) = as_json_box(obj) else {
        return 0.0;
    };

    match &inner.data {
        JsonData::Double(value) => *value,
        JsonData::Int(JsonInt::Int64(value)) => *value as f64,
        JsonData::Int(JsonInt::UInt64(value)) => *value as f64,
        JsonData::Boolean(value) => *value as f64,
        JsonData::String(_) => {
            let input = json_object_get_string_impl(obj.cast_mut());
            let mut end_ptr = ptr::null_mut();
            errno_set(0);
            let mut parsed = call_strtod(input, &mut end_ptr);
            if end_ptr == input.cast_mut() {
                errno_set(EINVAL);
                return 0.0;
            }
            if c_char_value(end_ptr) != 0 {
                errno_set(EINVAL);
                return 0.0;
            }
            if parsed.is_infinite() && errno_get() == 34 {
                parsed = 0.0;
            }
            parsed
        }
        JsonData::Object { .. } | JsonData::Array { .. } => {
            errno_set(EINVAL);
            0.0
        }
    }
}

pub(crate) fn json_object_set_double_impl(obj: *mut json_object, value: c_double) -> c_int {
    let Some(inner) = as_json_box_mut(obj) else {
        return 0;
    };
    let JsonData::Double(slot) = &mut inner.data else {
        return 0;
    };

    *slot = value;
    if serializer_matches(
        inner.to_json_string,
        serializer::json_object_userdata_to_json_string_wrapper_impl,
    ) {
        serializer::json_object_set_serializer_impl(obj, None, ptr::null_mut(), None);
    }
    1
}

fn with_nul(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 1);
    out.extend_from_slice(bytes);
    out.push(0);
    out
}

pub(crate) fn json_object_new_string_impl(value: *const c_char) -> *mut json_object {
    if value.is_null() {
        return new_json_object(6, JsonData::String(vec![0]));
    }
    json_object_new_string_len_impl(value, c_string_bytes(value).len() as c_int)
}

pub(crate) fn json_object_new_string_len_impl(
    value: *const c_char,
    len: c_int,
) -> *mut json_object {
    if len < 0 || (value.is_null() && len != 0) {
        return ptr::null_mut();
    }

    new_json_object(6, JsonData::String(with_nul(raw_string_bytes(value, len))))
}

pub(crate) fn json_object_get_string_impl(obj: *mut json_object) -> *const c_char {
    if obj.is_null() {
        return ptr::null();
    }

    let inner = as_json_box(obj).expect("valid json_object");
    match &inner.data {
        JsonData::String(bytes) => bytes.as_ptr().cast(),
        _ => serializer::json_object_to_json_string_impl(obj),
    }
}

pub(crate) fn json_object_get_string_len_impl(obj: *const json_object) -> c_int {
    string_bytes(obj)
        .map(|bytes| bytes.len() as c_int)
        .unwrap_or(0)
}

pub(crate) fn json_object_set_string_impl(obj: *mut json_object, value: *const c_char) -> c_int {
    if value.is_null() {
        return json_object_set_string_len_impl(obj, ptr::null(), 0);
    }
    json_object_set_string_len_impl(obj, value, c_string_bytes(value).len() as c_int)
}

pub(crate) fn json_object_set_string_len_impl(
    obj: *mut json_object,
    value: *const c_char,
    len: c_int,
) -> c_int {
    if len < 0 || (value.is_null() && len != 0) {
        return 0;
    }

    let Some(inner) = as_json_box_mut(obj) else {
        return 0;
    };
    match &mut inner.data {
        JsonData::String(bytes) => {
            *bytes = with_nul(raw_string_bytes(value, len));
            1
        }
        _ => 0,
    }
}

pub(crate) fn json_object_new_array_impl() -> *mut json_object {
    json_object_new_array_ext_impl(ARRAY_LIST_DEFAULT_SIZE)
}

pub(crate) fn json_object_new_array_ext_impl(initial_size: c_int) -> *mut json_object {
    if initial_size < 0 {
        return ptr::null_mut();
    }

    let list = arraylist::array_list_new2_impl(Some(json_object_array_entry_free), initial_size);
    if list.is_null() {
        set_errno(ENOMEM);
        return ptr::null_mut();
    }

    new_json_object(5, JsonData::Array { list })
}

pub(crate) fn json_object_get_array_impl(obj: *const json_object) -> *mut array_list {
    array_list_ptr(obj)
}

pub(crate) fn json_object_array_length_impl(obj: *const json_object) -> size_t {
    let list = array_list_ptr(obj);
    if list.is_null() {
        return 0;
    }
    arraylist::array_list_length_impl(list)
}

pub(crate) fn json_object_array_add_impl(obj: *mut json_object, value: *mut json_object) -> c_int {
    let list = array_list_ptr(obj);
    if list.is_null() {
        return -1;
    }
    arraylist::array_list_add_impl(list, value.cast())
}

pub(crate) fn json_object_array_insert_idx_impl(
    obj: *mut json_object,
    idx: size_t,
    value: *mut json_object,
) -> c_int {
    let list = array_list_ptr(obj);
    if list.is_null() {
        return -1;
    }
    arraylist::array_list_insert_idx_impl(list, idx, value.cast())
}

pub(crate) fn json_object_array_put_idx_impl(
    obj: *mut json_object,
    idx: size_t,
    value: *mut json_object,
) -> c_int {
    let list = array_list_ptr(obj);
    if list.is_null() {
        return -1;
    }
    arraylist::array_list_put_idx_impl(list, idx, value.cast())
}

pub(crate) fn json_object_array_del_idx_impl(
    obj: *mut json_object,
    idx: size_t,
    count: size_t,
) -> c_int {
    let list = array_list_ptr(obj);
    if list.is_null() {
        return -1;
    }
    arraylist::array_list_del_idx_impl(list, idx, count)
}

pub(crate) fn json_object_array_get_idx_impl(
    obj: *const json_object,
    idx: size_t,
) -> *mut json_object {
    let list = array_list_ptr(obj);
    if list.is_null() {
        return ptr::null_mut();
    }
    arraylist::array_list_get_idx_impl(list, idx).cast()
}

pub(crate) fn json_object_array_shrink_impl(obj: *mut json_object, empty_slots: c_int) -> c_int {
    if empty_slots < 0 {
        return -1;
    }
    let list = array_list_ptr(obj);
    if list.is_null() {
        return -1;
    }
    arraylist::array_list_shrink_impl(list, empty_slots as size_t)
}

pub(crate) fn json_object_array_sort_impl(obj: *mut json_object, sort_fn: Option<comparison_fn>) {
    let list = array_list_ptr(obj);
    if list.is_null() {
        return;
    }
    arraylist::array_list_sort_impl(list, sort_fn);
}

pub(crate) fn json_object_array_bsearch_impl(
    key: *const json_object,
    obj: *const json_object,
    sort_fn: Option<comparison_fn>,
) -> *mut json_object {
    let list = array_list_ptr(obj);
    if list.is_null() || key.is_null() {
        return ptr::null_mut();
    }

    let mut key_void = key.cast::<c_void>();
    let result = arraylist::array_list_bsearch_impl(&mut key_void, list, sort_fn);
    if result.is_null() {
        ptr::null_mut()
    } else {
        bsearch_result_object(result)
    }
}

pub(crate) fn json_object_new_null_impl() -> *mut json_object {
    ptr::null_mut()
}

fn strings_equal(left: &[u8], right: &[u8]) -> bool {
    left == right
}

fn int_equal(left: JsonInt, right: JsonInt) -> bool {
    match (left, right) {
        (JsonInt::Int64(a), JsonInt::Int64(b)) => a == b,
        (JsonInt::UInt64(a), JsonInt::UInt64(b)) => a == b,
        (JsonInt::Int64(a), JsonInt::UInt64(b)) | (JsonInt::UInt64(b), JsonInt::Int64(a)) => {
            if a < 0 {
                false
            } else {
                a as u64 == b
            }
        }
    }
}

fn objects_equal(left: *mut lh_table, right: *mut lh_table) -> bool {
    if linkhash_table_length(left) != linkhash_table_length(right) {
        return false;
    }

    let mut entry = table_head(left);
    while !entry.is_null() {
        let key = entry_key(entry);
        let mut other = ptr::null_mut();
        if linkhash_lookup_ex(right, key, &mut other) == 0 {
            return false;
        }
        if json_object_equal_impl(entry_value(entry), other.cast()) == 0 {
            return false;
        }
        entry = entry_next(entry);
    }

    let mut entry = table_head(right);
    while !entry.is_null() {
        let key = entry_key(entry);
        let mut other = ptr::null_mut();
        if linkhash_lookup_ex(left, key, &mut other) == 0 {
            return false;
        }
        entry = entry_next(entry);
    }

    true
}

fn arrays_equal(left: *mut array_list, right: *mut array_list) -> bool {
    let len = arraylist::array_list_length_impl(left);
    if len != arraylist::array_list_length_impl(right) {
        return false;
    }

    for idx in 0..len {
        if json_object_equal_impl(
            arraylist::array_list_get_idx_impl(left, idx).cast(),
            arraylist::array_list_get_idx_impl(right, idx).cast(),
        ) == 0
        {
            return false;
        }
    }
    true
}

pub(crate) fn json_object_equal_impl(left: *mut json_object, right: *mut json_object) -> c_int {
    if left == right {
        return 1;
    }
    if left.is_null() || right.is_null() {
        return 0;
    }

    let left_inner = as_json_box(left).expect("valid left object");
    let right_inner = as_json_box(right).expect("valid right object");
    if left_inner.o_type != right_inner.o_type {
        return 0;
    }

    let equal = match (&left_inner.data, &right_inner.data) {
        (JsonData::Boolean(a), JsonData::Boolean(b)) => *a == *b,
        (JsonData::Double(a), JsonData::Double(b)) => *a == *b,
        (JsonData::Int(a), JsonData::Int(b)) => int_equal(*a, *b),
        (JsonData::String(a), JsonData::String(b)) => strings_equal(
            &a[..a.len().saturating_sub(1)],
            &b[..b.len().saturating_sub(1)],
        ),
        (JsonData::Object { table: a }, JsonData::Object { table: b }) => objects_equal(*a, *b),
        (JsonData::Array { list: a }, JsonData::Array { list: b }) => arrays_equal(*a, *b),
        _ => false,
    };

    equal as c_int
}

fn copy_serializer_data(src: *mut json_object, dst: *mut json_object) -> c_int {
    let src_inner = as_json_box(src).expect("valid source object");
    let dst_inner = as_json_box_mut(dst).expect("valid destination object");

    if src_inner.userdata.is_null() && src_inner.user_delete.is_none() {
        return 0;
    }

    if is_userdata_serializer(dst_inner.to_json_string) {
        if src_inner.userdata.is_null() {
            call_abort();
        }

        let duplicated = unsafe { strdup(src_inner.userdata.cast()) };
        if duplicated.is_null() {
            errors::set_last_err_fmt(format_args!(
                "json_object_copy_serializer_data: out of memory\n"
            ));
            return -1;
        }
        dst_inner.userdata = duplicated.cast();
        dst_inner.user_delete = src_inner.user_delete;
        return 0;
    }

    if src_inner.userdata.is_null() && src_inner.user_delete.is_none() {
        return 0;
    }

    let serializer_ptr = dst_inner
        .to_json_string
        .map(|func| func as *const c_void)
        .unwrap_or(ptr::null());
    errors::set_last_err_fmt(format_args!(
        "json_object_copy_serializer_data: unable to copy unknown serializer data: {:p}\n",
        serializer_ptr
    ));
    -1
}

pub(crate) unsafe extern "C" fn json_c_shallow_copy_default_impl(
    src: *mut json_object,
    _parent: *mut json_object,
    _key: *const c_char,
    _index: size_t,
    dst: *mut *mut json_object,
) -> c_int {
    if src.is_null() || dst.is_null() {
        set_errno(EINVAL);
        return -1;
    }

    let src_inner = as_json_box(src).expect("valid source object");
    let created = match &src_inner.data {
        JsonData::Boolean(value) => json_object_new_boolean_impl(*value),
        JsonData::Double(value) => json_object_new_double_impl(*value),
        JsonData::Int(JsonInt::Int64(value)) => json_object_new_int64_impl(*value),
        JsonData::Int(JsonInt::UInt64(value)) => json_object_new_uint64_impl(*value),
        JsonData::String(bytes) => new_json_object(6, JsonData::String(bytes.clone())),
        JsonData::Object { .. } => json_object_new_object_impl(),
        JsonData::Array { .. } => json_object_new_array_impl(),
    };

    if created.is_null() {
        set_errno(ENOMEM);
        return -1;
    }

    as_json_box_mut(created)
        .expect("new object must be valid")
        .to_json_string = src_inner.to_json_string;
    *dst = created;
    1
}

fn json_object_deep_copy_recursive(
    src: *mut json_object,
    parent: *mut json_object,
    key_in_parent: *const c_char,
    index_in_parent: size_t,
    dst: *mut *mut json_object,
    shallow_copy: json_c_shallow_copy_fn,
) -> c_int {
    let shallow_rc = shallow_copy_call(
        shallow_copy,
        src,
        parent,
        key_in_parent,
        index_in_parent,
        dst,
    );
    if shallow_rc < 1 {
        set_errno(EINVAL);
        return -1;
    }

    let Some(src_inner) = as_json_box(src) else {
        set_errno(EINVAL);
        return -1;
    };

    match &src_inner.data {
        JsonData::Object { table } => {
            let mut entry = table_head(*table);
            while !entry.is_null() {
                let child = entry_value(entry);
                let mut copied = ptr::null_mut();
                if child.is_null() {
                    copied = ptr::null_mut();
                } else if json_object_deep_copy_recursive(
                    child,
                    src,
                    entry_key(entry).cast(),
                    usize::MAX,
                    &mut copied,
                    shallow_copy,
                ) < 0
                {
                    json_object_put_impl(copied);
                    return -1;
                }

                if json_object_object_add_impl(
                    read_object_out(dst),
                    entry_key(entry).cast(),
                    copied,
                ) < 0
                {
                    json_object_put_impl(copied);
                    return -1;
                }

                entry = entry_next(entry);
            }
        }
        JsonData::Array { list } => {
            let len = arraylist::array_list_length_impl(*list);
            for idx in 0..len {
                let child = arraylist::array_list_get_idx_impl(*list, idx).cast::<json_object>();
                let mut copied = ptr::null_mut();
                if child.is_null() {
                    copied = ptr::null_mut();
                } else if json_object_deep_copy_recursive(
                    child,
                    src,
                    ptr::null(),
                    idx,
                    &mut copied,
                    shallow_copy,
                ) < 0
                {
                    json_object_put_impl(copied);
                    return -1;
                }

                if json_object_array_add_impl(read_object_out(dst), copied) < 0 {
                    json_object_put_impl(copied);
                    return -1;
                }
            }
        }
        JsonData::Boolean(_) | JsonData::Double(_) | JsonData::Int(_) | JsonData::String(_) => {}
    }

    if shallow_rc != 2 {
        return copy_serializer_data(src, read_object_out(dst));
    }
    0
}

pub(crate) fn json_object_deep_copy_impl(
    src: *mut json_object,
    dst: *mut *mut json_object,
    shallow_copy: Option<json_c_shallow_copy_fn>,
) -> c_int {
    if src.is_null() || dst.is_null() || !object_out_is_null(dst) {
        set_errno(EINVAL);
        return -1;
    }

    let shallow = shallow_copy.unwrap_or(json_c_shallow_copy_default_impl);
    let rc = json_object_deep_copy_recursive(
        src,
        ptr::null_mut(),
        ptr::null(),
        usize::MAX,
        dst,
        shallow,
    );
    if rc < 0 {
        json_object_put_impl(read_object_out(dst));
        write_object_out(dst, ptr::null_mut());
    }
    rc
}
