#![allow(clippy::module_inception)]

mod error;
mod ini;
mod options;
mod util;

#[cfg(all(test, feature = "dyn"))]
mod tests;

#[cfg(feature = "dyn")]
pub(crate) use ini::dyn_config_from_ini;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) use util::{write_ini_key, write_ini_section, DisplayIni};

pub use error::*;
pub use ini::{parse_ini, IniConfig, IniValue};
pub use options::*;
