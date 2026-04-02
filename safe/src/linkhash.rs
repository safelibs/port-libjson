use crate::abi::*;
use crate::random_seed;
use core::sync::atomic::{AtomicI32, Ordering};
use std::ffi::CStr;
use std::mem::size_of;
use std::ptr;

extern "C"
{
    fn calloc(nmemb: size_t, size: size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
}

const JSON_C_STR_HASH_DFLT: c_int = 0;
const JSON_C_STR_HASH_PERLLIKE: c_int = 1;
const JSON_C_OBJECT_ADD_CONSTANT_KEY: c_uint = 1 << 2;
const LH_PRIME: c_ulong = 0x9e37_0001;
const LH_LOAD_FACTOR_NUM: c_int = 66;
const LH_LOAD_FACTOR_DEN: c_int = 100;
const LH_EMPTY: *const c_void = usize::MAX as *const c_void;
const LH_FREED: *const c_void = (usize::MAX - 1) as *const c_void;

static STRING_HASH_MODE: AtomicI32 = AtomicI32::new(JSON_C_STR_HASH_DFLT);
static RANDOM_SEED: AtomicI32 = AtomicI32::new(-1);

fn rot(value: u32, amount: u32) -> u32
{
    value.rotate_left(amount)
}

fn mix(a: &mut u32, b: &mut u32, c: &mut u32)
{
    *a = a.wrapping_sub(*c);
    *a ^= rot(*c, 4);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= rot(*a, 6);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= rot(*b, 8);
    *b = b.wrapping_add(*a);
    *a = a.wrapping_sub(*c);
    *a ^= rot(*c, 16);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= rot(*a, 19);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= rot(*b, 4);
    *b = b.wrapping_add(*a);
}

fn final_mix(a: &mut u32, b: &mut u32, c: &mut u32)
{
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 14));
    *a ^= *c;
    *a = a.wrapping_sub(rot(*c, 11));
    *b ^= *a;
    *b = b.wrapping_sub(rot(*a, 25));
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 16));
    *a ^= *c;
    *a = a.wrapping_sub(rot(*c, 4));
    *b ^= *a;
    *b = b.wrapping_sub(rot(*a, 14));
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 24));
}

fn hashlittle(bytes: &[u8], initval: u32) -> u32
{
    let mut a = 0xdead_beefu32
        .wrapping_add(bytes.len() as u32)
        .wrapping_add(initval);
    let mut b = a;
    let mut c = a;
    let mut offset = 0;

    while bytes.len().saturating_sub(offset) > 12
    {
        a = a
            .wrapping_add(bytes[offset] as u32)
            .wrapping_add((bytes[offset + 1] as u32) << 8)
            .wrapping_add((bytes[offset + 2] as u32) << 16)
            .wrapping_add((bytes[offset + 3] as u32) << 24);
        b = b
            .wrapping_add(bytes[offset + 4] as u32)
            .wrapping_add((bytes[offset + 5] as u32) << 8)
            .wrapping_add((bytes[offset + 6] as u32) << 16)
            .wrapping_add((bytes[offset + 7] as u32) << 24);
        c = c
            .wrapping_add(bytes[offset + 8] as u32)
            .wrapping_add((bytes[offset + 9] as u32) << 8)
            .wrapping_add((bytes[offset + 10] as u32) << 16)
            .wrapping_add((bytes[offset + 11] as u32) << 24);
        mix(&mut a, &mut b, &mut c);
        offset += 12;
    }

    let tail = &bytes[offset..];
    match tail.len()
    {
        12 => c = c.wrapping_add((tail[11] as u32) << 24),
        _ => {}
    }
    match tail.len()
    {
        11 | 12 => c = c.wrapping_add((tail[10] as u32) << 16),
        _ => {}
    }
    match tail.len()
    {
        10 | 11 | 12 => c = c.wrapping_add((tail[9] as u32) << 8),
        _ => {}
    }
    match tail.len()
    {
        9..=12 => c = c.wrapping_add(tail[8] as u32),
        _ => {}
    }
    match tail.len()
    {
        8..=12 => b = b.wrapping_add((tail[7] as u32) << 24),
        _ => {}
    }
    match tail.len()
    {
        7..=12 => b = b.wrapping_add((tail[6] as u32) << 16),
        _ => {}
    }
    match tail.len()
    {
        6..=12 => b = b.wrapping_add((tail[5] as u32) << 8),
        _ => {}
    }
    match tail.len()
    {
        5..=12 => b = b.wrapping_add(tail[4] as u32),
        _ => {}
    }
    match tail.len()
    {
        4..=12 => a = a.wrapping_add((tail[3] as u32) << 24),
        _ => {}
    }
    match tail.len()
    {
        3..=12 => a = a.wrapping_add((tail[2] as u32) << 16),
        _ => {}
    }
    match tail.len()
    {
        2..=12 => a = a.wrapping_add((tail[1] as u32) << 8),
        _ => {}
    }
    match tail.len()
    {
        1..=12 => a = a.wrapping_add(tail[0] as u32),
        0 => return c,
        _ => unreachable!(),
    }

    final_mix(&mut a, &mut b, &mut c);
    c
}

