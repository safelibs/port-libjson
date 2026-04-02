use crate::abi::*;
use crate::{object, pointer};
use std::ffi::CStr;
use std::ptr;

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
}

const EFAULT: c_int = 14;
const EINVAL: c_int = 22;
const ENOENT: c_int = 2;
const ENOMEM: c_int = 12;
const JSON_TYPE_ARRAY: json_type = 5;

const FIELD_OP: &[u8] = b"op\0";
const FIELD_PATH: &[u8] = b"path\0";
const FIELD_VALUE: &[u8] = b"value\0";
const FIELD_FROM: &[u8] = b"from\0";

const MSG_BAD_BASE: &[u8] = b"Exactly one of *base or copy_from must be non-NULL\0";
const MSG_PATCH_NOT_ARRAY: &[u8] = b"Patch object is not of type json_type_array\0";
const MSG_COPY_FAILED: &[u8] = b"Unable to copy copy_from using json_object_deep_copy()\0";
const MSG_MISSING_OP: &[u8] = b"Patch object does not contain 'op' field\0";
const MSG_MISSING_PATH: &[u8] = b"Patch object does not contain 'path' field\0";
const MSG_MISSING_VALUE: &[u8] = b"Patch object does not contain a 'value' field\0";
const MSG_MISSING_FROM: &[u8] = b"Patch does not contain a 'from' field\0";
const MSG_INVALID_OP: &[u8] = b"Patch object has invalid 'op' field\0";
const MSG_TEST_MISMATCH: &[u8] =
    b"Value of element referenced by 'path' field did not match 'value' field\0";
const MSG_REMOVE_FAILED: &[u8] = b"Unable to remove path referenced by 'path' field\0";
const MSG_SET_FAILED: &[u8] = b"Failed to set value at path referenced by 'path' field\0";
const MSG_INVALID_MOVE: &[u8] = b"Invalid attempt to move parent under a child\0";
const MSG_NOT_FOUND_PATH: &[u8] = b"Did not find element referenced by path field\0";
const MSG_INVALID_PATH: &[u8] = b"Invalid path field\0";
const MSG_NOT_FOUND_FROM: &[u8] = b"Did not find element referenced by from field\0";
const MSG_INVALID_FROM: &[u8] = b"Invalid from field\0";

fn msg_ptr(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr().cast()
}

unsafe fn errno_value() -> c_int {
    *__errno_location()
}

unsafe fn set_patch_err(
    patch_error: *mut json_patch_error,
    errno_code: c_int,
    errmsg: *const c_char,
) {
    (*patch_error).errno_code = errno_code;
    (*patch_error).errmsg = errmsg;
    object::set_errno(0);
}

unsafe fn set_patch_err_from_ptrget(
    patch_error: *mut json_patch_error,
    errno_code: c_int,
    field_name: &[u8],
) {
    let message = if errno_code == ENOENT {
        if field_name == b"path" {
            msg_ptr(MSG_NOT_FOUND_PATH)
        } else {
            msg_ptr(MSG_NOT_FOUND_FROM)
        }
    } else if field_name == b"path" {
        msg_ptr(MSG_INVALID_PATH)
    } else {
        msg_ptr(MSG_INVALID_FROM)
    };
    set_patch_err(patch_error, errno_code, message);
}

unsafe fn json_patch_apply_test(
    res: *mut *mut json_object,
    patch_elem: *mut json_object,
    path: *const c_char,
    patch_error: *mut json_patch_error,
) -> c_int {
    let mut expected = ptr::null_mut();
    if object::json_object_object_get_ex_impl(patch_elem, FIELD_VALUE.as_ptr().cast(), &mut expected)
        == 0
    {
        set_patch_err(patch_error, EINVAL, msg_ptr(MSG_MISSING_VALUE));
        return -1;
    }

    let mut actual = ptr::null_mut();
    if pointer::json_pointer_get_impl(*res, path, &mut actual) != 0 {
        set_patch_err_from_ptrget(patch_error, errno_value(), b"path");
        return -1;
    }

    if object::json_object_equal_impl(expected, actual) == 0 {
        set_patch_err(patch_error, ENOENT, msg_ptr(MSG_TEST_MISMATCH));
        return -1;
    }

    0
}

unsafe fn json_patch_apply_remove_internal(jpres: &mut pointer::JsonPointerGetResult) -> c_int {
    if object::json_object_is_type_impl(jpres.parent, JSON_TYPE_ARRAY) != 0 {
        object::json_object_array_del_idx_impl(jpres.parent, jpres.index_in_parent, 1)
    } else if !jpres.parent.is_null() {
        if let Some(key) = &jpres.key_in_parent {
            object::json_object_object_del_impl(jpres.parent, key.as_ptr());
            0
        } else {
            -1
        }
    } else {
        object::json_object_put_impl(jpres.obj);
        jpres.obj = ptr::null_mut();
        0
    }
}

