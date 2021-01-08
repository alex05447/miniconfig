#![allow(clippy::cognitive_complexity)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::new_without_default)]
#![allow(clippy::approx_constant)]

#[cfg(feature = "bin")]
mod bin_config;

#[cfg(feature = "dyn")]
mod dyn_config;

#[cfg(feature = "lua")]
mod lua_config;

#[cfg(feature = "ini")]
mod ini;

#[cfg(not(all(feature = "bin", feature = "str_hash")))]
mod util;

#[cfg(all(feature = "bin", feature = "str_hash"))]
#[macro_use]
mod util;

mod value;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
mod error;

pub use value::{Value, ValueType};

#[cfg(all(test, any(feature = "bin", feature = "dyn", feature = "lua")))]
pub(crate) use util::cmp_f64;

#[cfg(feature = "bin")]
pub use bin_config::*;

#[cfg(feature = "dyn")]
pub use dyn_config::*;

#[cfg(feature = "lua")]
pub use lua_config::*;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub use {error::*, util::*};

#[cfg(feature = "ini")]
pub use ini::*;

#[cfg(all(feature = "bin", feature = "str_hash"))]
pub use util::StringAndHash;
