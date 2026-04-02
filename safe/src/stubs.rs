// Generated from abi/public-api-manifest.tsv during impl_01_scaffold_surface.
#![allow(clippy::missing_safety_doc)]
#![allow(unused_variables)]

use crate::abi::*;

const JSON_C_VERSION_BYTES: &[u8; 5] = b"0.17\0";
const JSON_C_STRERROR_STUB: &[u8; 8] = b"ERRNO=0\0";
const JSON_NUMBER_CHARS_BYTES: &[u8; 16] = b"0123456789.+-eE\0";
const JSON_HEX_CHARS_BYTES: &[u8; 23] = b"0123456789abcdefABCDEF\0";

#[no_mangle]
pub static mut json_number_chars: *const c_char = JSON_NUMBER_CHARS_BYTES.as_ptr().cast();
#[no_mangle]
pub static mut json_hex_chars: *const c_char = JSON_HEX_CHARS_BYTES.as_ptr().cast();

#[no_mangle]
pub unsafe extern "C" fn array_list_add(_arg0: *mut array_list, _arg1: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn array_list_bsearch(_arg0: *mut *const c_void, _arg1: *mut array_list, _arg2: Option<comparison_fn>) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn array_list_del_idx(_arg0: *mut array_list, _arg1: size_t, _arg2: size_t) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn array_list_free(_arg0: *mut array_list) {
}

#[no_mangle]
pub unsafe extern "C" fn array_list_get_idx(_arg0: *mut array_list, _arg1: size_t) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn array_list_insert_idx(_arg0: *mut array_list, _arg1: size_t, _arg2: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn array_list_length(_arg0: *mut array_list) -> size_t {
    0
}

#[no_mangle]
pub unsafe extern "C" fn array_list_new(_arg0: Option<array_list_free_fn>) -> *mut array_list {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn array_list_new2(_arg0: Option<array_list_free_fn>, _arg1: c_int) -> *mut array_list {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn array_list_put_idx(_arg0: *mut array_list, _arg1: size_t, _arg2: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn array_list_shrink(_arg0: *mut array_list, _arg1: size_t) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn array_list_sort(_arg0: *mut array_list, _arg1: Option<comparison_fn>) {
}

#[no_mangle]
pub unsafe extern "C" fn json_c_get_random_seed() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_c_set_serialization_double_format(_arg0: *const c_char, _arg1: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_c_shallow_copy_default(_arg0: *mut json_object, _arg1: *mut json_object, _arg2: *const c_char, _arg3: size_t, _arg4: *mut *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_c_visit(_arg0: *mut json_object, _arg1: c_int, _arg2: Option<json_c_visit_userfunc>, _arg3: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_global_set_string_hash(_arg0: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_array_add(_arg0: *mut json_object, _arg1: *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_array_bsearch(_arg0: *const json_object, _arg1: *const json_object, _arg2: Option<comparison_fn>) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_array_del_idx(_arg0: *mut json_object, _arg1: size_t, _arg2: size_t) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_array_get_idx(_arg0: *const json_object, _arg1: size_t) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_array_insert_idx(_arg0: *mut json_object, _arg1: size_t, _arg2: *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_array_length(_arg0: *const json_object) -> size_t {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_array_put_idx(_arg0: *mut json_object, _arg1: size_t, _arg2: *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_array_shrink(_arg0: *mut json_object, _arg1: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_array_sort(_arg0: *mut json_object, _arg1: Option<comparison_fn>) {
}

#[no_mangle]
pub unsafe extern "C" fn json_object_deep_copy(_arg0: *mut json_object, _arg1: *mut *mut json_object, _arg2: Option<json_c_shallow_copy_fn>) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_double_to_json_string(_arg0: *mut json_object, _arg1: *mut printbuf, _arg2: c_int, _arg3: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_equal(_arg0: *mut json_object, _arg1: *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_free_userdata(_arg0: *mut json_object, _arg1: *mut c_void) {
}

#[no_mangle]
pub unsafe extern "C" fn json_object_from_fd(_arg0: c_int) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_from_fd_ex(_arg0: c_int, _arg1: c_int) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_from_file(_arg0: *const c_char) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get(_arg0: *mut json_object) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_array(_arg0: *const json_object) -> *mut array_list {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_boolean(_arg0: *const json_object) -> json_bool {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_double(_arg0: *const json_object) -> c_double {
    0.0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_int(_arg0: *const json_object) -> int32_t {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_int64(_arg0: *const json_object) -> int64_t {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_object(_arg0: *const json_object) -> *mut lh_table {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_string(_arg0: *mut json_object) -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_string_len(_arg0: *const json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_type(_arg0: *const json_object) -> json_type {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_uint64(_arg0: *const json_object) -> uint64_t {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_get_userdata(_arg0: *mut json_object) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_int_inc(_arg0: *mut json_object, _arg1: int64_t) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_is_type(_arg0: *const json_object, _arg1: json_type) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_iter_begin(_arg0: *mut json_object) -> json_object_iterator {
    json_object_iterator { opaque_: std::ptr::null() }
}

#[no_mangle]
pub unsafe extern "C" fn json_object_iter_end(_arg0: *const json_object) -> json_object_iterator {
    json_object_iterator { opaque_: std::ptr::null() }
}

#[no_mangle]
pub unsafe extern "C" fn json_object_iter_equal(_arg0: *const json_object_iterator, _arg1: *const json_object_iterator) -> json_bool {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_iter_init_default() -> json_object_iterator {
    json_object_iterator { opaque_: std::ptr::null() }
}

#[no_mangle]
pub unsafe extern "C" fn json_object_iter_next(_arg0: *mut json_object_iterator) {
}

#[no_mangle]
pub unsafe extern "C" fn json_object_iter_peek_name(_arg0: *const json_object_iterator) -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_iter_peek_value(_arg0: *const json_object_iterator) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_array() -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_array_ext(_arg0: c_int) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_boolean(_arg0: json_bool) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_double(_arg0: c_double) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_double_s(_arg0: c_double, _arg1: *const c_char) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_int(_arg0: int32_t) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_int64(_arg0: int64_t) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_null() -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_object() -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_string(_arg0: *const c_char) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_string_len(_arg0: *const c_char, _arg1: c_int) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_new_uint64(_arg0: uint64_t) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_object_add(_arg0: *mut json_object, _arg1: *const c_char, _arg2: *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_object_add_ex(_arg0: *mut json_object, _arg1: *const c_char, _arg2: *mut json_object, _arg3: c_uint) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_object_del(_arg0: *mut json_object, _arg1: *const c_char) {
}

#[no_mangle]
pub unsafe extern "C" fn json_object_object_get(_arg0: *const json_object, _arg1: *const c_char) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_object_get_ex(_arg0: *const json_object, _arg1: *const c_char, _arg2: *mut *mut json_object) -> json_bool {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_object_length(_arg0: *const json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_put(_arg0: *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_set_boolean(_arg0: *mut json_object, _arg1: json_bool) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_set_double(_arg0: *mut json_object, _arg1: c_double) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_set_int(_arg0: *mut json_object, _arg1: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_set_int64(_arg0: *mut json_object, _arg1: int64_t) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_set_serializer(_arg0: *mut json_object, _arg1: Option<json_object_to_json_string_fn>, _arg2: *mut c_void, _arg3: Option<json_object_delete_fn>) {
}

#[no_mangle]
pub unsafe extern "C" fn json_object_set_string(_arg0: *mut json_object, _arg1: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_set_string_len(_arg0: *mut json_object, _arg1: *const c_char, _arg2: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_set_uint64(_arg0: *mut json_object, _arg1: uint64_t) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_set_userdata(_arg0: *mut json_object, _arg1: *mut c_void, _arg2: Option<json_object_delete_fn>) {
}

#[no_mangle]
pub unsafe extern "C" fn json_object_to_fd(_arg0: c_int, _arg1: *mut json_object, _arg2: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_to_file(_arg0: *const c_char, _arg1: *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_to_file_ext(_arg0: *const c_char, _arg1: *mut json_object, _arg2: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_object_to_json_string(_arg0: *mut json_object) -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_to_json_string_ext(_arg0: *mut json_object, _arg1: c_int) -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_to_json_string_length(_arg0: *mut json_object, _arg1: c_int, _arg2: *mut size_t) -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn json_object_userdata_to_json_string(_arg0: *mut json_object, _arg1: *mut printbuf, _arg2: c_int, _arg3: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_parse_double(_arg0: *const c_char, _arg1: *mut c_double) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_parse_int64(_arg0: *const c_char, _arg1: *mut int64_t) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_parse_uint64(_arg0: *const c_char, _arg1: *mut uint64_t) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_patch_apply(_arg0: *mut json_object, _arg1: *mut json_object, _arg2: *mut *mut json_object, _arg3: *mut json_patch_error) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_pointer_get(_arg0: *mut json_object, _arg1: *const c_char, _arg2: *mut *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_pointer_set(_arg0: *mut *mut json_object, _arg1: *const c_char, _arg2: *mut json_object) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_error_desc(_arg0: json_tokener_error) -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_free(_arg0: *mut json_tokener) {
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_get_error(_arg0: *mut json_tokener) -> json_tokener_error {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_get_parse_end(_arg0: *mut json_tokener) -> size_t {
    0
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_new() -> *mut json_tokener {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_new_ex(_arg0: c_int) -> *mut json_tokener {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_parse(_arg0: *const c_char) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_parse_ex(_arg0: *mut json_tokener, _arg1: *const c_char, _arg2: c_int) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_parse_verbose(_arg0: *const c_char, _arg1: *mut json_tokener_error) -> *mut json_object {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_reset(_arg0: *mut json_tokener) {
}

#[no_mangle]
pub unsafe extern "C" fn json_tokener_set_flags(_arg0: *mut json_tokener, _arg1: c_int) {
}

#[no_mangle]
pub unsafe extern "C" fn json_type_to_name(_arg0: json_type) -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn json_util_get_last_err() -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn lh_char_equal(_arg0: *const c_void, _arg1: *const c_void) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn lh_kchar_table_new(_arg0: c_int, _arg1: Option<lh_entry_free_fn>) -> *mut lh_table {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn lh_kptr_table_new(_arg0: c_int, _arg1: Option<lh_entry_free_fn>) -> *mut lh_table {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn lh_ptr_equal(_arg0: *const c_void, _arg1: *const c_void) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_delete(_arg0: *mut lh_table, _arg1: *const c_void) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_delete_entry(_arg0: *mut lh_table, _arg1: *mut lh_entry) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_free(_arg0: *mut lh_table) {
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_insert(_arg0: *mut lh_table, _arg1: *const c_void, _arg2: *const c_void) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_insert_w_hash(_arg0: *mut lh_table, _arg1: *const c_void, _arg2: *const c_void, _arg3: c_ulong, _arg4: c_uint) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_length(_arg0: *mut lh_table) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_lookup_entry(_arg0: *mut lh_table, _arg1: *const c_void) -> *mut lh_entry {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_lookup_entry_w_hash(_arg0: *mut lh_table, _arg1: *const c_void, _arg2: c_ulong) -> *mut lh_entry {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_lookup_ex(_arg0: *mut lh_table, _arg1: *const c_void, _arg2: *mut *mut c_void) -> json_bool {
    0
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_new(_arg0: c_int, _arg1: Option<lh_entry_free_fn>, _arg2: Option<lh_hash_fn>, _arg3: Option<lh_equal_fn>) -> *mut lh_table {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn lh_table_resize(_arg0: *mut lh_table, _arg1: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn mc_get_debug() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn mc_set_debug(_arg0: c_int) {
}

#[no_mangle]
pub unsafe extern "C" fn mc_set_syslog(_arg0: c_int) {
}

#[no_mangle]
pub unsafe extern "C" fn printbuf_free(_arg0: *mut printbuf) {
}

#[no_mangle]
pub unsafe extern "C" fn printbuf_memappend(_arg0: *mut printbuf, _arg1: *const c_char, _arg2: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn printbuf_memset(_arg0: *mut printbuf, _arg1: c_int, _arg2: c_int, _arg3: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn printbuf_new() -> *mut printbuf {
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn printbuf_reset(_arg0: *mut printbuf) {
}

#[no_mangle]
pub unsafe extern "C" fn json_c_version() -> *const c_char {
    JSON_C_VERSION_BYTES.as_ptr().cast()
}

#[no_mangle]
pub unsafe extern "C" fn json_c_version_num() -> c_int {
    0x0000_1100
}

#[no_mangle]
pub unsafe extern "C" fn json_c_object_sizeof() -> size_t {
    40
}

#[no_mangle]
pub unsafe extern "C" fn _json_c_strerror(_arg0: c_int) -> *mut c_char {
    JSON_C_STRERROR_STUB.as_ptr() as *mut c_char
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(r#"
.globl json_pointer_getf
.type json_pointer_getf, @function
json_pointer_getf:
    xor eax, eax
    ret

.globl json_pointer_setf
.type json_pointer_setf, @function
json_pointer_setf:
    xor eax, eax
    ret

.globl mc_debug
.type mc_debug, @function
mc_debug:
    ret

.globl mc_error
.type mc_error, @function
mc_error:
    ret

.globl mc_info
.type mc_info, @function
mc_info:
    ret

.globl sprintbuf
.type sprintbuf, @function
sprintbuf:
    xor eax, eax
    ret

"#);

#[cfg(not(target_arch = "x86_64"))]
compile_error!("Phase 1 variadic stubs are only implemented for x86_64 targets.");
