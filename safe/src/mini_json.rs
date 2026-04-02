use crate::abi::*;
use std::ffi::CStr;
use std::fmt::Write;
use std::ptr;

const JSON_C_TO_STRING_PRETTY: c_int = 1 << 1;

enum JsonKind
{
    String(Vec<u8>),
    Int(i64),
    Array(Vec<*mut json_object>),
    Object(Vec<(Vec<u8>, *mut json_object)>),
}

struct MiniJson
{
    kind: JsonKind,
    serialized: Option<Vec<u8>>,
}

const JSON_NULL_BYTES: &[u8; 5] = b"null\0";

fn with_nul(bytes: &[u8]) -> Vec<u8>
{
    let mut data = Vec::with_capacity(bytes.len() + 1);
    data.extend_from_slice(bytes);
    data.push(0);
    data
}

unsafe fn wrap(kind: JsonKind) -> *mut json_object
{
    Box::into_raw(Box::new(MiniJson { kind, serialized: None })).cast()
}

unsafe fn as_json_mut<'a>(obj: *mut json_object) -> &'a mut MiniJson
{
    &mut *(obj.cast::<MiniJson>())
}

unsafe fn as_json<'a>(obj: *const json_object) -> &'a MiniJson
{
    &*(obj.cast::<MiniJson>())
}

fn append_indent(out: &mut String, indent: usize)
{
    for _ in 0..indent
    {
        out.push(' ');
    }
}

fn append_escaped_string(out: &mut String, bytes: &[u8])
{
    const HEX: &[u8; 16] = b"0123456789abcdef";

    out.push('"');
    for byte in bytes
    {
        match *byte
        {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x08 => out.push_str("\\b"),
            0x0c => out.push_str("\\f"),
            0x00..=0x1f =>
            {
                out.push_str("\\u00");
                out.push(HEX[(byte >> 4) as usize] as char);
                out.push(HEX[(byte & 0x0f) as usize] as char);
            }
            _ => out.push(*byte as char),
        }
    }
    out.push('"');
}

unsafe fn write_json(out: &mut String, obj: *mut json_object, pretty: bool, indent: usize)
{
    if obj.is_null()
    {
        out.push_str("null");
        return;
    }

    match &as_json(obj).kind
    {
        JsonKind::String(data) => append_escaped_string(out, &data[..data.len() - 1]),
        JsonKind::Int(value) =>
        {
            let _ = write!(out, "{value}");
        }
        JsonKind::Array(items) =>
        {
            out.push('[');
            if pretty && !items.is_empty()
            {
                out.push('\n');
                for (idx, item) in items.iter().enumerate()
                {
                    append_indent(out, indent + 2);
                    write_json(out, *item, true, indent + 2);
                    if idx + 1 != items.len()
                    {
                        out.push(',');
                    }
                    out.push('\n');
                }
                append_indent(out, indent);
                out.push(']');
                return;
            }

            for (idx, item) in items.iter().enumerate()
            {
                if idx != 0
                {
                    out.push(',');
                }
                write_json(out, *item, false, indent);
            }
            out.push(']');
        }
        JsonKind::Object(entries) =>
        {
            out.push('{');
            if pretty && !entries.is_empty()
            {
                out.push('\n');
                for (idx, (key, value)) in entries.iter().enumerate()
                {
                    append_indent(out, indent + 2);
                    append_escaped_string(out, key);
                    out.push(':');
                    write_json(out, *value, true, indent + 2);
                    if idx + 1 != entries.len()
                    {
                        out.push(',');
                    }
                    out.push('\n');
                }
                append_indent(out, indent);
                out.push('}');
                return;
            }

            for (idx, (key, value)) in entries.iter().enumerate()
            {
                if idx != 0
                {
                    out.push(',');
                }
                append_escaped_string(out, key);
                out.push(':');
                write_json(out, *value, false, indent);
            }
            out.push('}');
        }
    }
}

unsafe fn build_serialized(obj: *mut json_object, flags: c_int) -> Vec<u8>
{
    let mut out = String::new();
    write_json(&mut out, obj, (flags & JSON_C_TO_STRING_PRETTY) != 0, 0);
    with_nul(out.as_bytes())
}

fn hex_value(ch: u8) -> Option<u8>
{
    match ch
    {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        _ => None,
    }
}

