#[cfg(feature = "bin")]
mod bin_config;

#[cfg(feature = "dyn")]
mod dyn_config;

#[cfg(feature = "lua")]
mod lua_config;

#[cfg(feature = "ini")]
mod ini;

#[cfg(any(
    feature = "bin",
    feature = "dyn",
    feature = "ini",
    feature = "lua",
    feature = "str_hash"
))]
#[macro_use]
mod util;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
pub(crate) use util::debug_unreachable_impl;

mod value;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
mod error;

pub use value::*;

#[cfg(all(test, any(feature = "bin", feature = "dyn", feature = "lua")))]
pub(crate) use util::cmp_f64;

#[cfg(feature = "bin")]
pub use bin_config::*;

#[cfg(feature = "dyn")]
pub use dyn_config::*;

#[cfg(feature = "lua")]
pub use lua_config::*;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub use error::*;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
pub use util::*;

#[cfg(feature = "ini")]
pub use ini::*;

#[cfg(all(feature = "bin", feature = "str_hash"))]
pub use util::StringAndHash;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
pub use ministr::{NonEmptyStr, NonEmptyString};
