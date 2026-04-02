use crate::abi::*;
use std::fs::File;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn json_c_get_random_seed_impl() -> c_int
{
    let mut bytes = [0_u8; 4];
    if File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes)).is_ok()
    {
        return i32::from_ne_bytes(bytes);
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let mut seed = now.as_nanos() as u64;
    seed ^= (std::process::id() as u64) << 32;
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xff51afd7ed558ccd);
    seed ^= seed >> 33;
    seed = seed.wrapping_mul(0xc4ceb9fe1a85ec53);
    seed ^= seed >> 33;
    seed as u32 as c_int
}
