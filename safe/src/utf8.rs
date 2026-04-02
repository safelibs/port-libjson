use crate::abi::*;
use crate::printbuf as printbuf_impl;

pub(crate) const REPLACEMENT_CHAR: [u8; 3] = [0xEF, 0xBF, 0xBD];

pub(crate) fn validate_utf8(c: c_char, n_bytes: &mut c_uint) -> bool {
    let chr = c as u8;
    if *n_bytes == 0 {
        if chr >= 0x80 {
            if (chr & 0xe0) == 0xc0 {
                *n_bytes = 1;
            } else if (chr & 0xf0) == 0xe0 {
                *n_bytes = 2;
            } else if (chr & 0xf8) == 0xf0 {
                *n_bytes = 3;
            } else {
                return false;
            }
        }
    } else if (chr & 0xC0) != 0x80 {
        return false;
    } else {
        *n_bytes -= 1;
    }
    true
}

pub(crate) fn is_hex_char(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

pub(crate) fn hex_digit(c: u8) -> u32 {
    if c <= b'9' {
        (c - b'0') as u32
    } else {
        ((c & 7) + 9) as u32
    }
}

pub(crate) fn is_high_surrogate(ucs: c_uint) -> bool {
    (ucs & 0xFC00) == 0xD800
}

pub(crate) fn is_low_surrogate(ucs: c_uint) -> bool {
    (ucs & 0xFC00) == 0xDC00
}

pub(crate) fn decode_surrogate_pair(high: c_uint, low: c_uint) -> c_uint {
    ((high & 0x3FF) << 10) + (low & 0x3FF) + 0x10000
}

pub(crate) unsafe fn append_replacement_char(pb: *mut crate::abi::printbuf) -> c_int {
    printbuf_impl::printbuf_memappend_impl(
        pb,
        REPLACEMENT_CHAR.as_ptr().cast(),
        REPLACEMENT_CHAR.len() as c_int,
    )
}

pub(crate) unsafe fn append_codepoint(pb: *mut crate::abi::printbuf, ucs: c_uint) -> c_int {
    let mut buf = [0_u8; 4];
    let len = if ucs < 0x80 {
        buf[0] = ucs as u8;
        1
    } else if ucs < 0x800 {
        buf[0] = 0xc0 | ((ucs >> 6) as u8);
        buf[1] = 0x80 | ((ucs & 0x3f) as u8);
        2
    } else if ucs < 0x10000 {
        buf[0] = 0xe0 | ((ucs >> 12) as u8);
        buf[1] = 0x80 | (((ucs >> 6) & 0x3f) as u8);
        buf[2] = 0x80 | ((ucs & 0x3f) as u8);
        3
    } else if ucs < 0x110000 {
        buf[0] = 0xf0 | (((ucs >> 18) & 0x07) as u8);
        buf[1] = 0x80 | (((ucs >> 12) & 0x3f) as u8);
        buf[2] = 0x80 | (((ucs >> 6) & 0x3f) as u8);
        buf[3] = 0x80 | ((ucs & 0x3f) as u8);
        4
    } else {
        return append_replacement_char(pb);
    };

    printbuf_impl::printbuf_memappend_impl(pb, buf.as_ptr().cast(), len)
}