pub(crate) unsafe fn json_object_put_impl(obj: *mut json_object) -> c_int
{
    if obj.is_null()
    {
        return 1;
    }

    let MiniJson { kind, serialized: _ } = *Box::from_raw(obj.cast::<MiniJson>());
    match kind
    {
        JsonKind::Array(items) =>
        {
            for item in items
            {
                if !item.is_null()
                {
                    json_object_put_impl(item);
                }
            }
        }
        JsonKind::Object(entries) =>
        {
            for (_, value) in entries
            {
                if !value.is_null()
                {
                    json_object_put_impl(value);
                }
            }
        }
        JsonKind::String(_) | JsonKind::Int(_) => {}
    }
    1
}

pub(crate) unsafe fn json_object_new_string_impl(data: *const c_char) -> *mut json_object
{
    if data.is_null()
    {
        return wrap(JsonKind::String(with_nul(&[])));
    }

    wrap(JsonKind::String(with_nul(CStr::from_ptr(data).to_bytes())))
}

pub(crate) unsafe fn json_object_new_string_len_impl(data: *const c_char, len: c_int) -> *mut json_object
{
    if len < 0
    {
        return ptr::null_mut();
    }

    let slice = if data.is_null()
    {
        &[]
    }
    else
    {
        std::slice::from_raw_parts(data.cast::<u8>(), len as usize)
    };
    wrap(JsonKind::String(with_nul(slice)))
}

pub(crate) unsafe fn json_object_get_string_impl(obj: *mut json_object) -> *const c_char
{
    if obj.is_null()
    {
        return ptr::null();
    }

    match &as_json(obj).kind
    {
        JsonKind::String(data) => data.as_ptr().cast(),
        JsonKind::Int(_) | JsonKind::Array(_) | JsonKind::Object(_) => ptr::null(),
    }
}

pub(crate) unsafe fn json_object_get_string_len_impl(obj: *const json_object) -> c_int
{
    if obj.is_null()
    {
        return 0;
    }

    match &as_json(obj).kind
    {
        JsonKind::String(data) => (data.len() - 1) as c_int,
        JsonKind::Int(_) | JsonKind::Array(_) | JsonKind::Object(_) => 0,
    }
}

pub(crate) unsafe fn json_object_set_string_impl(obj: *mut json_object, data: *const c_char) -> c_int
{
    if data.is_null()
    {
        return json_object_set_string_len_impl(obj, ptr::null(), 0);
    }
    json_object_set_string_len_impl(obj, data, CStr::from_ptr(data).to_bytes().len() as c_int)
}

pub(crate) unsafe fn json_object_set_string_len_impl(
    obj: *mut json_object,
    data: *const c_char,
    len: c_int,
) -> c_int
{
    if obj.is_null() || len < 0
    {
        return 0;
    }

    let slice = if data.is_null()
    {
        &[]
    }
    else
    {
        std::slice::from_raw_parts(data.cast::<u8>(), len as usize)
    };

    let json = as_json_mut(obj);
    match &mut json.kind
    {
        JsonKind::String(value) =>
        {
            *value = with_nul(slice);
            json.serialized = None;
            1
        }
        JsonKind::Int(_) | JsonKind::Array(_) | JsonKind::Object(_) => 0,
    }
}

pub(crate) unsafe fn json_object_new_int_impl(value: int32_t) -> *mut json_object
{
    wrap(JsonKind::Int(value as i64))
}

pub(crate) unsafe fn json_object_get_int_impl(obj: *const json_object) -> int32_t
{
    if obj.is_null()
    {
        return 0;
    }

    match &as_json(obj).kind
    {
        JsonKind::Int(value) => *value as int32_t,
        JsonKind::String(_) | JsonKind::Array(_) | JsonKind::Object(_) => 0,
    }
}

pub(crate) unsafe fn json_object_new_array_impl() -> *mut json_object
{
    wrap(JsonKind::Array(Vec::new()))
}

pub(crate) unsafe fn json_object_new_array_ext_impl(_size: c_int) -> *mut json_object
{
    json_object_new_array_impl()
}

pub(crate) unsafe fn json_object_array_add_impl(obj: *mut json_object, value: *mut json_object) -> c_int
{
    if obj.is_null()
    {
        return -1;
    }

    let json = as_json_mut(obj);
    match &mut json.kind
    {
        JsonKind::Array(items) =>
        {
            items.push(value);
            json.serialized = None;
            0
        }
        JsonKind::String(_) | JsonKind::Int(_) | JsonKind::Object(_) => -1,
    }
}

