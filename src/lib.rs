#![allow(clippy::cognitive_complexity)]
#![allow(clippy::len_without_is_empty)]
#![allow(clippy::new_without_default)]
#![allow(clippy::approx_constant)]

#[cfg(feature = "ini")]
extern crate bitflags;

#[cfg(feature = "bin")]
mod bin_config;

#[cfg(feature = "dyn")]
mod dyn_config;

#[cfg(feature = "lua")]
mod lua_config;

#[cfg(all(feature = "dyn", feature = "ini"))]
mod ini;

mod util;
mod value;

pub(crate) use util::write_char;

#[cfg(feature = "lua")]
pub(crate) use lua_config::DisplayLua;

pub(crate) use value::{value_type_from_u32, value_type_to_u32};

pub use value::{Value, ValueType};

#[cfg(feature = "bin")]
pub use bin_config::*;

#[cfg(feature = "dyn")]
pub use dyn_config::*;

#[cfg(feature = "lua")]
pub use lua_config::*;

#[cfg(all(feature = "dyn", feature = "ini"))]
pub use ini::*;
