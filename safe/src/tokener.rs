use crate::abi::*;
use crate::{locale, numeric, object, printbuf as printbuf_impl, utf8};
use core::cmp::min;
use core::mem::size_of;
use std::ptr;

extern "C" {
    fn calloc(nmemb: size_t, size: size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strlen(s: *const c_char) -> size_t;
}

const JSON_TOKENER_DEFAULT_DEPTH: c_int = 32;

const JSON_TOKENER_STRICT: c_int = 0x01;
const JSON_TOKENER_ALLOW_TRAILING_CHARS: c_int = 0x02;
const JSON_TOKENER_VALIDATE_UTF8: c_int = 0x10;

const JSON_TOKENER_SUCCESS: json_tokener_error = 0;
const JSON_TOKENER_CONTINUE: json_tokener_error = 1;
const JSON_TOKENER_ERROR_DEPTH: json_tokener_error = 2;
const JSON_TOKENER_ERROR_PARSE_EOF: json_tokener_error = 3;
const JSON_TOKENER_ERROR_PARSE_UNEXPECTED: json_tokener_error = 4;
const JSON_TOKENER_ERROR_PARSE_NULL: json_tokener_error = 5;
const JSON_TOKENER_ERROR_PARSE_BOOLEAN: json_tokener_error = 6;
const JSON_TOKENER_ERROR_PARSE_NUMBER: json_tokener_error = 7;
const JSON_TOKENER_ERROR_PARSE_ARRAY: json_tokener_error = 8;
const JSON_TOKENER_ERROR_PARSE_OBJECT_KEY_NAME: json_tokener_error = 9;
const JSON_TOKENER_ERROR_PARSE_OBJECT_KEY_SEP: json_tokener_error = 10;
const JSON_TOKENER_ERROR_PARSE_OBJECT_VALUE_SEP: json_tokener_error = 11;
const JSON_TOKENER_ERROR_PARSE_STRING: json_tokener_error = 12;
const JSON_TOKENER_ERROR_PARSE_COMMENT: json_tokener_error = 13;
const JSON_TOKENER_ERROR_PARSE_UTF8_STRING: json_tokener_error = 14;
const JSON_TOKENER_ERROR_MEMORY: json_tokener_error = 15;
const JSON_TOKENER_ERROR_SIZE: json_tokener_error = 16;

const JSON_TOKENER_STATE_EATWS: json_tokener_state = 0;
const JSON_TOKENER_STATE_START: json_tokener_state = 1;
const JSON_TOKENER_STATE_FINISH: json_tokener_state = 2;
const JSON_TOKENER_STATE_NULL: json_tokener_state = 3;
const JSON_TOKENER_STATE_COMMENT_START: json_tokener_state = 4;
const JSON_TOKENER_STATE_COMMENT: json_tokener_state = 5;
const JSON_TOKENER_STATE_COMMENT_EOL: json_tokener_state = 6;
const JSON_TOKENER_STATE_COMMENT_END: json_tokener_state = 7;
const JSON_TOKENER_STATE_STRING: json_tokener_state = 8;
const JSON_TOKENER_STATE_STRING_ESCAPE: json_tokener_state = 9;
const JSON_TOKENER_STATE_ESCAPE_UNICODE: json_tokener_state = 10;
const JSON_TOKENER_STATE_ESCAPE_UNICODE_NEED_ESCAPE: json_tokener_state = 11;
const JSON_TOKENER_STATE_ESCAPE_UNICODE_NEED_U: json_tokener_state = 12;
const JSON_TOKENER_STATE_BOOLEAN: json_tokener_state = 13;
const JSON_TOKENER_STATE_NUMBER: json_tokener_state = 14;
const JSON_TOKENER_STATE_ARRAY: json_tokener_state = 15;
const JSON_TOKENER_STATE_ARRAY_ADD: json_tokener_state = 16;
const JSON_TOKENER_STATE_ARRAY_SEP: json_tokener_state = 17;
const JSON_TOKENER_STATE_OBJECT_FIELD_START: json_tokener_state = 18;
const JSON_TOKENER_STATE_OBJECT_FIELD: json_tokener_state = 19;
const JSON_TOKENER_STATE_OBJECT_FIELD_END: json_tokener_state = 20;
const JSON_TOKENER_STATE_OBJECT_VALUE: json_tokener_state = 21;
const JSON_TOKENER_STATE_OBJECT_VALUE_ADD: json_tokener_state = 22;
const JSON_TOKENER_STATE_OBJECT_SEP: json_tokener_state = 23;
const JSON_TOKENER_STATE_ARRAY_AFTER_SEP: json_tokener_state = 24;
const JSON_TOKENER_STATE_OBJECT_FIELD_START_AFTER_SEP: json_tokener_state = 25;
const JSON_TOKENER_STATE_INF: json_tokener_state = 26;

const JSON_NULL_STR: &[u8; 4] = b"null";
const JSON_INF_STR: &[u8; 8] = b"Infinity";
const JSON_INF_STR_INVERT: &[u8; 8] = b"iNFINITY";
const JSON_NAN_STR: &[u8; 3] = b"NaN";
const JSON_TRUE_STR: &[u8; 4] = b"true";
const JSON_FALSE_STR: &[u8; 5] = b"false";

const UNKNOWN_ERROR_DESC: &[u8] =
    b"Unknown error, invalid json_tokener_error value passed to json_tokener_error_desc()\0";
const JSON_TOKENER_ERRORS: [&[u8]; 17] = [
    b"success\0",
    b"continue\0",
    b"nesting too deep\0",
    b"unexpected end of data\0",
    b"unexpected character\0",
    b"null expected\0",
    b"boolean expected\0",
    b"number expected\0",
    b"array value separator ',' expected\0",
    b"quoted object property name expected\0",
    b"object property name separator ':' expected\0",
    b"object value separator ',' expected\0",
    b"invalid string sequence\0",
    b"expected comment\0",
    b"invalid utf-8 string\0",
    b"buffer size overflow\0",
    b"out of memory\0",
];

fn is_ws_char(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

unsafe fn stack_entry(tok: *mut json_tokener, depth: c_int) -> *mut json_tokener_srec {
    (*tok).stack.add(depth as usize)
}

unsafe fn active_entry(tok: *mut json_tokener) -> *mut json_tokener_srec {
    stack_entry(tok, (*tok).depth)
}

unsafe fn state(tok: *mut json_tokener) -> json_tokener_state {
    (*active_entry(tok)).state
}

unsafe fn set_state(tok: *mut json_tokener, value: json_tokener_state) {
    (*active_entry(tok)).state = value;
}

unsafe fn saved_state(tok: *mut json_tokener) -> json_tokener_state {
    (*active_entry(tok)).saved_state
}

unsafe fn set_saved_state(tok: *mut json_tokener, value: json_tokener_state) {
    (*active_entry(tok)).saved_state = value;
}

unsafe fn current(tok: *mut json_tokener) -> *mut json_object {
    (*active_entry(tok)).current
}

unsafe fn set_current(tok: *mut json_tokener, value: *mut json_object) {
    (*active_entry(tok)).current = value;
}

unsafe fn obj_field_name(tok: *mut json_tokener) -> *mut c_char {
    (*active_entry(tok)).obj_field_name
}

unsafe fn set_obj_field_name(tok: *mut json_tokener, value: *mut c_char) {
    (*active_entry(tok)).obj_field_name = value;
}

unsafe fn printbuf_len(pb: *mut crate::abi::printbuf) -> c_int {
    (*pb).bpos
}

unsafe fn buffer_bytes<'a>(pb: *mut crate::abi::printbuf, len: usize) -> &'a [u8] {
    std::slice::from_raw_parts((*pb).buf.cast::<u8>(), len)
}

unsafe fn printbuf_append_checked(
    tok: *mut json_tokener,
    pb: *mut crate::abi::printbuf,
    buf: *const c_char,
    len: c_int,
) -> bool {
    if printbuf_impl::printbuf_memappend_impl(pb, buf, len) < 0 {
        (*tok).err = JSON_TOKENER_ERROR_MEMORY;
        false
    } else {
        true
    }
}

unsafe fn printbuf_append_byte_checked(
    tok: *mut json_tokener,
    pb: *mut crate::abi::printbuf,
    byte: u8,
) -> bool {
    let buf = [byte as c_char];
    printbuf_append_checked(tok, pb, buf.as_ptr(), 1)
}

unsafe fn append_replacement_char_checked(
    tok: *mut json_tokener,
    pb: *mut crate::abi::printbuf,
) -> bool {
    if utf8::append_replacement_char(pb) < 0 {
        (*tok).err = JSON_TOKENER_ERROR_MEMORY;
        false
    } else {
        true
    }
}

unsafe fn append_codepoint_checked(
    tok: *mut json_tokener,
    pb: *mut crate::abi::printbuf,
    ucs: c_uint,
) -> bool {
    if utf8::append_codepoint(pb, ucs) < 0 {
        (*tok).err = JSON_TOKENER_ERROR_MEMORY;
        false
    } else {
        true
    }
}

unsafe fn ascii_prefix_matches(tok: *mut json_tokener, literal: &[u8], size: usize) -> bool {
    let strict = ((*tok).flags & JSON_TOKENER_STRICT) != 0;
    let actual = buffer_bytes((*tok).pb, size);
    actual
        .iter()
        .zip(literal.iter())
        .take(size)
        .all(|(lhs, rhs)| *lhs == *rhs || (!strict && lhs.eq_ignore_ascii_case(rhs)))
}

unsafe fn json_tokener_reset_level(tok: *mut json_tokener, depth: c_int) {
    let entry = stack_entry(tok, depth);
    (*entry).state = JSON_TOKENER_STATE_EATWS;
    (*entry).saved_state = JSON_TOKENER_STATE_START;
    (*entry).obj = ptr::null_mut();
    object::json_object_put_impl((*entry).current);
    (*entry).current = ptr::null_mut();
    free((*entry).obj_field_name.cast());
    (*entry).obj_field_name = ptr::null_mut();
}

unsafe fn peek_char(
    tok: *mut json_tokener,
    cursor: *const u8,
    len: c_int,
    c: &mut u8,
    n_bytes: &mut c_uint,
) -> bool {
    if len >= 0 && (*tok).char_offset == len {
        (*tok).err = if (*tok).depth == 0
            && state(tok) == JSON_TOKENER_STATE_EATWS
            && saved_state(tok) == JSON_TOKENER_STATE_FINISH
        {
            JSON_TOKENER_SUCCESS
        } else {
            JSON_TOKENER_CONTINUE
        };
        return false;
    }

    let next = *cursor;
    if ((*tok).flags & JSON_TOKENER_VALIDATE_UTF8) != 0
        && !utf8::validate_utf8(next as c_char, n_bytes)
    {
        (*tok).err = JSON_TOKENER_ERROR_PARSE_UTF8_STRING;
        return false;
    }

    *c = next;
    true
}

unsafe fn advance_char(cursor: &mut *const u8, tok: *mut json_tokener, c: u8) -> u8 {
    *cursor = (*cursor).add(1);
    (*tok).char_offset += 1;
    c
}

unsafe fn parse_string_escape(tok: *mut json_tokener, c: u8) -> bool {
    match c {
        b'"' | b'\\' | b'/' => {
            if !printbuf_append_byte_checked(tok, (*tok).pb, c) {
                return false;
            }
            set_state(tok, saved_state(tok));
            true
        }
        b'b' | b'n' | b'r' | b't' | b'f' => {
            let mapped = match c {
                b'b' => b'\x08',
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                _ => b'\x0c',
            };
            if !printbuf_append_byte_checked(tok, (*tok).pb, mapped) {
                return false;
            }
            set_state(tok, saved_state(tok));
            true
        }
        b'u' => {
            (*tok).ucs_char = 0;
            (*tok).st_pos = 0;
            set_state(tok, JSON_TOKENER_STATE_ESCAPE_UNICODE);
            true
        }
        _ => {
            (*tok).err = JSON_TOKENER_ERROR_PARSE_STRING;
            false
        }
    }
}

unsafe fn handle_completed_number(tok: *mut json_tokener) -> bool {
    let buf = (*(*tok).pb).buf;
    let len = printbuf_len((*tok).pb);
    let mut num64 = 0_i64;
    let mut numuint64 = 0_u64;
    let mut numd = 0_f64;

    if (*tok).is_double == 0
        && *buf.cast::<u8>() == b'-'
        && numeric::json_parse_int64_impl(buf, &mut num64) == 0
    {
        if *numeric::errno_location() == 34 && ((*tok).flags & JSON_TOKENER_STRICT) != 0 {
            (*tok).err = JSON_TOKENER_ERROR_PARSE_NUMBER;
            return false;
        }
        let current = object::json_object_new_int64_impl(num64);
        if current.is_null() {
            (*tok).err = JSON_TOKENER_ERROR_MEMORY;
            return false;
        }
        set_current(tok, current);
    } else if (*tok).is_double == 0
        && *buf.cast::<u8>() != b'-'
        && numeric::json_parse_uint64_impl(buf, &mut numuint64) == 0
    {
        if *numeric::errno_location() == 34 && ((*tok).flags & JSON_TOKENER_STRICT) != 0 {
            (*tok).err = JSON_TOKENER_ERROR_PARSE_NUMBER;
            return false;
        }
        if numuint64 != 0 && *buf.cast::<u8>() == b'0' && ((*tok).flags & JSON_TOKENER_STRICT) != 0
        {
            (*tok).err = JSON_TOKENER_ERROR_PARSE_NUMBER;
            return false;
        }

        let current = if numuint64 <= i64::MAX as u64 {
            object::json_object_new_int64_impl(numuint64 as i64)
        } else {
            object::json_object_new_uint64_impl(numuint64)
        };
        if current.is_null() {
            (*tok).err = JSON_TOKENER_ERROR_MEMORY;
            return false;
        }
        set_current(tok, current);
    } else if (*tok).is_double != 0 && numeric::json_parse_double_len_impl(buf, len, &mut numd) == 0
    {
        let current = object::json_object_new_double_s_impl(numd, buf);
        if current.is_null() {
            (*tok).err = JSON_TOKENER_ERROR_MEMORY;
            return false;
        }
        set_current(tok, current);
    } else {
        (*tok).err = JSON_TOKENER_ERROR_PARSE_NUMBER;
        return false;
    }

    set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
    set_state(tok, JSON_TOKENER_STATE_EATWS);
    true
}

pub(crate) unsafe fn json_tokener_error_desc_impl(jerr: json_tokener_error) -> *const c_char {
    let idx = jerr as usize;
    if idx >= JSON_TOKENER_ERRORS.len() {
        UNKNOWN_ERROR_DESC.as_ptr().cast()
    } else {
        JSON_TOKENER_ERRORS[idx].as_ptr().cast()
    }
}

pub(crate) unsafe fn json_tokener_get_error_impl(tok: *mut json_tokener) -> json_tokener_error {
    if tok.is_null() {
        JSON_TOKENER_SUCCESS
    } else {
        (*tok).err
    }
}

pub(crate) unsafe fn json_tokener_new_ex_impl(depth: c_int) -> *mut json_tokener {
    if depth < 1 {
        return ptr::null_mut();
    }

    let tok = calloc(1, size_of::<json_tokener>()).cast::<json_tokener>();
    if tok.is_null() {
        return ptr::null_mut();
    }

    (*tok).stack =
        calloc(depth as size_t, size_of::<json_tokener_srec>()).cast::<json_tokener_srec>();
    if (*tok).stack.is_null() {
        free(tok.cast());
        return ptr::null_mut();
    }

    (*tok).pb = printbuf_impl::printbuf_new_impl();
    if (*tok).pb.is_null() {
        free((*tok).stack.cast());
        free(tok.cast());
        return ptr::null_mut();
    }

    (*tok).max_depth = depth;
    json_tokener_reset_impl(tok);
    tok
}

pub(crate) unsafe fn json_tokener_new_impl() -> *mut json_tokener {
    json_tokener_new_ex_impl(JSON_TOKENER_DEFAULT_DEPTH)
}

pub(crate) unsafe fn json_tokener_free_impl(tok: *mut json_tokener) {
    if tok.is_null() {
        return;
    }

    json_tokener_reset_impl(tok);
    if !(*tok).pb.is_null() {
        printbuf_impl::printbuf_free_impl((*tok).pb);
    }
    free((*tok).stack.cast());
    free(tok.cast());
}

pub(crate) unsafe fn json_tokener_reset_impl(tok: *mut json_tokener) {
    if tok.is_null() {
        return;
    }

    let mut ii = (*tok).depth;
    loop {
        json_tokener_reset_level(tok, ii);
        if ii == 0 {
            break;
        }
        ii -= 1;
    }
    (*tok).depth = 0;
    (*tok).err = JSON_TOKENER_SUCCESS;
}

pub(crate) unsafe fn json_tokener_parse_impl(str_: *const c_char) -> *mut json_object {
    let mut ignored = JSON_TOKENER_SUCCESS;
    json_tokener_parse_verbose_impl(str_, &mut ignored)
}

pub(crate) unsafe fn json_tokener_parse_verbose_impl(
    str_: *const c_char,
    error: *mut json_tokener_error,
) -> *mut json_object {
    let tok = json_tokener_new_impl();
    if tok.is_null() {
        return ptr::null_mut();
    }

    let mut obj = json_tokener_parse_ex_impl(tok, str_, -1);
    if !error.is_null() {
        *error = (*tok).err;
    }
    if (*tok).err != JSON_TOKENER_SUCCESS {
        if !obj.is_null() {
            object::json_object_put_impl(obj);
        }
        obj = ptr::null_mut();
    }

    json_tokener_free_impl(tok);
    obj
}

pub(crate) unsafe fn json_tokener_set_flags_impl(tok: *mut json_tokener, flags: c_int) {
    if tok.is_null() {
        return;
    }
    (*tok).flags = flags;
}

pub(crate) unsafe fn json_tokener_get_parse_end_impl(tok: *mut json_tokener) -> size_t {
    if tok.is_null() {
        return 0;
    }
    debug_assert!((*tok).char_offset >= 0);
    (*tok).char_offset as size_t
}

pub(crate) unsafe fn json_tokener_parse_ex_impl(
    tok: *mut json_tokener,
    str_: *const c_char,
    len: c_int,
) -> *mut json_object {
    if tok.is_null() || str_.is_null() {
        return ptr::null_mut();
    }

    let mut obj = ptr::null_mut();
    let mut c = b'\x01';
    let mut n_bytes = 0_u32;

    (*tok).char_offset = 0;
    (*tok).err = JSON_TOKENER_SUCCESS;

    if len < -1 || (len == -1 && strlen(str_) > i32::MAX as size_t) {
        (*tok).err = JSON_TOKENER_ERROR_SIZE;
        return ptr::null_mut();
    }

    let _locale_guard = match locale::NumericLocaleGuard::enter() {
        Some(guard) => guard,
        None => return ptr::null_mut(),
    };

    let mut cursor = str_.cast::<u8>();

    'parse: while peek_char(tok, cursor, len, &mut c, &mut n_bytes) {
        loop {
            match state(tok) {
                JSON_TOKENER_STATE_EATWS => {
                    while is_ws_char(c) {
                        if advance_char(&mut cursor, tok, c) == 0
                            || !peek_char(tok, cursor, len, &mut c, &mut n_bytes)
                        {
                            break 'parse;
                        }
                    }
                    if c == b'/' && ((*tok).flags & JSON_TOKENER_STRICT) == 0 {
                        printbuf_impl::printbuf_reset_impl((*tok).pb);
                        if !printbuf_append_byte_checked(tok, (*tok).pb, c) {
                            break;
                        }
                        set_state(tok, JSON_TOKENER_STATE_COMMENT_START);
                    } else {
                        set_state(tok, saved_state(tok));
                        continue;
                    }
                }
                JSON_TOKENER_STATE_START => match c {
                    b'{' => {
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                        set_saved_state(tok, JSON_TOKENER_STATE_OBJECT_FIELD_START);
                        let current = object::json_object_new_object_impl();
                        if current.is_null() {
                            (*tok).err = JSON_TOKENER_ERROR_MEMORY;
                            break;
                        }
                        set_current(tok, current);
                    }
                    b'[' => {
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                        set_saved_state(tok, JSON_TOKENER_STATE_ARRAY);
                        let current = object::json_object_new_array_impl();
                        if current.is_null() {
                            (*tok).err = JSON_TOKENER_ERROR_MEMORY;
                            break;
                        }
                        set_current(tok, current);
                    }
                    b'I' | b'i' => {
                        set_state(tok, JSON_TOKENER_STATE_INF);
                        printbuf_impl::printbuf_reset_impl((*tok).pb);
                        (*tok).st_pos = 0;
                        continue;
                    }
                    b'N' | b'n' => {
                        set_state(tok, JSON_TOKENER_STATE_NULL);
                        printbuf_impl::printbuf_reset_impl((*tok).pb);
                        (*tok).st_pos = 0;
                        continue;
                    }
                    b'\'' => {
                        if ((*tok).flags & JSON_TOKENER_STRICT) != 0 {
                            (*tok).err = JSON_TOKENER_ERROR_PARSE_UNEXPECTED;
                            break;
                        }
                        set_state(tok, JSON_TOKENER_STATE_STRING);
                        printbuf_impl::printbuf_reset_impl((*tok).pb);
                        (*tok).quote_char = c as c_char;
                    }
                    b'"' => {
                        set_state(tok, JSON_TOKENER_STATE_STRING);
                        printbuf_impl::printbuf_reset_impl((*tok).pb);
                        (*tok).quote_char = c as c_char;
                    }
                    b'T' | b't' | b'F' | b'f' => {
                        set_state(tok, JSON_TOKENER_STATE_BOOLEAN);
                        printbuf_impl::printbuf_reset_impl((*tok).pb);
                        (*tok).st_pos = 0;
                        continue;
                    }
                    b'0'..=b'9' | b'-' => {
                        set_state(tok, JSON_TOKENER_STATE_NUMBER);
                        printbuf_impl::printbuf_reset_impl((*tok).pb);
                        (*tok).is_double = 0;
                        continue;
                    }
                    _ => {
                        (*tok).err = JSON_TOKENER_ERROR_PARSE_UNEXPECTED;
                        break;
                    }
                },
                JSON_TOKENER_STATE_FINISH => {
                    if (*tok).depth == 0 {
                        break 'parse;
                    }
                    obj = object::json_object_get_impl(current(tok));
                    json_tokener_reset_level(tok, (*tok).depth);
                    (*tok).depth -= 1;
                    continue;
                }
                JSON_TOKENER_STATE_INF => {
                    while (*tok).st_pos < JSON_INF_STR.len() as c_int {
                        let inf_char = *cursor;
                        if inf_char != JSON_INF_STR[(*tok).st_pos as usize]
                            && (((*tok).flags & JSON_TOKENER_STRICT) != 0
                                || inf_char != JSON_INF_STR_INVERT[(*tok).st_pos as usize])
                        {
                            (*tok).err = JSON_TOKENER_ERROR_PARSE_UNEXPECTED;
                            break;
                        }
                        (*tok).st_pos += 1;
                        let _ = advance_char(&mut cursor, tok, c);
                        if !peek_char(tok, cursor, len, &mut c, &mut n_bytes) {
                            break 'parse;
                        }
                    }
                    if (*tok).err != JSON_TOKENER_SUCCESS {
                        break;
                    }
                    if (*tok).st_pos < JSON_INF_STR.len() as c_int {
                        break;
                    }

                    let is_negative =
                        printbuf_len((*tok).pb) > 0 && *(*(*tok).pb).buf.cast::<u8>() == b'-';
                    let current = object::json_object_new_double_impl(if is_negative {
                        -f64::INFINITY
                    } else {
                        f64::INFINITY
                    });
                    if current.is_null() {
                        (*tok).err = JSON_TOKENER_ERROR_MEMORY;
                        break;
                    }
                    set_current(tok, current);
                    set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                    set_state(tok, JSON_TOKENER_STATE_EATWS);
                    continue;
                }
                JSON_TOKENER_STATE_NULL => {
                    if !printbuf_append_byte_checked(tok, (*tok).pb, c) {
                        break;
                    }

                    let size = min((*tok).st_pos + 1, JSON_NULL_STR.len() as c_int) as usize;
                    let size_nan = min((*tok).st_pos + 1, JSON_NAN_STR.len() as c_int) as usize;
                    if ascii_prefix_matches(tok, JSON_NULL_STR, size) {
                        if (*tok).st_pos == JSON_NULL_STR.len() as c_int {
                            set_current(tok, ptr::null_mut());
                            set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                            set_state(tok, JSON_TOKENER_STATE_EATWS);
                            continue;
                        }
                    } else if ascii_prefix_matches(tok, JSON_NAN_STR, size_nan) {
                        if (*tok).st_pos == JSON_NAN_STR.len() as c_int {
                            let current = object::json_object_new_double_impl(f64::NAN);
                            if current.is_null() {
                                (*tok).err = JSON_TOKENER_ERROR_MEMORY;
                                break;
                            }
                            set_current(tok, current);
                            set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                            set_state(tok, JSON_TOKENER_STATE_EATWS);
                            continue;
                        }
                    } else {
                        (*tok).err = JSON_TOKENER_ERROR_PARSE_NULL;
                        break;
                    }
                    (*tok).st_pos += 1;
                }
                JSON_TOKENER_STATE_COMMENT_START => {
                    if c == b'*' {
                        set_state(tok, JSON_TOKENER_STATE_COMMENT);
                    } else if c == b'/' {
                        set_state(tok, JSON_TOKENER_STATE_COMMENT_EOL);
                    } else {
                        (*tok).err = JSON_TOKENER_ERROR_PARSE_COMMENT;
                        break;
                    }
                    if !printbuf_append_byte_checked(tok, (*tok).pb, c) {
                        break;
                    }
                }
                JSON_TOKENER_STATE_COMMENT => {
                    let case_start = cursor;
                    while c != b'*' {
                        if advance_char(&mut cursor, tok, c) == 0
                            || !peek_char(tok, cursor, len, &mut c, &mut n_bytes)
                        {
                            let span = cursor.offset_from(case_start) as c_int;
                            let _ =
                                printbuf_append_checked(tok, (*tok).pb, case_start.cast(), span);
                            break 'parse;
                        }
                    }
                    let span = cursor.offset_from(case_start) as c_int + 1;
                    if !printbuf_append_checked(tok, (*tok).pb, case_start.cast(), span) {
                        break;
                    }
                    set_state(tok, JSON_TOKENER_STATE_COMMENT_END);
                }
                JSON_TOKENER_STATE_COMMENT_EOL => {
                    let case_start = cursor;
                    while c != b'\n' {
                        if advance_char(&mut cursor, tok, c) == 0
                            || !peek_char(tok, cursor, len, &mut c, &mut n_bytes)
                        {
                            let span = cursor.offset_from(case_start) as c_int;
                            let _ =
                                printbuf_append_checked(tok, (*tok).pb, case_start.cast(), span);
                            break 'parse;
                        }
                    }
                    if !printbuf_append_checked(
                        tok,
                        (*tok).pb,
                        case_start.cast(),
                        cursor.offset_from(case_start) as c_int,
                    ) {
                        break;
                    }
                    set_state(tok, JSON_TOKENER_STATE_EATWS);
                }
                JSON_TOKENER_STATE_COMMENT_END => {
                    if !printbuf_append_byte_checked(tok, (*tok).pb, c) {
                        break;
                    }
                    if c == b'/' {
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                    } else {
                        set_state(tok, JSON_TOKENER_STATE_COMMENT);
                    }
                }
                JSON_TOKENER_STATE_STRING => {
                    let case_start = cursor;
                    loop {
                        if c == (*tok).quote_char as u8 {
                            if !printbuf_append_checked(
                                tok,
                                (*tok).pb,
                                case_start.cast(),
                                cursor.offset_from(case_start) as c_int,
                            ) {
                                break;
                            }
                            let current = object::json_object_new_string_len_impl(
                                (*(*tok).pb).buf,
                                (*(*tok).pb).bpos,
                            );
                            if current.is_null() {
                                (*tok).err = JSON_TOKENER_ERROR_MEMORY;
                                break;
                            }
                            set_current(tok, current);
                            set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                            set_state(tok, JSON_TOKENER_STATE_EATWS);
                            break;
                        } else if c == b'\\' {
                            if !printbuf_append_checked(
                                tok,
                                (*tok).pb,
                                case_start.cast(),
                                cursor.offset_from(case_start) as c_int,
                            ) {
                                break;
                            }
                            set_saved_state(tok, JSON_TOKENER_STATE_STRING);
                            set_state(tok, JSON_TOKENER_STATE_STRING_ESCAPE);
                            break;
                        }
                        if advance_char(&mut cursor, tok, c) == 0
                            || !peek_char(tok, cursor, len, &mut c, &mut n_bytes)
                        {
                            let _ = printbuf_append_checked(
                                tok,
                                (*tok).pb,
                                case_start.cast(),
                                cursor.offset_from(case_start) as c_int,
                            );
                            break 'parse;
                        }
                    }
                }
                JSON_TOKENER_STATE_STRING_ESCAPE => {
                    if !parse_string_escape(tok, c) {
                        break;
                    }
                }
                JSON_TOKENER_STATE_ESCAPE_UNICODE => {
                    loop {
                        if c == 0 || !utf8::is_hex_char(c) {
                            (*tok).err = JSON_TOKENER_ERROR_PARSE_STRING;
                            break;
                        }
                        (*tok).ucs_char |= utf8::hex_digit(c) << ((3 - (*tok).st_pos) * 4);
                        (*tok).st_pos += 1;
                        if (*tok).st_pos >= 4 {
                            break;
                        }
                        let _ = advance_char(&mut cursor, tok, c);
                        if !peek_char(tok, cursor, len, &mut c, &mut n_bytes) {
                            break 'parse;
                        }
                    }
                    if (*tok).err != JSON_TOKENER_SUCCESS {
                        break;
                    }
                    if (*tok).st_pos < 4 {
                        break;
                    }
                    (*tok).st_pos = 0;

                    if (*tok).high_surrogate != 0 {
                        if utf8::is_low_surrogate((*tok).ucs_char) {
                            (*tok).ucs_char =
                                utf8::decode_surrogate_pair((*tok).high_surrogate, (*tok).ucs_char);
                        } else if !append_replacement_char_checked(tok, (*tok).pb) {
                            break;
                        }
                        (*tok).high_surrogate = 0;
                    }

                    if (*tok).ucs_char < 0x80
                        || (*tok).ucs_char < 0x800
                        || (*tok).ucs_char < 0x10000
                        || (*tok).ucs_char < 0x110000
                    {
                        if utf8::is_high_surrogate((*tok).ucs_char) {
                            (*tok).high_surrogate = (*tok).ucs_char;
                            (*tok).ucs_char = 0;
                            set_state(tok, JSON_TOKENER_STATE_ESCAPE_UNICODE_NEED_ESCAPE);
                            break;
                        } else if utf8::is_low_surrogate((*tok).ucs_char) {
                            if !append_replacement_char_checked(tok, (*tok).pb) {
                                break;
                            }
                        } else if !append_codepoint_checked(tok, (*tok).pb, (*tok).ucs_char) {
                            break;
                        }
                    } else if !append_replacement_char_checked(tok, (*tok).pb) {
                        break;
                    }

                    set_state(tok, saved_state(tok));
                }
                JSON_TOKENER_STATE_ESCAPE_UNICODE_NEED_ESCAPE => {
                    if c == 0 || c != b'\\' {
                        if !append_replacement_char_checked(tok, (*tok).pb) {
                            break;
                        }
                        (*tok).high_surrogate = 0;
                        (*tok).ucs_char = 0;
                        (*tok).st_pos = 0;
                        set_state(tok, saved_state(tok));
                        continue;
                    }
                    set_state(tok, JSON_TOKENER_STATE_ESCAPE_UNICODE_NEED_U);
                }
                JSON_TOKENER_STATE_ESCAPE_UNICODE_NEED_U => {
                    if c == 0 || c != b'u' {
                        if !append_replacement_char_checked(tok, (*tok).pb) {
                            break;
                        }
                        (*tok).high_surrogate = 0;
                        (*tok).ucs_char = 0;
                        (*tok).st_pos = 0;
                        set_state(tok, JSON_TOKENER_STATE_STRING_ESCAPE);
                        continue;
                    }
                    set_state(tok, JSON_TOKENER_STATE_ESCAPE_UNICODE);
                }
                JSON_TOKENER_STATE_BOOLEAN => {
                    if !printbuf_append_byte_checked(tok, (*tok).pb, c) {
                        break;
                    }

                    let size_true = min((*tok).st_pos + 1, JSON_TRUE_STR.len() as c_int) as usize;
                    let size_false = min((*tok).st_pos + 1, JSON_FALSE_STR.len() as c_int) as usize;
                    if ascii_prefix_matches(tok, JSON_TRUE_STR, size_true) {
                        if (*tok).st_pos == JSON_TRUE_STR.len() as c_int {
                            let current = object::json_object_new_boolean_impl(1);
                            if current.is_null() {
                                (*tok).err = JSON_TOKENER_ERROR_MEMORY;
                                break;
                            }
                            set_current(tok, current);
                            set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                            set_state(tok, JSON_TOKENER_STATE_EATWS);
                            continue;
                        }
                    } else if ascii_prefix_matches(tok, JSON_FALSE_STR, size_false) {
                        if (*tok).st_pos == JSON_FALSE_STR.len() as c_int {
                            let current = object::json_object_new_boolean_impl(0);
                            if current.is_null() {
                                (*tok).err = JSON_TOKENER_ERROR_MEMORY;
                                break;
                            }
                            set_current(tok, current);
                            set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                            set_state(tok, JSON_TOKENER_STATE_EATWS);
                            continue;
                        }
                    } else {
                        (*tok).err = JSON_TOKENER_ERROR_PARSE_BOOLEAN;
                        break;
                    }
                    (*tok).st_pos += 1;
                }
                JSON_TOKENER_STATE_NUMBER => {
                    let case_start = cursor;
                    let mut case_len = 0_i32;
                    let mut is_exponent = false;
                    let mut neg_sign_ok = true;
                    let mut pos_sign_ok = false;

                    if printbuf_len((*tok).pb) > 0 {
                        let saved = buffer_bytes((*tok).pb, printbuf_len((*tok).pb) as usize);
                        if let Some(e_idx) =
                            saved.iter().position(|byte| *byte == b'e' || *byte == b'E')
                        {
                            is_exponent = true;
                            pos_sign_ok = true;
                            neg_sign_ok = true;
                            if e_idx + 1 != saved.len() {
                                pos_sign_ok = false;
                                neg_sign_ok = false;
                            }
                        }
                    }

                    while c != 0
                        && (c.is_ascii_digit()
                            || (!is_exponent && matches!(c, b'e' | b'E'))
                            || (neg_sign_ok && c == b'-')
                            || (pos_sign_ok && c == b'+')
                            || ((*tok).is_double == 0 && c == b'.'))
                    {
                        pos_sign_ok = false;
                        neg_sign_ok = false;
                        case_len += 1;

                        match c {
                            b'.' => {
                                (*tok).is_double = 1;
                                pos_sign_ok = true;
                                neg_sign_ok = true;
                            }
                            b'e' | b'E' => {
                                is_exponent = true;
                                (*tok).is_double = 1;
                                pos_sign_ok = true;
                                neg_sign_ok = true;
                            }
                            _ => {}
                        }

                        if advance_char(&mut cursor, tok, c) == 0
                            || !peek_char(tok, cursor, len, &mut c, &mut n_bytes)
                        {
                            let _ = printbuf_append_checked(
                                tok,
                                (*tok).pb,
                                case_start.cast(),
                                case_len,
                            );
                            break 'parse;
                        }
                    }

                    if (*tok).depth > 0
                        && c != b','
                        && c != b']'
                        && c != b'}'
                        && c != b'/'
                        && c != b'I'
                        && c != b'i'
                        && !is_ws_char(c)
                    {
                        (*tok).err = JSON_TOKENER_ERROR_PARSE_NUMBER;
                        break;
                    }

                    if case_len > 0
                        && !printbuf_append_checked(tok, (*tok).pb, case_start.cast(), case_len)
                    {
                        break;
                    }

                    if *(*(*tok).pb).buf.cast::<u8>() == b'-'
                        && case_len <= 1
                        && matches!(c, b'i' | b'I')
                    {
                        set_state(tok, JSON_TOKENER_STATE_INF);
                        (*tok).st_pos = 0;
                        continue;
                    }

                    if (*tok).is_double != 0 && ((*tok).flags & JSON_TOKENER_STRICT) == 0 {
                        while printbuf_len((*tok).pb) > 1 {
                            let last_idx = printbuf_len((*tok).pb) as usize - 1;
                            let last_char = *(*(*tok).pb).buf.add(last_idx).cast::<u8>();
                            if !matches!(last_char, b'e' | b'E' | b'-' | b'+') {
                                break;
                            }
                            *(*(*tok).pb).buf.add(last_idx) = 0;
                            (*(*tok).pb).bpos -= 1;
                        }
                    }

                    if !handle_completed_number(tok) {
                        break;
                    }
                    continue;
                }
                JSON_TOKENER_STATE_ARRAY_AFTER_SEP | JSON_TOKENER_STATE_ARRAY => {
                    if c == b']' {
                        let _ = object::json_object_array_shrink_impl(current(tok), 0);
                        if state(tok) == JSON_TOKENER_STATE_ARRAY_AFTER_SEP
                            && ((*tok).flags & JSON_TOKENER_STRICT) != 0
                        {
                            (*tok).err = JSON_TOKENER_ERROR_PARSE_UNEXPECTED;
                            break;
                        }
                        set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                    } else {
                        if (*tok).depth >= (*tok).max_depth - 1 {
                            (*tok).err = JSON_TOKENER_ERROR_DEPTH;
                            break;
                        }
                        set_state(tok, JSON_TOKENER_STATE_ARRAY_ADD);
                        (*tok).depth += 1;
                        json_tokener_reset_level(tok, (*tok).depth);
                        continue;
                    }
                }
                JSON_TOKENER_STATE_ARRAY_ADD => {
                    if object::json_object_array_add_impl(current(tok), obj) != 0 {
                        (*tok).err = JSON_TOKENER_ERROR_MEMORY;
                        break;
                    }
                    set_saved_state(tok, JSON_TOKENER_STATE_ARRAY_SEP);
                    set_state(tok, JSON_TOKENER_STATE_EATWS);
                    continue;
                }
                JSON_TOKENER_STATE_ARRAY_SEP => {
                    if c == b']' {
                        let _ = object::json_object_array_shrink_impl(current(tok), 0);
                        set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                    } else if c == b',' {
                        set_saved_state(tok, JSON_TOKENER_STATE_ARRAY_AFTER_SEP);
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                    } else {
                        (*tok).err = JSON_TOKENER_ERROR_PARSE_ARRAY;
                        break;
                    }
                }
                JSON_TOKENER_STATE_OBJECT_FIELD_START
                | JSON_TOKENER_STATE_OBJECT_FIELD_START_AFTER_SEP => {
                    if c == b'}' {
                        if state(tok) == JSON_TOKENER_STATE_OBJECT_FIELD_START_AFTER_SEP
                            && ((*tok).flags & JSON_TOKENER_STRICT) != 0
                        {
                            (*tok).err = JSON_TOKENER_ERROR_PARSE_UNEXPECTED;
                            break;
                        }
                        set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                    } else if matches!(c, b'"' | b'\'') {
                        (*tok).quote_char = c as c_char;
                        printbuf_impl::printbuf_reset_impl((*tok).pb);
                        set_state(tok, JSON_TOKENER_STATE_OBJECT_FIELD);
                    } else {
                        (*tok).err = JSON_TOKENER_ERROR_PARSE_OBJECT_KEY_NAME;
                        break;
                    }
                }
                JSON_TOKENER_STATE_OBJECT_FIELD => {
                    let case_start = cursor;
                    loop {
                        if c == (*tok).quote_char as u8 {
                            if !printbuf_append_checked(
                                tok,
                                (*tok).pb,
                                case_start.cast(),
                                cursor.offset_from(case_start) as c_int,
                            ) {
                                break;
                            }
                            let copied = strdup((*(*tok).pb).buf);
                            if copied.is_null() {
                                (*tok).err = JSON_TOKENER_ERROR_MEMORY;
                                break;
                            }
                            set_obj_field_name(tok, copied);
                            set_saved_state(tok, JSON_TOKENER_STATE_OBJECT_FIELD_END);
                            set_state(tok, JSON_TOKENER_STATE_EATWS);
                            break;
                        } else if c == b'\\' {
                            if !printbuf_append_checked(
                                tok,
                                (*tok).pb,
                                case_start.cast(),
                                cursor.offset_from(case_start) as c_int,
                            ) {
                                break;
                            }
                            set_saved_state(tok, JSON_TOKENER_STATE_OBJECT_FIELD);
                            set_state(tok, JSON_TOKENER_STATE_STRING_ESCAPE);
                            break;
                        }
                        if advance_char(&mut cursor, tok, c) == 0
                            || !peek_char(tok, cursor, len, &mut c, &mut n_bytes)
                        {
                            let _ = printbuf_append_checked(
                                tok,
                                (*tok).pb,
                                case_start.cast(),
                                cursor.offset_from(case_start) as c_int,
                            );
                            break 'parse;
                        }
                    }
                }
                JSON_TOKENER_STATE_OBJECT_FIELD_END => {
                    if c == b':' {
                        set_saved_state(tok, JSON_TOKENER_STATE_OBJECT_VALUE);
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                    } else {
                        (*tok).err = JSON_TOKENER_ERROR_PARSE_OBJECT_KEY_SEP;
                        break;
                    }
                }
                JSON_TOKENER_STATE_OBJECT_VALUE => {
                    if (*tok).depth >= (*tok).max_depth - 1 {
                        (*tok).err = JSON_TOKENER_ERROR_DEPTH;
                        break;
                    }
                    set_state(tok, JSON_TOKENER_STATE_OBJECT_VALUE_ADD);
                    (*tok).depth += 1;
                    json_tokener_reset_level(tok, (*tok).depth);
                    continue;
                }
                JSON_TOKENER_STATE_OBJECT_VALUE_ADD => {
                    let parent_current = current(tok);
                    let key = obj_field_name(tok);
                    let _ = object::json_object_object_add_impl(parent_current, key, obj);
                    free(key.cast());
                    set_obj_field_name(tok, ptr::null_mut());
                    set_saved_state(tok, JSON_TOKENER_STATE_OBJECT_SEP);
                    set_state(tok, JSON_TOKENER_STATE_EATWS);
                    continue;
                }
                JSON_TOKENER_STATE_OBJECT_SEP => {
                    if c == b'}' {
                        set_saved_state(tok, JSON_TOKENER_STATE_FINISH);
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                    } else if c == b',' {
                        set_saved_state(tok, JSON_TOKENER_STATE_OBJECT_FIELD_START_AFTER_SEP);
                        set_state(tok, JSON_TOKENER_STATE_EATWS);
                    } else {
                        (*tok).err = JSON_TOKENER_ERROR_PARSE_OBJECT_VALUE_SEP;
                        break;
                    }
                }
                _ => unreachable!(),
            }

            if (*tok).err != JSON_TOKENER_SUCCESS {
                break;
            }
            break;
        }

        if (*tok).err != JSON_TOKENER_SUCCESS {
            break;
        }

        if advance_char(&mut cursor, tok, c) == 0 {
            break;
        }
    }

    if ((*tok).flags & JSON_TOKENER_VALIDATE_UTF8) != 0 && n_bytes != 0 {
        (*tok).err = JSON_TOKENER_ERROR_PARSE_UTF8_STRING;
    }
    if c != 0
        && state(tok) == JSON_TOKENER_STATE_FINISH
        && (*tok).depth == 0
        && ((*tok).flags & (JSON_TOKENER_STRICT | JSON_TOKENER_ALLOW_TRAILING_CHARS))
            == JSON_TOKENER_STRICT
    {
        (*tok).err = JSON_TOKENER_ERROR_PARSE_UNEXPECTED;
    }
    if c == 0
        && state(tok) != JSON_TOKENER_STATE_FINISH
        && saved_state(tok) != JSON_TOKENER_STATE_FINISH
    {
        (*tok).err = JSON_TOKENER_ERROR_PARSE_EOF;
    }

    if (*tok).err == JSON_TOKENER_SUCCESS {
        let ret = object::json_object_get_impl(current(tok));
        let mut ii = (*tok).depth;
        loop {
            json_tokener_reset_level(tok, ii);
            if ii == 0 {
                break;
            }
            ii -= 1;
        }
        return ret;
    }

    ptr::null_mut()
}
