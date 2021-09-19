use {
    crate::*,
    std::{
        borrow::Cow,
        fmt::{Display, Formatter},
    },
};

#[cfg(feature = "bin")]
use crate::bin_config::string_hash_fnv1a;

#[cfg(all(feature = "bin", feature = "str_hash"))]
mod string_and_hash {
    use super::*;

    /// A non-empty string literal and its compile-time hash (created via `str_hash_fnv1a!`).
    /// Used as an optimization for string literals used as binary config table keys to avoid runtime string hashing.
    /// Requires "bin" and "str_hash" features.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct StringAndHash {
        pub string: &'static NonEmptyStr,
        pub hash: u32,
    }

    impl StringAndHash {
        /// Creates a `StringAndHash` from a `string` literal and its precomputed FNV1-a `hash`.
        /// The caller guarantees `hash` is the correct FNV1-a hash of the `string` literal.
        pub fn new(string: &'static NonEmptyStr, hash: u32) -> Self {
            debug_assert!(
                string_hash_fnv1a(string) == hash,
                "string and hash mismatch"
            );
            Self { string, hash }
        }
    }

    impl Display for StringAndHash {
        fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
            write!(f, "\"{}\"", self.string)
        }
    }

    /// Creates a [`binary config`] [`table`] [`key`] and its hash from a non-empty string literal.
    /// This is slightly more efficient then using a [`normal`] string key,
    /// as this avoids runtime string hashing used internally by the binary config [`table`] accessor.
    ///
    /// [`binary config`]: struct.BinConfig.html
    /// [`table`]: struct.BinTable.html
    /// [`key`]: enum.TableKey.html
    /// [`normal`]: enum.TableKey.html#variant.String
    #[macro_export]
    macro_rules! key {
        ($string:literal) => {
            $crate::TableKey::StringAndHash($crate::StringAndHash::new(
                ministr_macro::nestr!($string),
                ministr_macro::str_hash_fnv1a!($string),
            ))
        };
    }
}

#[cfg(all(feature = "bin", feature = "str_hash"))]
pub use string_and_hash::*;

/// A config [`table`] string key.
/// Borrowed, owned, or a compile-time hashed string literal
/// (created via `key!` macro for a binary config table, requires `"bin"` and `"str_hash"` features).
///
/// Valid [`table`] keys are non-empty, but we allow creating keys from empty strings,
/// instead handling the error in the config accessors to make using the code simpler.
///
/// [`table`]: enum.Value.html#variant.Table
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TableKey<'a> {
    /// A normal table string key, borrowed or owned.
    String(Cow<'a, str>),
    /// A string literal + its compile time hash created via the [`key!`] macro.
    #[cfg(all(feature = "bin", feature = "str_hash"))]
    StringAndHash(StringAndHash),
}

#[cfg(feature = "bin")]
impl<'a> TableKey<'a> {
    /// Returns the FNV1-a hash of the key string.
    /// Used by binary config tables (requires `"bin"` feature).
    /// Computed on the fly for non-string-literal keys, or just returns the compile-time hash for
    /// keys created by the [`key!`] macro from a string literal (requires `"str_hash"` feature).
    pub(crate) fn key_hash(&self) -> u32 {
        match self {
            TableKey::String(string) => string_hash_fnv1a(string),

            #[cfg(feature = "str_hash")]
            TableKey::StringAndHash(StringAndHash { hash, .. }) => *hash,
        }
    }
}

impl<'a> AsRef<str> for TableKey<'a> {
    fn as_ref(&self) -> &str {
        match self {
            TableKey::String(string) => string.as_ref(),
            #[cfg(all(feature = "bin", feature = "str_hash"))]
            TableKey::StringAndHash(StringAndHash { string, .. }) => string.as_ref(),
        }
    }
}

impl<'a> Display for TableKey<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            TableKey::String(string) => write!(f, "\"{}\"", string),
            #[cfg(all(feature = "bin", feature = "str_hash"))]
            TableKey::StringAndHash(string_and_hash) => string_and_hash.fmt(f),
        }
    }
}

impl<'a> From<&'a str> for TableKey<'a> {
    fn from(other: &'a str) -> Self {
        Self::String(other.into())
    }
}

impl<'a> From<&'a NonEmptyStr> for TableKey<'a> {
    fn from(other: &'a NonEmptyStr) -> Self {
        Self::String(other.into())
    }
}

impl<'a> From<String> for TableKey<'a> {
    fn from(other: String) -> Self {
        Self::String(other.into())
    }
}

impl<'a> From<NonEmptyString> for TableKey<'a> {
    fn from(other: NonEmptyString) -> Self {
        Self::String(other.into())
    }
}

/// String key (in the [`table`]) or integer index (in the [`array`]) of a config element.
///
/// [`table`]: enum.Value.html#variant.Table
/// [`array`]: enum.Value.html#variant.Array
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ConfigKey<'a> {
    /// A string [`table`] key.
    ///
    /// [`table`]: enum.Value.html#variant.Table
    Table(TableKey<'a>),
    /// A (`0`-based) integer [`array`] index.
    ///
    /// [`array`]: enum.Value.html#variant.Array
    Array(u32),
}

impl<'a> From<&'a str> for ConfigKey<'a> {
    fn from(key: &'a str) -> Self {
        ConfigKey::Table(key.into())
    }
}

impl<'a> From<&'a NonEmptyStr> for ConfigKey<'a> {
    fn from(key: &'a NonEmptyStr) -> Self {
        ConfigKey::Table(key.as_str().into())
    }
}

impl<'a> From<String> for ConfigKey<'a> {
    fn from(key: String) -> Self {
        ConfigKey::Table(key.into())
    }
}

impl<'a> From<NonEmptyString> for ConfigKey<'a> {
    fn from(key: NonEmptyString) -> Self {
        ConfigKey::Table(key.into())
    }
}

impl<'a> From<TableKey<'a>> for ConfigKey<'a> {
    fn from(key: TableKey<'a>) -> Self {
        ConfigKey::Table(key.into())
    }
}

impl<'a> From<u32> for ConfigKey<'a> {
    fn from(index: u32) -> Self {
        ConfigKey::Array(index)
    }
}

impl<'a> Display for ConfigKey<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ConfigKey::Table(key) => key.fmt(f),
            ConfigKey::Array(key) => key.fmt(f),
        }
    }
}

/// Describes the full path to a config element.
/// Empty path means the root table.
/// Used in error reporting by config accessors and parsers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConfigPath<'a>(pub Vec<ConfigKey<'a>>);

impl<'a> ConfigPath<'a> {
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }
}

impl<'a> From<Vec<ConfigKey<'a>>> for ConfigPath<'a> {
    fn from(path: Vec<ConfigKey<'a>>) -> Self {
        Self(path)
    }
}

impl<'a> Display for ConfigPath<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if self.0.is_empty() {
            "<root>".fmt(f)
        } else {
            for (key_index, key) in self.0.iter().enumerate() {
                key.fmt(f)?;

                if !key_index == (self.0.len() - 1) {
                    '/'.fmt(f)?;
                }
            }

            Ok(())
        }
    }
}
