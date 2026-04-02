use crate::abi::*;
use std::mem::size_of;
use std::ptr;

extern "C" {
    fn bsearch(
        key: *const c_void,
        base: *const c_void,
        nmemb: size_t,
        size: size_t,
        compar: comparison_fn,
    ) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn malloc(size: size_t) -> *mut c_void;
    fn memmove(dest: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    fn qsort(base: *mut c_void, nmemb: size_t, size: size_t, compar: comparison_fn);
    fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
}

const ARRAY_LIST_DEFAULT_SIZE: c_int = 32;

fn array_list_mut<'a>(arr: *mut array_list) -> Option<&'a mut array_list> {
    unsafe { arr.as_mut() }
}

fn array_slot(arr: &array_list, idx: size_t) -> *mut *mut c_void {
    unsafe { arr.array.add(idx) }
}

fn array_slot_value(arr: &array_list, idx: size_t) -> *mut c_void {
    unsafe { *array_slot(arr, idx) }
}

fn set_array_slot(arr: &array_list, idx: size_t, value: *mut c_void) {
    unsafe {
        *array_slot(arr, idx) = value;
    }
}

fn call_free_fn(free_fn: array_list_free_fn, value: *mut c_void) {
    unsafe {
        free_fn(value);
    }
}

fn array_list_expand_internal(arr: *mut array_list, max: size_t) -> c_int {
    let Some(arr) = array_list_mut(arr) else {
        return -1;
    };

    if max < arr.size {
        return 0;
    }

    let mut new_size = if arr.size >= usize::MAX / 2 {
        max
    } else {
        let doubled = arr.size << 1;
        if doubled < max {
            max
        } else {
            doubled
        }
    };
    if new_size > usize::MAX / size_of::<*mut c_void>() {
        return -1;
    }
    if new_size == 0 {
        new_size = 1;
    }

    let resized = unsafe {
        realloc(arr.array.cast(), new_size * size_of::<*mut c_void>()).cast::<*mut c_void>()
    };
    if resized.is_null() {
        return -1;
    }

    arr.array = resized;
    arr.size = new_size;
    0
}

pub(crate) fn array_list_new_impl(free_fn: Option<array_list_free_fn>) -> *mut array_list {
    array_list_new2_impl(free_fn, ARRAY_LIST_DEFAULT_SIZE)
}

pub(crate) fn array_list_new2_impl(
    free_fn: Option<array_list_free_fn>,
    initial_size: c_int,
) -> *mut array_list {
    if initial_size < 0 || initial_size as usize >= usize::MAX / size_of::<*mut c_void>() {
        return ptr::null_mut();
    }

    let arr = unsafe { malloc(size_of::<array_list>()).cast::<array_list>() };
    if arr.is_null() {
        return ptr::null_mut();
    }

    let inner = array_list_mut(arr).expect("fresh malloc array_list pointer");
    inner.size = initial_size as usize;
    inner.length = 0;
    inner.free_fn = free_fn;

    let alloc_size = inner.size.max(1) * size_of::<*mut c_void>();
    inner.array = unsafe { malloc(alloc_size).cast() };
    if inner.array.is_null() {
        unsafe {
            free(arr.cast());
        }
        return ptr::null_mut();
    }

    arr
}

pub(crate) fn array_list_free_impl(arr: *mut array_list) {
    let Some(arr) = array_list_mut(arr) else {
        return;
    };

    for idx in 0..arr.length {
        let value = array_slot_value(arr, idx);
        if !value.is_null() {
            if let Some(free_fn) = arr.free_fn {
                call_free_fn(free_fn, value);
            }
        }
    }

    unsafe {
        free(arr.array.cast());
        free((arr as *mut array_list).cast());
    }
}

pub(crate) fn array_list_get_idx_impl(arr: *mut array_list, idx: size_t) -> *mut c_void {
    let Some(arr) = array_list_mut(arr) else {
        return ptr::null_mut();
    };
    if idx >= arr.length {
        return ptr::null_mut();
    }
    array_slot_value(arr, idx)
}

pub(crate) fn array_list_insert_idx_impl(
    arr: *mut array_list,
    idx: size_t,
    data: *mut c_void,
) -> c_int {
    let Some(arr_inner) = array_list_mut(arr) else {
        return -1;
    };
    if idx >= arr_inner.length {
        return array_list_put_idx_impl(arr, idx, data);
    }
    if arr_inner.length == usize::MAX {
        return -1;
    }
    if array_list_expand_internal(arr, arr_inner.length + 1) != 0 {
        return -1;
    }

    unsafe {
        memmove(
            arr_inner.array.add(idx + 1).cast(),
            arr_inner.array.add(idx).cast(),
            (arr_inner.length - idx) * size_of::<*mut c_void>(),
        );
    }
    set_array_slot(arr_inner, idx, data);
    arr_inner.length += 1;
    0
}

pub(crate) fn array_list_put_idx_impl(
    arr: *mut array_list,
    idx: size_t,
    data: *mut c_void,
) -> c_int {
    let Some(arr_inner) = array_list_mut(arr) else {
        return -1;
    };
    if idx == usize::MAX {
        return -1;
    }
    if array_list_expand_internal(arr, idx + 1) != 0 {
        return -1;
    }

    if idx < arr_inner.length {
        let existing = array_slot_value(arr_inner, idx);
        if !existing.is_null() {
            if let Some(free_fn) = arr_inner.free_fn {
                call_free_fn(free_fn, existing);
            }
        }
    }

    set_array_slot(arr_inner, idx, data);
    if idx > arr_inner.length {
        unsafe {
            memset(
                arr_inner.array.add(arr_inner.length).cast(),
                0,
                (idx - arr_inner.length) * size_of::<*mut c_void>(),
            );
        }
    }
    if arr_inner.length <= idx {
        arr_inner.length = idx + 1;
    }
    0
}

pub(crate) fn array_list_add_impl(arr: *mut array_list, data: *mut c_void) -> c_int {
    let Some(arr_inner) = array_list_mut(arr) else {
        return -1;
    };

    let idx = arr_inner.length;
    if idx == usize::MAX {
        return -1;
    }
    if array_list_expand_internal(arr, idx + 1) != 0 {
        return -1;
    }

    set_array_slot(arr_inner, idx, data);
    arr_inner.length += 1;
    0
}

pub(crate) fn array_list_length_impl(arr: *mut array_list) -> size_t {
    array_list_mut(arr).map(|arr| arr.length).unwrap_or(0)
}

pub(crate) fn array_list_sort_impl(arr: *mut array_list, compar: Option<comparison_fn>) {
    let Some(arr) = array_list_mut(arr) else {
        return;
    };
    if let Some(compar) = compar {
        unsafe {
            qsort(
                arr.array.cast(),
                arr.length,
                size_of::<*mut c_void>(),
                compar,
            );
        }
    }
}

pub(crate) fn array_list_bsearch_impl(
    key: *mut *const c_void,
    arr: *mut array_list,
    compar: Option<comparison_fn>,
) -> *mut c_void {
    let Some(arr) = array_list_mut(arr) else {
        return ptr::null_mut();
    };
    if key.is_null() {
        return ptr::null_mut();
    }

    let Some(compar) = compar else {
        return ptr::null_mut();
    };

    unsafe {
        bsearch(
            key.cast(),
            arr.array.cast(),
            arr.length,
            size_of::<*mut c_void>(),
            compar,
        )
    }
}

pub(crate) fn array_list_del_idx_impl(arr: *mut array_list, idx: size_t, count: size_t) -> c_int {
    let Some(arr) = array_list_mut(arr) else {
        return -1;
    };

    let Some(stop) = idx.checked_add(count) else {
        return -1;
    };
    if idx >= arr.length || stop > arr.length {
        return -1;
    }

    for current in idx..stop {
        let value = array_slot_value(arr, current);
        if !value.is_null() {
            if let Some(free_fn) = arr.free_fn {
                call_free_fn(free_fn, value);
            }
        }
    }

    unsafe {
        memmove(
            arr.array.add(idx).cast(),
            arr.array.add(stop).cast(),
            (arr.length - stop) * size_of::<*mut c_void>(),
        );
    }
    arr.length -= count;
    0
}

pub(crate) fn array_list_shrink_impl(arr: *mut array_list, empty_slots: size_t) -> c_int {
    let Some(arr) = array_list_mut(arr) else {
        return -1;
    };
    if empty_slots >= usize::MAX / size_of::<*mut c_void>() - arr.length {
        return -1;
    }

    let mut new_size = arr.length + empty_slots;
    if new_size == arr.size {
        return 0;
    }
    if new_size > arr.size {
        return array_list_expand_internal(arr, new_size);
    }
    if new_size == 0 {
        new_size = 1;
    }

    let resized = unsafe {
        realloc(arr.array.cast(), new_size * size_of::<*mut c_void>()).cast::<*mut c_void>()
    };
    if resized.is_null() {
        return -1;
    }

    arr.array = resized;
    arr.size = new_size;
    0
}