unsafe fn current_seed() -> u32
{
    let mut seed = RANDOM_SEED.load(Ordering::Acquire);
    if seed == -1
    {
        let mut candidate = -1;
        while candidate == -1
        {
            candidate = random_seed::json_c_get_random_seed_impl();
        }
        let _ = RANDOM_SEED.compare_exchange(-1, candidate, Ordering::AcqRel, Ordering::Acquire);
        seed = RANDOM_SEED.load(Ordering::Acquire);
    }
    seed as u32
}

unsafe extern "C" fn lh_ptr_hash(key: *const c_void) -> c_ulong
{
    (((key as isize).wrapping_mul(LH_PRIME as isize) >> 4) as usize) as c_ulong
}

unsafe extern "C" fn lh_perllike_str_hash(key: *const c_void) -> c_ulong
{
    let bytes = CStr::from_ptr(key.cast()).to_bytes();
    let mut hashval = 1_u32;
    for byte in bytes
    {
        let promoted = (*byte as c_char as c_int) as u32;
        hashval = hashval.wrapping_mul(33).wrapping_add(promoted);
    }
    hashval as c_ulong
}

unsafe extern "C" fn lh_char_hash(key: *const c_void) -> c_ulong
{
    let bytes = CStr::from_ptr(key.cast()).to_bytes();
    hashlittle(bytes, current_seed()) as c_ulong
}

fn active_char_hash_fn() -> lh_hash_fn
{
    match STRING_HASH_MODE.load(Ordering::Acquire)
    {
        JSON_C_STR_HASH_PERLLIKE => lh_perllike_str_hash,
        _ => lh_char_hash,
    }
}

unsafe fn table_hash(t: *mut lh_table, key: *const c_void) -> c_ulong
{
    (*t).hash_fn.expect("hash function")(key)
}

pub(crate) unsafe fn json_global_set_string_hash_impl(hash_mode: c_int) -> c_int
{
    match hash_mode
    {
        JSON_C_STR_HASH_DFLT | JSON_C_STR_HASH_PERLLIKE =>
        {
            STRING_HASH_MODE.store(hash_mode, Ordering::Release);
            0
        }
        _ => -1,
    }
}

pub(crate) unsafe extern "C" fn lh_ptr_equal_impl(k1: *const c_void, k2: *const c_void) -> c_int
{
    (k1 == k2) as c_int
}

pub(crate) unsafe extern "C" fn lh_char_equal_impl(k1: *const c_void, k2: *const c_void) -> c_int
{
    (strcmp(k1.cast(), k2.cast()) == 0) as c_int
}

pub(crate) unsafe fn lh_table_new_impl(
    size: c_int,
    free_fn: Option<lh_entry_free_fn>,
    hash_fn: Option<lh_hash_fn>,
    equal_fn: Option<lh_equal_fn>,
) -> *mut lh_table
{
    if size <= 0 || hash_fn.is_none() || equal_fn.is_none()
    {
        return ptr::null_mut();
    }

    let table = calloc(1, size_of::<lh_table>()).cast::<lh_table>();
    if table.is_null()
    {
        return ptr::null_mut();
    }

    let entries = calloc(size as usize, size_of::<lh_entry>()).cast::<lh_entry>();
    if entries.is_null()
    {
        free(table.cast());
        return ptr::null_mut();
    }

    (*table).count = 0;
    (*table).size = size;
    (*table).table = entries;
    (*table).free_fn = free_fn;
    (*table).hash_fn = hash_fn;
    (*table).equal_fn = equal_fn;

    for idx in 0..size as usize
    {
        (*entries.add(idx)).k = LH_EMPTY;
    }
    table
}

