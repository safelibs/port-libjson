use crate::abi::*;

pub const JSON_C_VERSION_BYTES: &[u8; 5] = b"0.17\0";
pub const JSON_C_VERSION_NUM: c_int = 0x0000_1100;

pub(crate) unsafe fn json_c_version_impl() -> *const c_char
{
    JSON_C_VERSION_BYTES.as_ptr().cast()
}

pub(crate) unsafe fn json_c_version_num_impl() -> c_int
{
    JSON_C_VERSION_NUM
}
