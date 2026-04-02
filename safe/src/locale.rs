use crate::abi::*;
use std::ptr;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn strdup(s: *const c_char) -> *mut c_char;
}

const LC_NUMERIC: c_int = 1;
const C_LOCALE: &[u8; 2] = b"C\0";

pub(crate) struct NumericLocaleGuard {
    previous: *mut c_char,
}

impl NumericLocaleGuard {
    pub(crate) unsafe fn enter() -> Option<Self> {
        let current = setlocale(LC_NUMERIC, ptr::null());
        let previous = if current.is_null() {
            ptr::null_mut()
        } else {
            let copy = strdup(current);
            if copy.is_null() {
                return None;
            }
            copy
        };

        let _ = setlocale(LC_NUMERIC, C_LOCALE.as_ptr().cast());
        Some(Self { previous })
    }
}

impl Drop for NumericLocaleGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.previous.is_null() {
                let _ = setlocale(LC_NUMERIC, self.previous);
                free(self.previous.cast());
            }
        }
    }
}
