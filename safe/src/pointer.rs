use crate::abi::*;
use crate::object;
use std::ffi::CStr;
use std::ptr;

const EINVAL: c_int = 22;
const ENOENT: c_int = 2;
const JSON_TYPE_ARRAY: json_type = 5;
const JSON_TYPE_OBJECT: json_type = 4;

pub(crate) struct JsonPointerGetResult {
    pub parent: *mut json_object,
    pub obj: *mut json_object,
    pub key_in_parent: Option<Vec<c_char>>,
    pub index_in_parent: size_t,
}

pub(crate) type JsonPointerArraySetCb =
    unsafe fn(*mut json_object, size_t, *mut json_object, *mut c_void) -> c_int;

fn is_plain_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

fn replace_all_occurrences_with_char(bytes: &mut Vec<u8>, occur: &[u8], repl: u8) {
    let mut idx = 0usize;
    while idx + occur.len() <= bytes.len() {
        if &bytes[idx..idx + occur.len()] == occur {
            bytes[idx] = repl;
            bytes.drain(idx + 1..idx + occur.len());
            idx += 1;
        } else {
            idx += 1;
        }
    }
}

fn nul_terminated(bytes: Vec<u8>) -> Vec<c_char> {
    let mut out = Vec::with_capacity(bytes.len() + 1);
    out.extend(bytes.into_iter().map(|byte| byte as c_char));
    out.push(0);
    out
}

fn unescape_object_path(segment: &[u8]) -> Vec<c_char> {
    let mut bytes = segment.to_vec();
    replace_all_occurrences_with_char(&mut bytes, b"~1", b'/');
    replace_all_occurrences_with_char(&mut bytes, b"~0", b'~');
    nul_terminated(bytes)
}

fn literal_object_path(segment: &[u8]) -> Vec<c_char> {
    nul_terminated(segment.to_vec())
}

fn parse_index(segment: &[u8]) -> Result<size_t, c_int> {
    match segment.len() {
        1 => {
            if is_plain_digit(segment[0]) {
                Ok((segment[0] - b'0') as size_t)
            } else {
                Err(EINVAL)
            }
        }
        0 => Err(EINVAL),
        _ => {
            if segment[0] == b'0' {
                return Err(EINVAL);
            }
            let mut value = 0usize;
            for &byte in segment {
                if !is_plain_digit(byte) {
                    return Err(EINVAL);
                }
                value = value
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as usize);
            }
            Ok(value)
        }
    }
}

unsafe fn json_pointer_get_single_path(
    obj: *mut json_object,
    segment: &[u8],
) -> Result<(*mut json_object, Option<Vec<c_char>>, size_t), c_int> {
    if object::json_object_is_type_impl(obj, JSON_TYPE_ARRAY) != 0 {
        let idx = parse_index(segment)?;
        if idx >= object::json_object_array_length_impl(obj) {
            return Err(ENOENT);
        }

        let child = object::json_object_array_get_idx_impl(obj, idx);
        if child.is_null() {
            return Err(ENOENT);
        }

        return Ok((child, None, idx));
    }

    let key = unescape_object_path(segment);
    let mut child = ptr::null_mut();
    if object::json_object_object_get_ex_impl(obj, key.as_ptr(), &mut child) == 0 {
        return Err(ENOENT);
    }

    Ok((child, Some(literal_object_path(segment)), 0))
}

unsafe fn json_pointer_result_get_recursive_bytes(
    obj: *mut json_object,
    path: &[u8],
) -> Result<JsonPointerGetResult, c_int> {
    if path.first() != Some(&b'/') {
        return Err(EINVAL);
    }

    let mut current = obj;
    let mut start = 1usize;

    loop {
        let next_slash = path[start..]
            .iter()
            .position(|&byte| byte == b'/')
            .map(|offset| start + offset)
            .unwrap_or(path.len());
        let segment = &path[start..next_slash];

        let parent = current;
        let (child, key, idx) = json_pointer_get_single_path(current, segment)?;
        if next_slash == path.len() {
            return Ok(JsonPointerGetResult {
                parent,
                obj: child,
                key_in_parent: key,
                index_in_parent: idx,
            });
        }

        current = child;
        start = next_slash + 1;
    }
}

