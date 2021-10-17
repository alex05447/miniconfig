#[cfg(any(feature = "bin", feature = "dyn", feature = "ini", feature = "lua"))]
mod display;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
mod display_lua;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
mod config_path;

#[cfg(any(feature = "bin", feature = "dyn", feature = "ini", feature = "lua"))]
pub(crate) use display::*;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) use display_lua::*;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
pub use config_path::*;

#[cfg(all(test, any(feature = "bin", feature = "dyn", feature = "lua")))]
pub(crate) fn cmp_f64(l: f64, r: f64) -> bool {
    (l - r).abs() < 0.000_001
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
pub(crate) fn debug_unreachable_impl(msg: &'static str) -> ! {
    if cfg!(debug_assertions) {
        unreachable!(msg)
    } else {
        unsafe { std::hint::unreachable_unchecked() }
    }
}

/// `unreachable!()` in debug to `panic!()` and catch the logic error,
/// `std::hint::unreachable_unchecked()` in release to avoid unnecessary `panic!()` codegen.
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
#[macro_export]
macro_rules! debug_unreachable {
    () => {{
        $crate::debug_unreachable_impl("internal error: entered unreachable code")
    }};
    ($msg:expr $(,)?) => {{
        $crate::debug_unreachable_impl($msg)
    }};
}

/// A helper trait to perfrom unwrapping of `Option`'s / `Result`'s
/// which are known to be `Some` / `Ok`.
/// Unlike the (currently unstable) `.unwrap_unchecked()` method on `Option`'s / `Result`'s,
/// this uses `unreachable!()` in debug configuration and `std::hint::unreachable_unchecked()` in release configuration.
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
pub(crate) trait UnwrapUnchecked<T> {
    fn unwrap_unchecked(self, msg: &'static str) -> T;
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<T> UnwrapUnchecked<T> for Option<T> {
    fn unwrap_unchecked(self, msg: &'static str) -> T {
        if let Some(val) = self {
            val
        } else {
            debug_unreachable!(msg)
        }
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<T, E> UnwrapUnchecked<T> for Result<T, E> {
    fn unwrap_unchecked(self, msg: &'static str) -> T {
        if let Ok(val) = self {
            val
        } else {
            debug_unreachable!(msg)
        }
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
pub(crate) fn unwrap_unchecked<U: UnwrapUnchecked<T>, T>(
    option_or_result: U,
    msg: &'static str,
) -> T {
    option_or_result.unwrap_unchecked(msg)
}