pub(crate) unsafe fn lh_kchar_table_new_impl(
    size: c_int,
    free_fn: Option<lh_entry_free_fn>,
) -> *mut lh_table
{
    lh_table_new_impl(size, free_fn, Some(active_char_hash_fn()), Some(lh_char_equal_impl))
}

pub(crate) unsafe fn lh_kptr_table_new_impl(
    size: c_int,
    free_fn: Option<lh_entry_free_fn>,
) -> *mut lh_table
{
    lh_table_new_impl(size, free_fn, Some(lh_ptr_hash), Some(lh_ptr_equal_impl))
}

pub(crate) unsafe fn lh_table_resize_impl(t: *mut lh_table, new_size: c_int) -> c_int
{
    if t.is_null() || new_size <= 0
    {
        return -1;
    }

    let new_t = lh_table_new_impl(new_size, None, (*t).hash_fn, (*t).equal_fn);
    if new_t.is_null()
    {
        return -1;
    }

    let mut entry = (*t).head;
    while !entry.is_null()
    {
        let opts = if (*entry).k_is_constant != 0 {
            JSON_C_OBJECT_ADD_CONSTANT_KEY
        } else {
            0
        };
        if lh_table_insert_w_hash_impl(new_t, (*entry).k, (*entry).v, table_hash(new_t, (*entry).k), opts) != 0
        {
            lh_table_free_impl(new_t);
            return -1;
        }
        entry = (*entry).next;
    }

    free((*t).table.cast());
    (*t).table = (*new_t).table;
    (*t).size = (*new_t).size;
    (*t).count = (*new_t).count;
    (*t).head = (*new_t).head;
    (*t).tail = (*new_t).tail;
    free(new_t.cast());
    0
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::ffi::CString;

    #[test]
    fn perllike_hash_uses_c_char_promotion_for_high_bytes()
    {
        let key = CString::new(vec![0xff]).expect("CString");
        let expected = 1_u32
            .wrapping_mul(33)
            .wrapping_add(((0xff_u8 as c_char) as c_int) as u32);
        let actual = unsafe { lh_perllike_str_hash(key.as_ptr().cast()) as u32 };
        assert_eq!(actual, expected);
    }
}

pub(crate) unsafe fn lh_table_free_impl(t: *mut lh_table)
{
    if t.is_null()
    {
        return;
    }

    if let Some(free_fn) = (*t).free_fn
    {
        let mut current = (*t).head;
        while !current.is_null()
        {
            free_fn(current);
            current = (*current).next;
        }
    }

    free((*t).table.cast());
    free(t.cast());
}

pub(crate) unsafe fn lh_table_insert_w_hash_impl(
    t: *mut lh_table,
    key: *const c_void,
    value: *const c_void,
    hash: c_ulong,
    opts: c_uint,
) -> c_int
{
    if t.is_null()
    {
        return -1;
    }

    if (*t).count.saturating_mul(LH_LOAD_FACTOR_DEN) >= (*t).size.saturating_mul(LH_LOAD_FACTOR_NUM)
    {
        let new_size = if (*t).size > c_int::MAX / 2 { c_int::MAX } else { (*t).size * 2 };
        if (*t).size == c_int::MAX || lh_table_resize_impl(t, new_size) != 0
        {
            return -1;
        }
    }

    let mut slot_idx = (hash % (*t).size as c_ulong) as usize;
    loop
    {
        let slot = (*t).table.add(slot_idx);
        if (*slot).k == LH_EMPTY || (*slot).k == LH_FREED
        {
            (*slot).k = key;
            (*slot).k_is_constant = (opts & JSON_C_OBJECT_ADD_CONSTANT_KEY) as c_int;
            (*slot).v = value;
            (*t).count += 1;

            if (*t).head.is_null()
            {
                (*t).head = slot;
                (*t).tail = slot;
                (*slot).next = ptr::null_mut();
                (*slot).prev = ptr::null_mut();
            }
            else
            {
                (*(*t).tail).next = slot;
                (*slot).prev = (*t).tail;
                (*slot).next = ptr::null_mut();
                (*t).tail = slot;
            }
            return 0;
        }

        slot_idx += 1;
        if slot_idx == (*t).size as usize
        {
            slot_idx = 0;
        }
    }
}

