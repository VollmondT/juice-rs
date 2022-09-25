use std::ffi::CStr;

use lazy_static::lazy_static;
use libjuice_sys as sys;

lazy_static! {
    static ref INIT_LOGGING: () = {
        let level = match log::max_level() {
            log::LevelFilter::Off => sys::juice_log_level_JUICE_LOG_LEVEL_NONE,
            log::LevelFilter::Error => sys::juice_log_level_JUICE_LOG_LEVEL_ERROR,
            log::LevelFilter::Warn => sys::juice_log_level_JUICE_LOG_LEVEL_WARN,
            log::LevelFilter::Info => sys::juice_log_level_JUICE_LOG_LEVEL_INFO,
            log::LevelFilter::Debug => sys::juice_log_level_JUICE_LOG_LEVEL_DEBUG,
            log::LevelFilter::Trace => sys::juice_log_level_JUICE_LOG_LEVEL_VERBOSE,
        };
        unsafe {
            sys::juice_set_log_handler(Some(log_callback));
            sys::juice_set_log_level(level)
        };
    };
}

unsafe extern "C" fn log_callback(
    level: sys::juice_log_level_t,
    message: *const std::os::raw::c_char,
) {
    let message = CStr::from_ptr(message).to_string_lossy();
    match level {
        sys::juice_log_level_JUICE_LOG_LEVEL_NONE => (),
        sys::juice_log_level_JUICE_LOG_LEVEL_FATAL => log::error!("{}", message),
        sys::juice_log_level_JUICE_LOG_LEVEL_ERROR => log::error!("{}", message),
        sys::juice_log_level_JUICE_LOG_LEVEL_WARN => log::warn!("{}", message),
        sys::juice_log_level_JUICE_LOG_LEVEL_INFO => log::info!("{}", message),
        sys::juice_log_level_JUICE_LOG_LEVEL_DEBUG => log::debug!("{}", message),
        sys::juice_log_level_JUICE_LOG_LEVEL_VERBOSE => log::trace!("{}", message),
        _ => unreachable!(),
    }
}

/// Init logger singleton
#[allow(clippy::no_effect)]
pub(crate) fn ensure_logging() {
    *INIT_LOGGING;
}
