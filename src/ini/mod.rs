#![allow(clippy::module_inception)]

mod error;
mod ini;
mod options;

#[cfg(test)]
mod tests;

pub(crate) use ini::dyn_config_from_ini;

pub use error::*;
pub use options::*;
