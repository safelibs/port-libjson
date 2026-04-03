use crate::abi::*;
use crate::object;
use std::ptr;

pub(crate) unsafe fn json_object_iter_begin_impl(obj: *mut json_object) -> json_object_iterator {
    let table = object::object_table(obj);
    json_object_iterator {
        opaque_: if table.is_null() {
            ptr::null()
        } else {
            (*table).head.cast()
        },
    }
}

pub(crate) unsafe fn json_object_iter_end_impl(_obj: *const json_object) -> json_object_iterator {
    json_object_iterator {
        opaque_: ptr::null(),
    }
}

pub(crate) unsafe fn json_object_iter_init_default_impl() -> json_object_iterator {
    json_object_iterator {
        opaque_: ptr::null(),
    }
}

pub(crate) unsafe fn json_object_iter_next_impl(iter: *mut json_object_iterator) {
    if iter.is_null() || (*iter).opaque_.is_null() {
        return;
    }

    let entry = (*iter).opaque_.cast::<lh_entry>();
    (*iter).opaque_ = (*entry).next.cast();
}

pub(crate) unsafe fn json_object_iter_peek_name_impl(
    iter: *const json_object_iterator,
) -> *const c_char {
    if iter.is_null() || (*iter).opaque_.is_null() {
        return ptr::null();
    }

    let entry = (*iter).opaque_.cast::<lh_entry>();
    (*entry).k.cast()
}

pub(crate) unsafe fn json_object_iter_peek_value_impl(
    iter: *const json_object_iterator,
) -> *mut json_object {
    if iter.is_null() || (*iter).opaque_.is_null() {
        return ptr::null_mut();
    }

    let entry = (*iter).opaque_.cast::<lh_entry>();
    (*entry).v.cast_mut().cast()
}

pub(crate) unsafe fn json_object_iter_equal_impl(
    left: *const json_object_iterator,
    right: *const json_object_iterator,
) -> json_bool {
    if left.is_null() || right.is_null() {
        return 0;
    }

    ((*left).opaque_ == (*right).opaque_) as json_bool
}