pub(crate) unsafe fn lh_table_insert_impl(
    t: *mut lh_table,
    key: *const c_void,
    value: *const c_void,
) -> c_int
{
    if t.is_null()
    {
        return -1;
    }
    lh_table_insert_w_hash_impl(t, key, value, table_hash(t, key), 0)
}

pub(crate) unsafe fn lh_table_lookup_entry_w_hash_impl(
    t: *mut lh_table,
    key: *const c_void,
    hash: c_ulong,
) -> *mut lh_entry
{
    if t.is_null() || (*t).size <= 0
    {
        return ptr::null_mut();
    }

    let mut slot_idx = (hash % (*t).size as c_ulong) as usize;
    let mut scanned = 0;
    while scanned < (*t).size
    {
        let slot = (*t).table.add(slot_idx);
        if (*slot).k == LH_EMPTY
        {
            return ptr::null_mut();
        }
        if (*slot).k != LH_FREED && (*t).equal_fn.expect("equal function")((*slot).k, key) != 0
        {
            return slot;
        }

        slot_idx += 1;
        if slot_idx == (*t).size as usize
        {
            slot_idx = 0;
        }
        scanned += 1;
    }

    ptr::null_mut()
}

pub(crate) unsafe fn lh_table_lookup_entry_impl(t: *mut lh_table, key: *const c_void) -> *mut lh_entry
{
    if t.is_null()
    {
        return ptr::null_mut();
    }
    lh_table_lookup_entry_w_hash_impl(t, key, table_hash(t, key))
}

pub(crate) unsafe fn lh_table_lookup_ex_impl(
    t: *mut lh_table,
    key: *const c_void,
    value_out: *mut *mut c_void,
) -> json_bool
{
    let entry = lh_table_lookup_entry_impl(t, key);
    if !entry.is_null()
    {
        if !value_out.is_null()
        {
            *value_out = (*entry).v as *mut c_void;
        }
        return 1;
    }

    if !value_out.is_null()
    {
        *value_out = ptr::null_mut();
    }
    0
}

pub(crate) unsafe fn lh_table_delete_entry_impl(t: *mut lh_table, entry: *mut lh_entry) -> c_int
{
    if t.is_null() || entry.is_null()
    {
        return -1;
    }

    let slot_idx = entry.offset_from((*t).table);
    if slot_idx < 0 || slot_idx as usize >= (*t).size as usize
    {
        return -2;
    }
    let slot = (*t).table.add(slot_idx as usize);
    if (*slot).k == LH_EMPTY || (*slot).k == LH_FREED
    {
        return -1;
    }

    (*t).count -= 1;
    if let Some(free_fn) = (*t).free_fn
    {
        free_fn(entry);
    }
    (*slot).v = ptr::null();
    (*slot).k = LH_FREED;

    if (*t).tail == slot && (*t).head == slot
    {
        (*t).head = ptr::null_mut();
        (*t).tail = ptr::null_mut();
    }
    else if (*t).head == slot
    {
        (*(*t).head).next.as_mut().unwrap().prev = ptr::null_mut();
        (*t).head = (*(*t).head).next;
    }
    else if (*t).tail == slot
    {
        (*(*t).tail).prev.as_mut().unwrap().next = ptr::null_mut();
        (*t).tail = (*(*t).tail).prev;
    }
    else
    {
        (*(*slot).prev).next = (*slot).next;
        (*(*slot).next).prev = (*slot).prev;
    }

    (*slot).next = ptr::null_mut();
    (*slot).prev = ptr::null_mut();
    0
}

pub(crate) unsafe fn lh_table_delete_impl(t: *mut lh_table, key: *const c_void) -> c_int
{
    let entry = lh_table_lookup_entry_impl(t, key);
    if entry.is_null()
    {
        return -1;
    }
    lh_table_delete_entry_impl(t, entry)
}

pub(crate) unsafe fn lh_table_length_impl(t: *mut lh_table) -> c_int
{
    if t.is_null() { 0 } else { (*t).count }
}