unsafe fn json_patch_apply_remove(
    res: *mut *mut json_object,
    path: *const c_char,
    patch_error: *mut json_patch_error,
) -> c_int {
    let mut found = pointer::JsonPointerGetResult {
        parent: ptr::null_mut(),
        obj: ptr::null_mut(),
        key_in_parent: None,
        index_in_parent: usize::MAX,
    };
    if pointer::json_pointer_get_internal_impl(*res, path, &mut found) != 0 {
        set_patch_err_from_ptrget(patch_error, errno_value(), b"path");
        return -1;
    }

    let rc = json_patch_apply_remove_internal(&mut found);
    if rc < 0 {
        set_patch_err(patch_error, EINVAL, msg_ptr(MSG_REMOVE_FAILED));
    }
    if found.parent.is_null() {
        *res = ptr::null_mut();
    }
    rc
}

unsafe fn json_object_array_insert_idx_cb(
    parent: *mut json_object,
    idx: size_t,
    value: *mut json_object,
    priv_: *mut c_void,
) -> c_int {
    let add = &mut *priv_.cast::<c_int>();
    if idx > object::json_object_array_length_impl(parent) {
        object::set_errno(EINVAL);
        return -1;
    }

    let rc = if *add != 0 {
        object::json_object_array_insert_idx_impl(parent, idx, value)
    } else {
        object::json_object_array_put_idx_impl(parent, idx, value)
    };
    if rc < 0 {
        object::set_errno(EINVAL);
    }
    rc
}

unsafe fn json_patch_apply_add_replace(
    res: *mut *mut json_object,
    patch_elem: *mut json_object,
    path: *const c_char,
    add: c_int,
    patch_error: *mut json_patch_error,
) -> c_int {
    let mut value = ptr::null_mut();
    if object::json_object_object_get_ex_impl(patch_elem, FIELD_VALUE.as_ptr().cast(), &mut value)
        == 0
    {
        set_patch_err(patch_error, EINVAL, msg_ptr(MSG_MISSING_VALUE));
        return -1;
    }

    if add == 0 && pointer::json_pointer_get_impl(*res, path, ptr::null_mut()) != 0 {
        set_patch_err_from_ptrget(patch_error, errno_value(), b"path");
        return -1;
    }

    let rc = pointer::json_pointer_set_with_array_cb_impl(
        res,
        path,
        object::json_object_get_impl(value),
        json_object_array_insert_idx_cb,
        (&add as *const c_int).cast_mut().cast(),
    );
    if rc != 0 {
        set_patch_err(patch_error, errno_value(), msg_ptr(MSG_SET_FAILED));
        object::json_object_put_impl(value);
    }

    rc
}

unsafe fn json_object_array_move_cb(
    parent: *mut json_object,
    idx: size_t,
    value: *mut json_object,
    priv_: *mut c_void,
) -> c_int {
    let from = &*priv_.cast::<pointer::JsonPointerGetResult>();
    let mut len = object::json_object_array_length_impl(parent);
    if parent == from.parent {
        len = len.saturating_add(1);
    }

    if idx > len {
        object::set_errno(EINVAL);
        return -1;
    }

    let rc = object::json_object_array_insert_idx_impl(parent, idx, value);
    if rc < 0 {
        object::set_errno(EINVAL);
    }
    rc
}

unsafe fn json_patch_apply_move_copy(
    res: *mut *mut json_object,
    patch_elem: *mut json_object,
    path: *const c_char,
    move_op: c_int,
    patch_error: *mut json_patch_error,
) -> c_int {
    let mut jfrom = ptr::null_mut();
    if object::json_object_object_get_ex_impl(patch_elem, FIELD_FROM.as_ptr().cast(), &mut jfrom)
        == 0
    {
        set_patch_err(patch_error, EINVAL, msg_ptr(MSG_MISSING_FROM));
        return -1;
    }

    let from_s = object::json_object_get_string_impl(jfrom);
    if from_s.is_null() || path.is_null() {
        set_patch_err_from_ptrget(patch_error, EINVAL, b"from");
        return -1;
    }

    let from_bytes = CStr::from_ptr(from_s).to_bytes();
    let path_bytes = CStr::from_ptr(path).to_bytes();
    if path_bytes.starts_with(from_bytes) {
        if from_bytes.len() == path_bytes.len() {
            return 0;
        }
        set_patch_err(patch_error, EINVAL, msg_ptr(MSG_INVALID_MOVE));
        return -1;
    }

    let mut from = pointer::JsonPointerGetResult {
        parent: ptr::null_mut(),
        obj: ptr::null_mut(),
        key_in_parent: None,
        index_in_parent: usize::MAX,
    };
    let rc = pointer::json_pointer_get_internal_impl(*res, from_s, &mut from);
    if rc != 0 {
        set_patch_err_from_ptrget(patch_error, errno_value(), b"from");
        return rc;
    }

    object::json_object_get_impl(from.obj);

    let array_cb = if move_op == 0 {
        json_object_array_insert_idx_cb
    } else {
        let remove_rc = json_patch_apply_remove_internal(&mut from);
        if remove_rc < 0 {
            object::json_object_put_impl(from.obj);
            return remove_rc;
        }
        json_object_array_move_cb
    };

    let rc = pointer::json_pointer_set_with_array_cb_impl(
        res,
        path,
        from.obj,
        array_cb,
        (&mut from as *mut pointer::JsonPointerGetResult).cast(),
    );
    if rc != 0 {
        set_patch_err(patch_error, errno_value(), msg_ptr(MSG_SET_FAILED));
        object::json_object_put_impl(from.obj);
    }

    rc
}

