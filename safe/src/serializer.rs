use crate::abi::*;
use crate::object;
use crate::{arraylist, errors, printbuf as printbuf_impl};
use std::cell::RefCell;
use std::ffi::CStr;
use std::ptr;
use std::sync::Mutex;

unsafe extern "C"
{
    fn free(ptr: *mut c_void);
    fn snprintf(dst: *mut c_char, size: size_t, format: *const c_char, ...) -> c_int;
}

const JSON_NULL_BYTES: &[u8; 5] = b"null\0";
const TRUE_BYTES: &[u8; 5] = b"true\0";
const FALSE_BYTES: &[u8; 6] = b"false\0";
const DEFAULT_DOUBLE_FORMAT: &[u8; 6] = b"%.17g\0";

static GLOBAL_DOUBLE_FORMAT: Mutex<Option<Vec<u8>>> = Mutex::new(None);

thread_local! {
    static THREAD_DOUBLE_FORMAT: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

fn append_bytes(pb: *mut printbuf, bytes: &[u8]) -> c_int
{
    unsafe { printbuf_impl::printbuf_memappend_impl(pb, bytes.as_ptr().cast(), bytes.len() as c_int) }
}

fn append_literal(pb: *mut printbuf, bytes_with_nul: &[u8]) -> c_int
{
    append_bytes(pb, &bytes_with_nul[..bytes_with_nul.len() - 1])
}

fn indent(pb: *mut printbuf, level: c_int, flags: c_int)
{
    if (flags & object::JSON_C_TO_STRING_PRETTY) == 0
    {
        return;
    }

    let fill = if (flags & object::JSON_C_TO_STRING_PRETTY_TAB) != 0
    {
        b'\t'
    }
    else
    {
        b' '
    };
    let count = if fill == b'\t' { level } else { level * 2 };
    unsafe {
        let _ = printbuf_impl::printbuf_memset_impl(pb, -1, fill as c_int, count);
    }
}

fn escape_string(pb: *mut printbuf, bytes: &[u8], flags: c_int) -> c_int
{
    let mut start = 0usize;
    for (idx, byte) in bytes.iter().copied().enumerate()
    {
        let escaped = match byte
        {
            b'\x08' => Some(*b"\\b"),
            b'\n' => Some(*b"\\n"),
            b'\r' => Some(*b"\\r"),
            b'\t' => Some(*b"\\t"),
            b'\x0c' => Some(*b"\\f"),
            b'"' => Some(*b"\\\""),
            b'\\' => Some(*b"\\\\"),
            b'/' if (flags & object::JSON_C_TO_STRING_NOSLASHESCAPE) == 0 => Some(*b"\\/"),
            0x00..=0x1f =>
            {
                let hex = b"0123456789abcdef";
                let escaped = [
                    b'\\',
                    b'u',
                    b'0',
                    b'0',
                    hex[(byte >> 4) as usize],
                    hex[(byte & 0x0f) as usize],
                ];
                if idx > start
                {
                    if append_bytes(pb, &bytes[start..idx]) < 0
                    {
                        return -1;
                    }
                }
                if append_bytes(pb, &escaped) < 0
                {
                    return -1;
                }
                start = idx + 1;
                continue;
            }
            _ => None,
        };

        if let Some(escaped) = escaped
        {
            if idx > start
            {
                if append_bytes(pb, &bytes[start..idx]) < 0
                {
                    return -1;
                }
            }
            if append_bytes(pb, &escaped) < 0
            {
                return -1;
            }
            start = idx + 1;
        }
    }

    if start < bytes.len()
    {
        append_bytes(pb, &bytes[start..])
    }
    else
    {
        0
    }
}

unsafe fn serialize_child(
    child: *mut json_object,
    pb: *mut printbuf,
    level: c_int,
    flags: c_int,
) -> c_int
{
    if child.is_null()
    {
        return append_literal(pb, JSON_NULL_BYTES);
    }

    let inner = object::as_json_box(child).expect("child object must be valid");
    let serializer = inner.to_json_string.expect("json_object serializer must be set");
    serializer(child, pb, level, flags)
}

pub(crate) unsafe fn json_object_get_userdata_impl(obj: *mut json_object) -> *mut c_void
{
    object::as_json_box(obj).map(|inner| inner.userdata).unwrap_or(ptr::null_mut())
}

pub(crate) unsafe fn json_object_set_userdata_impl(
    obj: *mut json_object,
    userdata: *mut c_void,
    user_delete: Option<json_object_delete_fn>,
)
{
    let Some(inner) = object::as_json_box_mut(obj) else {
        return;
    };

    if let Some(delete_fn) = inner.user_delete
    {
        delete_fn(obj, inner.userdata);
    }
    inner.userdata = userdata;
    inner.user_delete = user_delete;
}

pub(crate) unsafe fn json_object_set_serializer_impl(
    obj: *mut json_object,
    to_string_func: Option<json_object_to_json_string_fn>,
    userdata: *mut c_void,
    user_delete: Option<json_object_delete_fn>,
)
{
    let Some(inner) = object::as_json_box_mut(obj) else {
        return;
    };

    json_object_set_userdata_impl(obj, userdata, user_delete);
    inner.to_json_string = match to_string_func
    {
        Some(func) => Some(func),
        None => object::default_serializer_for_type(inner.o_type),
    };
}

pub(crate) unsafe fn json_object_to_json_string_length_impl(
    obj: *mut json_object,
    flags: c_int,
    length_out: *mut size_t,
) -> *const c_char
{
    if obj.is_null()
    {
        if !length_out.is_null()
        {
            *length_out = 4;
        }
        return JSON_NULL_BYTES.as_ptr().cast();
    }

    let inner = object::as_json_box_mut(obj).expect("valid object");
    if inner.pb.is_null()
    {
        inner.pb = printbuf_impl::printbuf_new_impl();
        if inner.pb.is_null()
        {
            if !length_out.is_null()
            {
                *length_out = 0;
            }
            return ptr::null();
        }
    }

    printbuf_impl::printbuf_reset_impl(inner.pb);
    let serializer = inner.to_json_string.expect("json_object serializer must exist");
    if serializer(obj, inner.pb, 0, flags) < 0
    {
        if !length_out.is_null()
        {
            *length_out = 0;
        }
        return ptr::null();
    }

    if !length_out.is_null()
    {
        *length_out = (*inner.pb).bpos as size_t;
    }
    (*inner.pb).buf.cast()
}

pub(crate) unsafe fn json_object_to_json_string_ext_impl(
    obj: *mut json_object,
    flags: c_int,
) -> *const c_char
{
    json_object_to_json_string_length_impl(obj, flags, ptr::null_mut())
}

pub(crate) unsafe fn json_object_to_json_string_impl(obj: *mut json_object) -> *const c_char
{
    json_object_to_json_string_length_impl(obj, object::JSON_C_TO_STRING_SPACED, ptr::null_mut())
}

pub(crate) unsafe extern "C" fn json_object_object_to_json_string_impl(
    obj: *mut json_object,
    pb: *mut printbuf,
    level: c_int,
    flags: c_int,
) -> c_int
{
    let table = object::object_table(obj);
    if table.is_null()
    {
        return -1;
    }

    append_literal(pb, b"{\0");
    let mut had_children = false;
    let mut entry = (*table).head;
    while !entry.is_null()
    {
        if had_children
        {
            append_literal(pb, b",\0");
        }
        if (flags & object::JSON_C_TO_STRING_PRETTY) != 0
        {
            append_literal(pb, b"\n\0");
        }
        had_children = true;
        if (flags & object::JSON_C_TO_STRING_SPACED) != 0 &&
            (flags & object::JSON_C_TO_STRING_PRETTY) == 0
        {
            append_literal(pb, b" \0");
        }

        indent(pb, level + 1, flags);
        append_literal(pb, b"\"\0");
        let key = CStr::from_ptr((*entry).k.cast()).to_bytes();
        if escape_string(pb, key, flags) < 0
        {
            return -1;
        }
        append_literal(pb, b"\"\0");
        if (flags & object::JSON_C_TO_STRING_SPACED) != 0
        {
            append_literal(pb, b": \0");
        }
        else
        {
            append_literal(pb, b":\0");
        }
        if serialize_child((*entry).v.cast_mut().cast(), pb, level + 1, flags) < 0
        {
            return -1;
        }
        entry = (*entry).next;
    }

    if (flags & object::JSON_C_TO_STRING_PRETTY) != 0 && had_children
    {
        append_literal(pb, b"\n\0");
        indent(pb, level, flags);
    }
    if (flags & object::JSON_C_TO_STRING_SPACED) != 0 &&
        (flags & object::JSON_C_TO_STRING_PRETTY) == 0
    {
        append_literal(pb, b" }\0")
    }
    else
    {
        append_literal(pb, b"}\0")
    }
}

pub(crate) unsafe extern "C" fn json_object_array_to_json_string_impl(
    obj: *mut json_object,
    pb: *mut printbuf,
    level: c_int,
    flags: c_int,
) -> c_int
{
    let list = object::array_list_ptr(obj);
    if list.is_null()
    {
        return -1;
    }

    append_literal(pb, b"[\0");
    let len = arraylist::array_list_length_impl(list);
    let mut had_children = false;
    for idx in 0..len
    {
        if had_children
        {
            append_literal(pb, b",\0");
        }
        if (flags & object::JSON_C_TO_STRING_PRETTY) != 0
        {
            append_literal(pb, b"\n\0");
        }
        had_children = true;
        if (flags & object::JSON_C_TO_STRING_SPACED) != 0 &&
            (flags & object::JSON_C_TO_STRING_PRETTY) == 0
        {
            append_literal(pb, b" \0");
        }

        indent(pb, level + 1, flags);
        let child = arraylist::array_list_get_idx_impl(list, idx).cast::<json_object>();
        if serialize_child(child, pb, level + 1, flags) < 0
        {
            return -1;
        }
    }

    if (flags & object::JSON_C_TO_STRING_PRETTY) != 0 && had_children
    {
        append_literal(pb, b"\n\0");
        indent(pb, level, flags);
    }
    if (flags & object::JSON_C_TO_STRING_SPACED) != 0 &&
        (flags & object::JSON_C_TO_STRING_PRETTY) == 0
    {
        append_literal(pb, b" ]\0")
    }
    else
    {
        append_literal(pb, b"]\0")
    }
}

pub(crate) unsafe extern "C" fn json_object_boolean_to_json_string_impl(
    obj: *mut json_object,
    pb: *mut printbuf,
    _level: c_int,
    _flags: c_int,
) -> c_int
{
    let inner = object::as_json_box(obj).expect("boolean object must be valid");
    match inner.data
    {
        object::JsonData::Boolean(value) =>
        {
            if value != 0 { append_literal(pb, TRUE_BYTES) } else { append_literal(pb, FALSE_BYTES) }
        }
        _ => -1,
    }
}

pub(crate) unsafe extern "C" fn json_object_int_to_json_string_impl(
    obj: *mut json_object,
    pb: *mut printbuf,
    _level: c_int,
    _flags: c_int,
) -> c_int
{
    let inner = object::as_json_box(obj).expect("int object must be valid");
    match inner.data
    {
        object::JsonData::Int(object::JsonInt::Int64(value)) =>
        {
            let rendered = value.to_string();
            append_bytes(pb, rendered.as_bytes())
        }
        object::JsonData::Int(object::JsonInt::UInt64(value)) =>
        {
            let rendered = value.to_string();
            append_bytes(pb, rendered.as_bytes())
        }
        _ => -1,
    }
}

pub(crate) unsafe extern "C" fn json_object_string_to_json_string_impl(
    obj: *mut json_object,
    pb: *mut printbuf,
    _level: c_int,
    flags: c_int,
) -> c_int
{
    let Some(bytes) = object::string_bytes(obj) else {
        return -1;
    };

    append_literal(pb, b"\"\0");
    if escape_string(pb, bytes, flags) < 0
    {
        return -1;
    }
    append_literal(pb, b"\"\0")
}

fn current_double_format() -> Option<Vec<u8>>
{
    THREAD_DOUBLE_FORMAT.with(|tls| {
        if let Some(bytes) = tls.borrow().as_ref()
        {
            Some(bytes.clone())
        }
        else
        {
            GLOBAL_DOUBLE_FORMAT.lock().expect("global format mutex poisoned").clone()
        }
    })
}

fn format_uses_default_decimal_rule(format: &[u8]) -> bool
{
    format == DEFAULT_DOUBLE_FORMAT || !format.windows(3).any(|window| window == b".0f")
}

unsafe fn format_double(
    value: c_double,
    explicit_format: *const c_char,
    flags: c_int,
) -> Option<Vec<u8>>
{
    if value.is_nan()
    {
        return Some(b"NaN".to_vec());
    }
    if value.is_infinite()
    {
        return Some(if value.is_sign_positive() {
            b"Infinity".to_vec()
        } else {
            b"-Infinity".to_vec()
        });
    }

    let format_bytes = if explicit_format.is_null()
    {
        current_double_format().unwrap_or_else(|| DEFAULT_DOUBLE_FORMAT.to_vec())
    }
    else
    {
        CStr::from_ptr(explicit_format).to_bytes_with_nul().to_vec()
    };

    let mut buffer = [0_i8; 128];
    let rc = snprintf(
        buffer.as_mut_ptr(),
        buffer.len(),
        format_bytes.as_ptr().cast(),
        value,
    );
    if rc < 0
    {
        return None;
    }

    let mut rendered = CStr::from_ptr(buffer.as_ptr()).to_bytes().to_vec();
    if let Some(comma) = rendered.iter().position(|byte| *byte == b',')
    {
        rendered[comma] = b'.';
    }

    let has_decimal = rendered.contains(&b'.');
    let has_exponent = rendered.contains(&b'e');
    let looks_numeric = rendered.first().is_some_and(|first| first.is_ascii_digit()) ||
        (rendered.len() > 1 && rendered[0] == b'-' && rendered[1].is_ascii_digit());
    if looks_numeric &&
        !has_decimal &&
        !has_exponent &&
        format_uses_default_decimal_rule(&format_bytes)
    {
        rendered.extend_from_slice(b".0");
    }

    if (flags & object::JSON_C_TO_STRING_NOZERO) != 0
    {
        if let Some(dot) = rendered.iter().position(|byte| *byte == b'.')
        {
            let mut keep = dot + 1;
            for idx in dot + 1..rendered.len()
            {
                if rendered[idx] != b'0'
                {
                    keep = idx + 1;
                }
            }
            rendered.truncate(keep);
        }
    }

    Some(rendered)
}

pub(crate) unsafe extern "C" fn json_object_double_to_json_string_default_impl(
    obj: *mut json_object,
    pb: *mut printbuf,
    _level: c_int,
    flags: c_int,
) -> c_int
{
    let inner = object::as_json_box(obj).expect("double object must be valid");
    let object::JsonData::Double(value) = inner.data else {
        return -1;
    };
    let Some(rendered) = format_double(value, ptr::null(), flags) else {
        return -1;
    };
    append_bytes(pb, &rendered)
}

pub(crate) unsafe extern "C" fn json_object_double_to_json_string_impl(
    obj: *mut json_object,
    pb: *mut printbuf,
    _level: c_int,
    flags: c_int,
) -> c_int
{
    let inner = object::as_json_box(obj).expect("double object must be valid");
    let object::JsonData::Double(value) = inner.data else {
        return -1;
    };
    let Some(rendered) = format_double(value, inner.userdata.cast(), flags) else {
        return -1;
    };
    append_bytes(pb, &rendered)
}

pub(crate) unsafe extern "C" fn json_object_userdata_to_json_string_wrapper_impl(
    obj: *mut json_object,
    pb: *mut printbuf,
    level: c_int,
    flags: c_int,
) -> c_int
{
    json_object_userdata_to_json_string_impl(obj, pb, level, flags)
}

pub(crate) unsafe extern "C" fn json_object_userdata_to_json_string_impl(
    obj: *mut json_object,
    pb: *mut printbuf,
    _level: c_int,
    _flags: c_int,
) -> c_int
{
    let inner = object::as_json_box(obj).expect("object must be valid");
    if inner.userdata.is_null()
    {
        return -1;
    }

    let bytes = CStr::from_ptr(inner.userdata.cast()).to_bytes();
    append_bytes(pb, bytes)
}

pub(crate) unsafe extern "C" fn json_object_free_userdata_impl(
    _obj: *mut json_object,
    userdata: *mut c_void,
)
{
    free(userdata);
}

pub(crate) unsafe fn json_c_set_serialization_double_format_impl(
    double_format: *const c_char,
    global_or_thread: c_int,
) -> c_int
{
    let duplicated = if double_format.is_null()
    {
        None
    }
    else
    {
        Some(CStr::from_ptr(double_format).to_bytes_with_nul().to_vec())
    };

    match global_or_thread
    {
        object::JSON_C_OPTION_GLOBAL =>
        {
            THREAD_DOUBLE_FORMAT.with(|tls| {
                *tls.borrow_mut() = None;
            });
            *GLOBAL_DOUBLE_FORMAT.lock().expect("global format mutex poisoned") = duplicated;
            0
        }
        object::JSON_C_OPTION_THREAD =>
        {
            THREAD_DOUBLE_FORMAT.with(|tls| {
                *tls.borrow_mut() = duplicated;
            });
            0
        }
        _ =>
        {
            errors::set_last_err_fmt(format_args!(
                "json_c_set_serialization_double_format: invalid global_or_thread value: {}\n",
                global_or_thread
            ));
            -1
        }
    }
}
