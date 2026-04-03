use crate::abi::*;
use crate::{arraylist, object};
use std::ptr;

pub(crate) const JSON_C_VISIT_SECOND: c_int = 0x02;
pub(crate) const JSON_C_VISIT_RETURN_CONTINUE: c_int = 0;
pub(crate) const JSON_C_VISIT_RETURN_SKIP: c_int = 7547;
pub(crate) const JSON_C_VISIT_RETURN_POP: c_int = 767;
pub(crate) const JSON_C_VISIT_RETURN_STOP: c_int = 7867;
pub(crate) const JSON_C_VISIT_RETURN_ERROR: c_int = -1;

pub(crate) unsafe fn json_c_visit_impl(
    obj: *mut json_object,
    _future_flags: c_int,
    userfunc: Option<json_c_visit_userfunc>,
    userarg: *mut c_void,
) -> c_int {
    let Some(userfunc) = userfunc else {
        return JSON_C_VISIT_RETURN_ERROR;
    };

    let rc = visit_recursive(
        obj,
        ptr::null_mut(),
        ptr::null(),
        ptr::null_mut(),
        userfunc,
        userarg,
    );
    match rc {
        JSON_C_VISIT_RETURN_CONTINUE
        | JSON_C_VISIT_RETURN_SKIP
        | JSON_C_VISIT_RETURN_POP
        | JSON_C_VISIT_RETURN_STOP => 0,
        _ => JSON_C_VISIT_RETURN_ERROR,
    }
}

unsafe fn visit_recursive(
    obj: *mut json_object,
    parent: *mut json_object,
    key: *const c_char,
    index: *mut size_t,
    userfunc: json_c_visit_userfunc,
    userarg: *mut c_void,
) -> c_int {
    let user_rc = userfunc(obj, 0, parent, key, index, userarg);
    match user_rc {
        JSON_C_VISIT_RETURN_CONTINUE => {}
        JSON_C_VISIT_RETURN_SKIP
        | JSON_C_VISIT_RETURN_POP
        | JSON_C_VISIT_RETURN_STOP
        | JSON_C_VISIT_RETURN_ERROR => return user_rc,
        _ => return JSON_C_VISIT_RETURN_ERROR,
    }

    match object::json_object_get_type_impl(obj) {
        0 | 1 | 2 | 3 | 6 => return JSON_C_VISIT_RETURN_CONTINUE,
        4 => {
            let table = object::object_table(obj);
            if table.is_null() {
                return JSON_C_VISIT_RETURN_ERROR;
            }

            let mut entry = (*table).head;
            while !entry.is_null() {
                let child_rc = visit_recursive(
                    (*entry).v.cast_mut().cast(),
                    obj,
                    (*entry).k.cast(),
                    ptr::null_mut(),
                    userfunc,
                    userarg,
                );
                if child_rc == JSON_C_VISIT_RETURN_POP {
                    break;
                }
                if child_rc == JSON_C_VISIT_RETURN_STOP || child_rc == JSON_C_VISIT_RETURN_ERROR {
                    return child_rc;
                }
                if child_rc != JSON_C_VISIT_RETURN_CONTINUE && child_rc != JSON_C_VISIT_RETURN_SKIP
                {
                    return JSON_C_VISIT_RETURN_ERROR;
                }
                entry = (*entry).next;
            }
        }
        5 => {
            let list = object::array_list_ptr(obj);
            if list.is_null() {
                return JSON_C_VISIT_RETURN_ERROR;
            }

            let len = arraylist::array_list_length_impl(list);
            let mut idx = 0usize;
            while idx < len {
                let mut current_index = idx;
                let child = arraylist::array_list_get_idx_impl(list, idx).cast::<json_object>();
                let child_rc = visit_recursive(
                    child,
                    obj,
                    ptr::null(),
                    &mut current_index,
                    userfunc,
                    userarg,
                );
                if child_rc == JSON_C_VISIT_RETURN_POP {
                    break;
                }
                if child_rc == JSON_C_VISIT_RETURN_STOP || child_rc == JSON_C_VISIT_RETURN_ERROR {
                    return child_rc;
                }
                if child_rc != JSON_C_VISIT_RETURN_CONTINUE && child_rc != JSON_C_VISIT_RETURN_SKIP
                {
                    return JSON_C_VISIT_RETURN_ERROR;
                }
                idx += 1;
            }
        }
        _ => return JSON_C_VISIT_RETURN_ERROR,
    }

    let second_rc = userfunc(obj, JSON_C_VISIT_SECOND, parent, key, index, userarg);
    match second_rc {
        JSON_C_VISIT_RETURN_SKIP | JSON_C_VISIT_RETURN_POP | JSON_C_VISIT_RETURN_CONTINUE => {
            JSON_C_VISIT_RETURN_CONTINUE
        }
        JSON_C_VISIT_RETURN_STOP | JSON_C_VISIT_RETURN_ERROR => second_rc,
        _ => JSON_C_VISIT_RETURN_ERROR,
    }
}
