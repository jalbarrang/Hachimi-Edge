//! Log level constants and `hlog_*` macros.

/// Log levels matching the host `log` vtable slot.
pub mod log_level {
    pub const ERROR: i32 = 1;
    pub const WARN: i32 = 2;
    pub const INFO: i32 = 3;
    pub const DEBUG: i32 = 4;
    pub const TRACE: i32 = 5;
}

/// Log through the host vtable. Optional `target: "name"` overrides the default crate target.
#[macro_export]
macro_rules! hlog {
    (target: $target:literal, $level:expr, $($arg:tt)*) => {{
        let msg = format!($($arg)*);
        let msg_c = std::ffi::CString::new(msg).unwrap_or_default();
        let target = std::ffi::CStr::from_bytes_with_nul($target.as_bytes()).unwrap_or_default();
        #[allow(unused_unsafe)]
        // SAFETY: Plugin FFI interop with Hachimi vtable
        unsafe {
            ($crate::vt().log)($level, target.as_ptr(), msg_c.as_ptr());
        }
    }};
    ($level:expr, $($arg:tt)*) => {
        $crate::hlog!(target: "plugin", $level, $($arg)*)
    };
}

#[macro_export]
macro_rules! hlog_info {
    (target: $target:literal, $($arg:tt)*) => {
        $crate::hlog!(target: $target, $crate::log_level::INFO, $($arg)*)
    };
    ($($arg:tt)*) => { $crate::hlog!($crate::log_level::INFO, $($arg)*) };
}

#[macro_export]
macro_rules! hlog_error {
    (target: $target:literal, $($arg:tt)*) => {
        $crate::hlog!(target: $target, $crate::log_level::ERROR, $($arg)*)
    };
    ($($arg:tt)*) => { $crate::hlog!($crate::log_level::ERROR, $($arg)*) };
}

#[macro_export]
macro_rules! hlog_warn {
    (target: $target:literal, $($arg:tt)*) => {
        $crate::hlog!(target: $target, $crate::log_level::WARN, $($arg)*)
    };
    ($($arg:tt)*) => { $crate::hlog!($crate::log_level::WARN, $($arg)*) };
}

#[macro_export]
macro_rules! hlog_debug {
    (target: $target:literal, $($arg:tt)*) => {
        $crate::hlog!(target: $target, $crate::log_level::DEBUG, $($arg)*)
    };
    ($($arg:tt)*) => { $crate::hlog!($crate::log_level::DEBUG, $($arg)*) };
}

#[macro_export]
macro_rules! hlog_trace {
    (target: $target:literal, $($arg:tt)*) => {
        $crate::hlog!(target: $target, $crate::log_level::TRACE, $($arg)*)
    };
    ($($arg:tt)*) => { $crate::hlog!($crate::log_level::TRACE, $($arg)*) };
}
