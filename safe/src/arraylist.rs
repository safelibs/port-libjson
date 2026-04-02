use crate::abi::*;
use std::mem::size_of;
use std::ptr;

unsafe extern "C"
{
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

unsafe fn array_list_expand_internal(arr: *mut array_list, max: size_t) -> c_int
{
    if arr.is_null()
    {
        return -1;
    }

    if max < (*arr).size
    {
        return 0;
    }

    let mut new_size = if (*arr).size >= usize::MAX / 2
    {
        max
    }
    else
    {
        let doubled = (*arr).size << 1;
        if doubled < max { max } else { doubled }
    };
    if new_size > usize::MAX / size_of::<*mut c_void>()
    {
        return -1;
    }
    if new_size == 0
    {
        new_size = 1;
    }

    let resized =
        realloc((*arr).array.cast(), new_size * size_of::<*mut c_void>()).cast::<*mut c_void>();
    if resized.is_null()
    {
        return -1;
    }

    (*arr).array = resized;
    (*arr).size = new_size;
    0
}

pub(crate) unsafe fn array_list_new_impl(free_fn: Option<array_list_free_fn>) -> *mut array_list
{
    array_list_new2_impl(free_fn, ARRAY_LIST_DEFAULT_SIZE)
}

pub(crate) unsafe fn array_list_new2_impl(
    free_fn: Option<array_list_free_fn>,
    initial_size: c_int,
) -> *mut array_list
{
    if initial_size < 0 || initial_size as usize >= usize::MAX / size_of::<*mut c_void>()
    {
        return ptr::null_mut();
    }

    let arr = malloc(size_of::<array_list>()).cast::<array_list>();
    if arr.is_null()
    {
        return ptr::null_mut();
    }

    (*arr).size = initial_size as usize;
    (*arr).length = 0;
    (*arr).free_fn = free_fn;

    let alloc_size = (*arr).size.max(1) * size_of::<*mut c_void>();
    (*arr).array = malloc(alloc_size).cast();
    if (*arr).array.is_null()
    {
        free(arr.cast());
        return ptr::null_mut();
    }

    arr
}

pub(crate) unsafe fn array_list_free_impl(arr: *mut array_list)
{
    if arr.is_null()
    {
        return;
    }

    for idx in 0..(*arr).length
    {
        let value = *(*arr).array.add(idx);
        if !value.is_null()
        {
            if let Some(free_fn) = (*arr).free_fn
            {
                free_fn(value);
            }
        }
    }

    free((*arr).array.cast());
    free(arr.cast());
}

pub(crate) unsafe fn array_list_get_idx_impl(arr: *mut array_list, idx: size_t) -> *mut c_void
{
    if arr.is_null() || idx >= (*arr).length
    {
        return ptr::null_mut();
    }
    *(*arr).array.add(idx)
}

pub(crate) unsafe fn array_list_insert_idx_impl(
    arr: *mut array_list,
    idx: size_t,
    data: *mut c_void,
) -> c_int
{
    if arr.is_null()
    {
        return -1;
    }
    if idx >= (*arr).length
    {
        return array_list_put_idx_impl(arr, idx, data);
    }
    if (*arr).length == usize::MAX
    {
        return -1;
    }
    if array_list_expand_internal(arr, (*arr).length + 1) != 0
    {
        return -1;
    }

    let move_amount = ((*arr).length - idx) * size_of::<*mut c_void>();
    memmove(
        (*arr).array.add(idx + 1).cast(),
        (*arr).array.add(idx).cast(),
        move_amount,
    );
    *(*arr).array.add(idx) = data;
    (*arr).length += 1;
    0
}

pub(crate) unsafe fn array_list_put_idx_impl(
    arr: *mut array_list,
    idx: size_t,
    data: *mut c_void,
) -> c_int
{
    if arr.is_null()
    {
        return -1;
    }
    if idx == usize::MAX
    {
        return -1;
    }
    if array_list_expand_internal(arr, idx + 1) != 0
    {
        return -1;
    }

    if idx < (*arr).length
    {
        let existing = *(*arr).array.add(idx);
        if !existing.is_null()
        {
            if let Some(free_fn) = (*arr).free_fn
            {
                free_fn(existing);
            }
        }
    }

    *(*arr).array.add(idx) = data;
    if idx > (*arr).length
    {
        memset(
            (*arr).array.add((*arr).length).cast(),
            0,
            (idx - (*arr).length) * size_of::<*mut c_void>(),
        );
    }
    if (*arr).length <= idx
    {
        (*arr).length = idx + 1;
    }
    0
}

pub(crate) unsafe fn array_list_add_impl(arr: *mut array_list, data: *mut c_void) -> c_int
{
    if arr.is_null()
    {
        return -1;
    }

    let idx = (*arr).length;
    if idx == usize::MAX
    {
        return -1;
    }
    if array_list_expand_internal(arr, idx + 1) != 0
    {
        return -1;
    }

    *(*arr).array.add(idx) = data;
    (*arr).length += 1;
    0
}

pub(crate) unsafe fn array_list_length_impl(arr: *mut array_list) -> size_t
{
    if arr.is_null() { 0 } else { (*arr).length }
}

pub(crate) unsafe fn array_list_sort_impl(arr: *mut array_list, compar: Option<comparison_fn>)
{
    if arr.is_null()
    {
        return;
    }
    if let Some(compar) = compar
    {
        qsort((*arr).array.cast(), (*arr).length, size_of::<*mut c_void>(), compar);
    }
}

pub(crate) unsafe fn array_list_bsearch_impl(
    key: *mut *const c_void,
    arr: *mut array_list,
    compar: Option<comparison_fn>,
) -> *mut c_void
{
    if key.is_null() || arr.is_null()
    {
        return ptr::null_mut();
    }

    let Some(compar) = compar else {
        return ptr::null_mut();
    };

    bsearch(
        key.cast(),
        (*arr).array.cast(),
        (*arr).length,
        size_of::<*mut c_void>(),
        compar,
    )
}

pub(crate) unsafe fn array_list_del_idx_impl(
    arr: *mut array_list,
    idx: size_t,
    count: size_t,
) -> c_int
{
    if arr.is_null()
    {
        return -1;
    }

    let Some(stop) = idx.checked_add(count) else {
        return -1;
    };
    if idx >= (*arr).length || stop > (*arr).length
    {
        return -1;
    }

    for current in idx..stop
    {
        let value = *(*arr).array.add(current);
        if !value.is_null()
        {
            if let Some(free_fn) = (*arr).free_fn
            {
                free_fn(value);
            }
        }
    }

    memmove(
        (*arr).array.add(idx).cast(),
        (*arr).array.add(stop).cast(),
        ((*arr).length - stop) * size_of::<*mut c_void>(),
    );
    (*arr).length -= count;
    0
}

pub(crate) unsafe fn array_list_shrink_impl(arr: *mut array_list, empty_slots: size_t) -> c_int
{
    if arr.is_null()
    {
        return -1;
    }
    if empty_slots >= usize::MAX / size_of::<*mut c_void>() - (*arr).length
    {
        return -1;
    }

    let mut new_size = (*arr).length + empty_slots;
    if new_size == (*arr).size
    {
        return 0;
    }
    if new_size > (*arr).size
    {
        return array_list_expand_internal(arr, new_size);
    }
    if new_size == 0
    {
        new_size = 1;
    }

    let resized =
        realloc((*arr).array.cast(), new_size * size_of::<*mut c_void>()).cast::<*mut c_void>();
    if resized.is_null()
    {
        return -1;
    }

    (*arr).array = resized;
    (*arr).size = new_size;
    0
}
