#![allow(clippy::module_inception)]

mod ini;
mod error;
mod options;

#[cfg(test)]
mod tests;

pub(crate) use ini::dyn_config_from_ini;

pub use options::*;
pub use error::*;