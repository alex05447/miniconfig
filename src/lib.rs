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

#[cfg(feature = "ini")]
mod ini;

mod util;
mod value;

pub use value::{Value, ValueType};

#[cfg(feature = "bin")]
pub use bin_config::*;

#[cfg(feature = "dyn")]
pub use dyn_config::*;

#[cfg(feature = "lua")]
pub use lua_config::*;

#[cfg(feature = "ini")]
pub use ini::*;