pub(crate) unsafe fn json_object_array_get_idx_impl(
    obj: *const json_object,
    idx: size_t,
) -> *mut json_object
{
    if obj.is_null()
    {
        return ptr::null_mut();
    }

    match &as_json(obj).kind
    {
        JsonKind::Array(items) => items.get(idx).copied().unwrap_or(ptr::null_mut()),
        JsonKind::String(_) | JsonKind::Int(_) | JsonKind::Object(_) => ptr::null_mut(),
    }
}

pub(crate) unsafe fn json_object_new_object_impl() -> *mut json_object
{
    wrap(JsonKind::Object(Vec::new()))
}

pub(crate) unsafe fn json_object_object_add_impl(
    obj: *mut json_object,
    key: *const c_char,
    value: *mut json_object,
) -> c_int
{
    if obj.is_null() || key.is_null()
    {
        return -1;
    }

    let json = as_json_mut(obj);
    match &mut json.kind
    {
        JsonKind::Object(entries) =>
        {
            entries.push((CStr::from_ptr(key).to_bytes().to_vec(), value));
            json.serialized = None;
            0
        }
        JsonKind::String(_) | JsonKind::Int(_) | JsonKind::Array(_) => -1,
    }
}

pub(crate) unsafe fn json_object_to_json_string_impl(obj: *mut json_object) -> *const c_char
{
    json_object_to_json_string_length_impl(obj, 0, ptr::null_mut())
}

pub(crate) unsafe fn json_object_to_json_string_ext_impl(
    obj: *mut json_object,
    flags: c_int,
) -> *const c_char
{
    json_object_to_json_string_length_impl(obj, flags, ptr::null_mut())
}

pub(crate) unsafe fn json_object_to_json_string_length_impl(
    obj: *mut json_object,
    flags: c_int,
    length: *mut size_t,
) -> *const c_char
{
    if obj.is_null()
    {
        if !length.is_null()
        {
            *length = 4;
        }
        return JSON_NULL_BYTES.as_ptr().cast();
    }

    let rendered = build_serialized(obj, flags);
    let json = as_json_mut(obj);
    json.serialized = Some(rendered);

    let bytes = json.serialized.as_ref().unwrap();
    if !length.is_null()
    {
        *length = bytes.len() - 1;
    }
    bytes.as_ptr().cast()
}

pub(crate) unsafe fn json_tokener_parse_impl(text: *const c_char) -> *mut json_object
{
    let bytes = if text.is_null()
    {
        return ptr::null_mut();
    }
    else
    {
        CStr::from_ptr(text).to_bytes()
    };

    if bytes.len() < 2 || bytes[0] != b'"'
    {
        return ptr::null_mut();
    }

    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 1;
    while idx < bytes.len()
    {
        match bytes[idx]
        {
            b'"' =>
            {
                if idx + 1 != bytes.len()
                {
                    return ptr::null_mut();
                }
                return wrap(JsonKind::String(with_nul(&out)));
            }
            b'\\' =>
            {
                idx += 1;
                if idx >= bytes.len()
                {
                    return ptr::null_mut();
                }
                match bytes[idx]
                {
                    b'"' => out.push(b'"'),
                    b'\\' => out.push(b'\\'),
                    b'/' => out.push(b'/'),
                    b'b' => out.push(0x08),
                    b'f' => out.push(0x0c),
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'u' =>
                    {
                        if idx + 4 >= bytes.len()
                        {
                            return ptr::null_mut();
                        }
                        let hi1 = hex_value(bytes[idx + 1]);
                        let hi2 = hex_value(bytes[idx + 2]);
                        let lo1 = hex_value(bytes[idx + 3]);
                        let lo2 = hex_value(bytes[idx + 4]);
                        let (Some(hi1), Some(hi2), Some(lo1), Some(lo2)) = (hi1, hi2, lo1, lo2) else {
                            return ptr::null_mut();
                        };

                        let value =
                            ((hi1 as u16) << 12) | ((hi2 as u16) << 8) | ((lo1 as u16) << 4) | lo2 as u16;
                        if value > 0xff
                        {
                            return ptr::null_mut();
                        }
                        out.push(value as u8);
                        idx += 4;
                    }
                    _ => return ptr::null_mut(),
                }
            }
            byte => out.push(byte),
        }
        idx += 1;
    }

    ptr::null_mut()
}
