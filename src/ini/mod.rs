mod error;

#[allow(clippy::module_inception)]
mod ini;

mod options;
mod util;

#[cfg(all(test, feature = "dyn"))]
mod tests;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) use util::{write_ini_key, write_ini_section, DisplayIni};

pub use error::*;
pub use ini::{IniConfig, IniParser, IniValue, NonEmptyStr};
pub use options::*;
