use crate::abi::*;
use core::sync::atomic::{AtomicI32, Ordering};

static DEBUG_ENABLED: AtomicI32 = AtomicI32::new(0);
static SYSLOG_ENABLED: AtomicI32 = AtomicI32::new(0);

pub(crate) fn mc_set_debug_impl(debug: c_int)
{
    DEBUG_ENABLED.store(debug, Ordering::Relaxed);
}

pub(crate) fn mc_get_debug_impl() -> c_int
{
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

pub(crate) fn mc_set_syslog_impl(syslog: c_int)
{
    SYSLOG_ENABLED.store(syslog, Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "C" fn __json_c_get_syslog_enabled() -> c_int
{
    SYSLOG_ENABLED.load(Ordering::Relaxed)
}
