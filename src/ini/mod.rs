#![allow(clippy::module_inception)]

mod error;
mod ini;
mod options;
mod util;

#[cfg(test)]
mod tests;

pub(crate) use ini::dyn_config_from_ini;
pub(crate) use util::{write_ini_key, write_ini_section, DisplayIni};

pub use error::*;
pub use options::*;