unsafe fn json_pointer_object_get_recursive_bytes(
    obj: *mut json_object,
    path: &[u8],
) -> Result<*mut json_object, c_int> {
    Ok(json_pointer_result_get_recursive_bytes(obj, path)?.obj)
}

pub(crate) unsafe fn json_pointer_get_internal_impl(
    obj: *mut json_object,
    path: *const c_char,
    res: *mut JsonPointerGetResult,
) -> c_int {
    if obj.is_null() || path.is_null() {
        object::set_errno(EINVAL);
        return -1;
    }

    let bytes = CStr::from_ptr(path).to_bytes();
    if bytes.is_empty() {
        if !res.is_null() {
            *res = JsonPointerGetResult {
                parent: ptr::null_mut(),
                obj,
                key_in_parent: None,
                index_in_parent: usize::MAX,
            };
        }
        return 0;
    }

    match json_pointer_result_get_recursive_bytes(obj, bytes) {
        Ok(found) => {
            if !res.is_null() {
                *res = found;
            }
            0
        }
        Err(err) => {
            object::set_errno(err);
            -1
        }
    }
}

pub(crate) unsafe fn json_pointer_get_impl(
    obj: *mut json_object,
    path: *const c_char,
    res: *mut *mut json_object,
) -> c_int {
    let mut found = JsonPointerGetResult {
        parent: ptr::null_mut(),
        obj: ptr::null_mut(),
        key_in_parent: None,
        index_in_parent: usize::MAX,
    };
    let rc = json_pointer_get_internal_impl(obj, path, &mut found);
    if rc != 0 {
        return rc;
    }

    if !res.is_null() {
        *res = found.obj;
    }

    0
}

pub(crate) unsafe fn json_object_array_put_idx_cb_impl(
    parent: *mut json_object,
    idx: size_t,
    value: *mut json_object,
    _priv: *mut c_void,
) -> c_int {
    object::json_object_array_put_idx_impl(parent, idx, value)
}

unsafe fn json_pointer_set_single_path(
    parent: *mut json_object,
    segment: &[u8],
    value: *mut json_object,
    array_set_cb: JsonPointerArraySetCb,
    priv_: *mut c_void,
) -> c_int {
    if object::json_object_is_type_impl(parent, JSON_TYPE_ARRAY) != 0 {
        if segment == b"-" {
            return object::json_object_array_add_impl(parent, value);
        }

        let idx = match parse_index(segment) {
            Ok(idx) => idx,
            Err(err) => {
                object::set_errno(err);
                return -1;
            }
        };
        return array_set_cb(parent, idx, value, priv_);
    }

    if object::json_object_is_type_impl(parent, JSON_TYPE_OBJECT) != 0 {
        let key = literal_object_path(segment);
        return object::json_object_object_add_impl(parent, key.as_ptr(), value);
    }

    object::set_errno(ENOENT);
    -1
}

pub(crate) unsafe fn json_pointer_set_with_array_cb_impl(
    obj: *mut *mut json_object,
    path: *const c_char,
    value: *mut json_object,
    array_set_cb: JsonPointerArraySetCb,
    priv_: *mut c_void,
) -> c_int {
    if obj.is_null() || path.is_null() {
        object::set_errno(EINVAL);
        return -1;
    }

    let bytes = CStr::from_ptr(path).to_bytes();
    if bytes.is_empty() {
        object::json_object_put_impl(*obj);
        *obj = value;
        return 0;
    }

    if bytes[0] != b'/' {
        object::set_errno(EINVAL);
        return -1;
    }

    let endp = bytes.iter().rposition(|&byte| byte == b'/').unwrap_or(0);
    if endp == 0 {
        return json_pointer_set_single_path(*obj, &bytes[1..], value, array_set_cb, priv_);
    }

    let set = match json_pointer_object_get_recursive_bytes(*obj, &bytes[..endp]) {
        Ok(found) => found,
        Err(err) => {
            object::set_errno(err);
            return -1;
        }
    };

    json_pointer_set_single_path(set, &bytes[endp + 1..], value, array_set_cb, priv_)
}

pub(crate) unsafe fn json_pointer_set_impl(
    obj: *mut *mut json_object,
    path: *const c_char,
    value: *mut json_object,
) -> c_int {
    json_pointer_set_with_array_cb_impl(
        obj,
        path,
        value,
        json_object_array_put_idx_cb_impl,
        ptr::null_mut(),
    )
}