pub(crate) unsafe fn json_patch_apply_impl(
    copy_from: *mut json_object,
    patch: *mut json_object,
    base: *mut *mut json_object,
    patch_error: *mut json_patch_error,
) -> c_int {
    let mut placeholder = json_patch_error {
        errno_code: 0,
        patch_failure_idx: usize::MAX,
        errmsg: ptr::null(),
    };
    let patch_error = if patch_error.is_null() {
        &mut placeholder as *mut json_patch_error
    } else {
        patch_error
    };

    (*patch_error).patch_failure_idx = usize::MAX;
    (*patch_error).errno_code = 0;
    (*patch_error).errmsg = ptr::null();

    if base.is_null()
        || ((*base).is_null() && copy_from.is_null())
        || (!(*base).is_null() && !copy_from.is_null())
    {
        set_patch_err(patch_error, EFAULT, msg_ptr(MSG_BAD_BASE));
        return -1;
    }

    if object::json_object_is_type_impl(patch, JSON_TYPE_ARRAY) == 0 {
        set_patch_err(patch_error, EFAULT, msg_ptr(MSG_PATCH_NOT_ARRAY));
        return -1;
    }

    if !copy_from.is_null() && object::json_object_deep_copy_impl(copy_from, base, None) < 0 {
        set_patch_err(patch_error, ENOMEM, msg_ptr(MSG_COPY_FAILED));
        return -1;
    }

    for idx in 0..object::json_object_array_length_impl(patch) {
        let patch_elem = object::json_object_array_get_idx_impl(patch, idx);
        let mut jop = ptr::null_mut();
        let mut jpath = ptr::null_mut();

        (*patch_error).patch_failure_idx = idx;

        if object::json_object_object_get_ex_impl(patch_elem, FIELD_OP.as_ptr().cast(), &mut jop)
            == 0
        {
            set_patch_err(patch_error, EINVAL, msg_ptr(MSG_MISSING_OP));
            return -1;
        }
        if object::json_object_object_get_ex_impl(patch_elem, FIELD_PATH.as_ptr().cast(), &mut jpath)
            == 0
        {
            set_patch_err(patch_error, EINVAL, msg_ptr(MSG_MISSING_PATH));
            return -1;
        }

        let op = object::json_object_get_string_impl(jop);
        let path = object::json_object_get_string_impl(jpath);

        let rc = if !op.is_null() && CStr::from_ptr(op).to_bytes() == b"test" {
            json_patch_apply_test(base, patch_elem, path, patch_error)
        } else if !op.is_null() && CStr::from_ptr(op).to_bytes() == b"remove" {
            json_patch_apply_remove(base, path, patch_error)
        } else if !op.is_null() && CStr::from_ptr(op).to_bytes() == b"add" {
            json_patch_apply_add_replace(base, patch_elem, path, 1, patch_error)
        } else if !op.is_null() && CStr::from_ptr(op).to_bytes() == b"replace" {
            json_patch_apply_add_replace(base, patch_elem, path, 0, patch_error)
        } else if !op.is_null() && CStr::from_ptr(op).to_bytes() == b"move" {
            json_patch_apply_move_copy(base, patch_elem, path, 1, patch_error)
        } else if !op.is_null() && CStr::from_ptr(op).to_bytes() == b"copy" {
            json_patch_apply_move_copy(base, patch_elem, path, 0, patch_error)
        } else {
            set_patch_err(patch_error, EINVAL, msg_ptr(MSG_INVALID_OP));
            return -1;
        };

        if rc < 0 {
            return rc;
        }
    }

    0
}
