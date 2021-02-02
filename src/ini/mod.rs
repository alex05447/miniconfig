mod error;

#[allow(clippy::module_inception)]
mod ini;

mod options;
mod util;

#[cfg(all(test, feature = "dyn"))]
mod tests;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) use util::{write_ini_array, write_ini_table, write_ini_value, DisplayIni, IniPath};

pub use error::*;
pub use ini::{IniConfig, IniParser, IniValue};
pub use options::*;
