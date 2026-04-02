use crate::abi::*;
use crate::{errors, printbuf as printbuf_impl, serializer, strerror, tokener};
use std::ffi::CStr;
use std::ptr;

extern "C" {
    fn __errno_location() -> *mut c_int;
    fn close(fd: c_int) -> c_int;
    fn open(pathname: *const c_char, flags: c_int, ...) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t;
}

const JSON_FILE_BUF_SIZE: usize = 4096;
const JSON_TOKENER_DEFAULT_DEPTH: c_int = 32;
const JSON_C_TO_STRING_PLAIN: c_int = 0;

const O_RDONLY: c_int = 0;
const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

unsafe fn errno_value() -> c_int {
    *__errno_location()
}

unsafe fn set_errno(value: c_int) {
    *__errno_location() = value;
}

unsafe fn errno_text(errno_in: c_int) -> String {
    let ptr = strerror::_json_c_strerror_impl(errno_in);
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

unsafe fn printbuf_length(pb: *mut printbuf) -> c_int {
    (*pb).bpos
}

unsafe fn json_object_to_fd_inner(
    fd: c_int,
    obj: *mut json_object,
    flags: c_int,
    filename: *const c_char,
) -> c_int {
    let display_name = if filename.is_null() {
        "(fd)".to_string()
    } else {
        CStr::from_ptr(filename).to_string_lossy().into_owned()
    };

    let json_str = serializer::json_object_to_json_string_ext_impl(obj, flags);
    if json_str.is_null() {
        return -1;
    }

    let bytes = CStr::from_ptr(json_str).to_bytes();
    let mut written = 0usize;
    while written < bytes.len() {
        let rc = write(
            fd,
            bytes[written..].as_ptr().cast(),
            bytes.len().saturating_sub(written),
        );
        if rc < 0 {
            errors::set_last_err_fmt(format_args!(
                "json_object_to_fd: error writing file {}: {}\n",
                display_name,
                errno_text(errno_value())
            ));
            return -1;
        }
        written += rc as usize;
    }

    0
}

pub(crate) unsafe fn json_object_from_fd_impl(fd: c_int) -> *mut json_object {
    json_object_from_fd_ex_impl(fd, -1)
}

pub(crate) unsafe fn json_object_from_fd_ex_impl(fd: c_int, in_depth: c_int) -> *mut json_object {
    let pb = printbuf_impl::printbuf_new_impl();
    if pb.is_null() {
        errors::set_last_err_fmt(format_args!(
            "json_object_from_fd_ex: printbuf_new failed\n"
        ));
        return ptr::null_mut();
    }

    let depth = if in_depth != -1 {
        in_depth
    } else {
        JSON_TOKENER_DEFAULT_DEPTH
    };
    let tok = tokener::json_tokener_new_ex_impl(depth);
    if tok.is_null() {
        let err = errno_value();
        errors::set_last_err_fmt(format_args!(
            "json_object_from_fd_ex: unable to allocate json_tokener(depth={}): {}\n",
            depth,
            errno_text(err)
        ));
        printbuf_impl::printbuf_free_impl(pb);
        return ptr::null_mut();
    }

    let mut buf = [0_u8; JSON_FILE_BUF_SIZE];
    loop {
        let rc = read(fd, buf.as_mut_ptr().cast(), buf.len());
        if rc > 0 {
            if printbuf_impl::printbuf_memappend_impl(pb, buf.as_ptr().cast(), rc as c_int) < 0 {
                errors::set_last_err_fmt(format_args!(
                    "json_object_from_fd_ex: failed to printbuf_memappend after reading {}+{} bytes: {}",
                    printbuf_length(pb),
                    rc,
                    errno_text(errno_value())
                ));
                tokener::json_tokener_free_impl(tok);
                printbuf_impl::printbuf_free_impl(pb);
                return ptr::null_mut();
            }
            continue;
        }

        if rc < 0 {
            errors::set_last_err_fmt(format_args!(
                "json_object_from_fd_ex: error reading fd {}: {}\n",
                fd,
                errno_text(errno_value())
            ));
            tokener::json_tokener_free_impl(tok);
            printbuf_impl::printbuf_free_impl(pb);
            return ptr::null_mut();
        }

        break;
    }

    let obj = tokener::json_tokener_parse_ex_impl(tok, (*pb).buf, printbuf_length(pb));
    if obj.is_null() {
        errors::set_last_err_fmt(format_args!(
            "json_tokener_parse_ex failed: {}\n",
            CStr::from_ptr(tokener::json_tokener_error_desc_impl(
                tokener::json_tokener_get_error_impl(tok)
            ))
            .to_string_lossy()
        ));
    }

    tokener::json_tokener_free_impl(tok);
    printbuf_impl::printbuf_free_impl(pb);
    obj
}

pub(crate) unsafe fn json_object_from_file_impl(filename: *const c_char) -> *mut json_object {
    let fd = open(filename, O_RDONLY);
    if fd < 0 {
        let display_name = if filename.is_null() {
            "(null)".to_string()
        } else {
            CStr::from_ptr(filename).to_string_lossy().into_owned()
        };
        errors::set_last_err_fmt(format_args!(
            "json_object_from_file: error opening file {}: {}\n",
            display_name,
            errno_text(errno_value())
        ));
        return ptr::null_mut();
    }

    let obj = json_object_from_fd_impl(fd);
    close(fd);
    obj
}

pub(crate) unsafe fn json_object_to_file_impl(
    filename: *const c_char,
    obj: *mut json_object,
) -> c_int {
    json_object_to_file_ext_impl(filename, obj, JSON_C_TO_STRING_PLAIN)
}

pub(crate) unsafe fn json_object_to_file_ext_impl(
    filename: *const c_char,
    obj: *mut json_object,
    flags: c_int,
) -> c_int {
    if obj.is_null() {
        errors::set_last_err_fmt(format_args!("json_object_to_file_ext: object is null\n"));
        return -1;
    }

    let fd = open(filename, O_WRONLY | O_TRUNC | O_CREAT, 0o644);
    if fd < 0 {
        let display_name = if filename.is_null() {
            "(null)".to_string()
        } else {
            CStr::from_ptr(filename).to_string_lossy().into_owned()
        };
        errors::set_last_err_fmt(format_args!(
            "json_object_to_file_ext: error opening file {}: {}\n",
            display_name,
            errno_text(errno_value())
        ));
        return -1;
    }

    let rc = json_object_to_fd_inner(fd, obj, flags, filename);
    let saved_errno = errno_value();
    close(fd);
    set_errno(saved_errno);
    rc
}

pub(crate) unsafe fn json_object_to_fd_impl(
    fd: c_int,
    obj: *mut json_object,
    flags: c_int,
) -> c_int {
    if obj.is_null() {
        errors::set_last_err_fmt(format_args!("json_object_to_fd: object is null\n"));
        return -1;
    }

    json_object_to_fd_inner(fd, obj, flags, ptr::null())
}
