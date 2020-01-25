mod array;
mod array_or_table;
mod config;
mod error;
mod table;
mod util;
mod value;
mod writer;

#[cfg(test)]
mod tests;

pub use array::BinArray;
pub use config::BinConfig;
pub use error::*;
pub use table::BinTable;
pub use writer::BinConfigWriter;
