#[cfg(any(feature = "bin", feature = "dyn", feature = "ini", feature = "lua"))]
use std::fmt::Write;

#[cfg(feature = "lua")]
use std::convert::From;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
};

#[cfg(feature = "bin")]
use super::bin_config::string_hash_fnv1a;

#[cfg(any(feature = "bin", feature = "dyn", feature = "ini", feature = "lua"))]
pub(crate) enum WriteCharError {
    /// General write error (out of memory?).
    WriteError,
    /// Encountered a disallowed escaped character.
    /// Contains the escaped character.
    EscapedCharacter(char),
}

/// Writes the char `c` to the writer `w`.
/// If `escape` is `true`, escapes special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f'),
/// double quotes ('"'),
/// and, if `quoted` is `false`, single quotes ('\'') and spaces (' ');
/// if additionally `ini` is `true` and `quoted` is `false`, also escapes `.ini` special characters
/// ('[', ']', ';', '#', '=', ':').
/// If `escape` is `false` and `c` must be escaped, returns an error.
#[cfg(any(feature = "bin", feature = "dyn", feature = "ini", feature = "lua"))]
pub(crate) fn write_char<W: Write>(
    w: &mut W,
    c: char,
    ini: bool,
    quoted: bool,
    escape: bool,
) -> Result<(), WriteCharError> {
    use WriteCharError::*;

    let escape_char = |c: char| -> &'static str {
        match c {
            '\0' => "\\0",
            '\x07' => "\\x07", // \a
            '\x08' => "\\x08", // \b
            '\t' => "\\t",
            '\n' => "\\n",
            '\r' => "\\r",
            '\x0b' => "\\x0b", // \v
            '\x0c' => "\\x0c", // \f

            '"' => "\\\"",

            '\'' => "\\\'",
            ' ' => "\\ ",

            '[' => "\\[",
            ']' => "\\]",
            ';' => "\\;",
            '#' => "\\#",
            '=' => "\\=",
            ':' => "\\:",

            _ => unreachable!(),
        }
    };

    match c {
        // Don't escape the backslashes and just write them as-is if `escape` is false.
        '\\' if escape => write!(w, "\\\\").map_err(|_| WriteError),

        // It's an error if it's a special character or the double quotes and `escape` is `false`.
        c @ '\0'
        | c @ '\x07'
        | c @ '\x08'
        | c @ '\t'
        | c @ '\n'
        | c @ '\r'
        | c @ '\x0b'
        | c @ '\x0c'
        | c @ '"' => {
            if escape {
                w.write_str(escape_char(c)).map_err(|_| WriteError)
            } else {
                Err(EscapedCharacter(c))
            }
        }

        // Don't escape the single quotes and spaces in quoted strings.
        // Else it's an error if `escape` is `false`.
        c @ '\'' | c @ ' ' if !quoted => {
            if escape {
                w.write_str(escape_char(c)).map_err(|_| WriteError)
            } else {
                Err(EscapedCharacter(c))
            }
        }

        // Don't escape the `.ini` special characters in quoted `.ini` strings.
        // Else it's an error if `escape` is `false`.
        c @ '[' | c @ ']' | c @ ';' | c @ '#' | c @ '=' | c @ ':' if ini && !quoted => {
            if escape {
                w.write_str(escape_char(c)).map_err(|_| WriteError)
            } else {
                Err(EscapedCharacter(c))
            }
        }

        c => write!(w, "{}", c).map_err(|_| WriteError),
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) trait DisplayLua {
    fn fmt_lua(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result;

    fn do_indent(f: &mut Formatter, indent: u32) -> std::fmt::Result {
        for _ in 0..indent {
            "\t".fmt(f)?;
        }

        Ok(())
    }
}

/// Writes the `string` to the writer `w`, enclosing it in quotes and escaping special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f') and double quotes ('"').
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) fn write_lua_string<W: Write>(w: &mut W, string: &str) -> std::fmt::Result {
    write!(w, "\"")?;

    for c in string.chars() {
        write_char(w, c, false, true, true).map_err(|err| match err {
            WriteCharError::WriteError => std::fmt::Error,
            WriteCharError::EscapedCharacter(_) => unreachable!(),
        })?;
    }

    write!(w, "\"")
}

/// Writes the Lua table `key` to the writer `w`.
/// Writes the string as-is if it's a valid Lua identifier,
/// otherwise encloses it in brackets and quotes, and escapes special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f') and quotes ('"').
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) fn write_lua_key<'k, W: Write>(w: &mut W, key: NonEmptyStr<'k>) -> std::fmt::Result {
    if is_lua_identifier_key(key) {
        write!(w, "{}", key.as_ref())
    } else {
        write!(w, "[")?;
        write_lua_string(w, key.as_ref())?;
        write!(w, "]")
    }
}

/// Returns `true` if the char `c` is a valid Lua identifier character.
/// Lua identifiers start with an ASCII letter and may contain ASCII letters, digits and underscores.
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
fn is_lua_identifier_char(c: char, first: bool) -> bool {
    c.is_ascii_alphabetic() || (!first && ((c == '_') || c.is_ascii_digit()))
}

/// Returns `true` if the non-empty string `key` is a valid Lua identifier.
/// Lua identifiers start with an ASCII letter and may contain ASCII letters, digits and underscores.
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
fn is_lua_identifier_key<'k>(key: NonEmptyStr<'k>) -> bool {
    let mut chars = key.as_ref().chars();

    if !is_lua_identifier_char(chars.next().unwrap(), true) {
        return false;
    }

    for c in chars {
        if !is_lua_identifier_char(c, false) {
            return false;
        }
    }

    true
}

/// A non-empty string slice.
/// Implements `AsRef<str>`.
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NonEmptyStr<'s>(&'s str);

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'s> NonEmptyStr<'s> {
    pub fn new(string: &'s str) -> Result<Self, ()> {
        if string.is_empty() {
            Err(())
        } else {
            Ok(Self(string))
        }
    }

    pub fn into_inner(self) -> &'s str {
        self.0
    }

    pub(crate) unsafe fn new_unchecked(string: &'s str) -> Self {
        Self(string)
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'s> AsRef<str> for NonEmptyStr<'s> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

/// A string literal and its compile-time hash (created via `ministrhash::str_hash_fnv1a!`).
/// Used as an optimization for binary config tables to avoid runtime string hashing.
/// Requires "bin" and "str_hash" features.
#[cfg(all(feature = "bin", feature = "str_hash"))]
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StringAndHash {
    pub string: &'static str,
    pub hash: u32,
}

#[cfg(all(feature = "bin", feature = "str_hash"))]
impl StringAndHash {
    pub fn new(string: &'static str, hash: u32) -> Self {
        Self { string, hash }
    }
}

#[cfg(all(feature = "bin", feature = "str_hash"))]
impl Display for StringAndHash {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "\"{}\"", self.string)
    }
}

/// Creates a binary config [`table`] key and its hash from a string literal.
/// This is slightly more optimal then using a string key
/// as this avoids runtime string hashing, used internally by the binary config [`table`] accessor.
///
/// [`table`]: struct.BinTable.html
#[cfg(all(feature = "bin", feature = "str_hash"))]
#[macro_export]
macro_rules! key {
    ($string:literal) => {
        $crate::TableKey::StringAndHash($crate::StringAndHash::new(
            $string,
            ministrhash::str_hash_fnv1a!($string),
        ))
    };
}

/// A (non-empty) config [`table`] string key.
/// Borrowed, owned, or a compile-time hashed string literal
/// (created via `key!` macro for a binary config table, requires "bin" and "str_hash" features).
///
/// [`table`]: enum.Value.html#variant.Table
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TableKey<'a> {
    /// A normal table string key, borrowed or owned.
    String(Cow<'a, str>),
    /// A string literal + its compile time hash created via `key!` macro.
    #[cfg(all(feature = "bin", feature = "str_hash"))]
    StringAndHash(StringAndHash),
}

#[cfg(feature = "bin")]
impl<'a> TableKey<'a> {
    /// Returns the string key hash.
    /// Used by binary config tables (requires "bin" feature).
    /// Computed on the fly for string keys, or just returns the compile-time hash for
    /// keys created by `key!` macro from a string literal (requires "str_hash" feature).
    pub(crate) fn key_hash(&self) -> u32 {
        match self {
            TableKey::String(string) => string_hash_fnv1a(string),

            #[cfg(feature = "str_hash")]
            TableKey::StringAndHash(StringAndHash { hash, .. }) => *hash,
        }
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> AsRef<str> for TableKey<'a> {
    fn as_ref(&self) -> &str {
        match self {
            TableKey::String(string) => string.as_ref(),
            #[cfg(all(feature = "bin", feature = "str_hash"))]
            TableKey::StringAndHash(StringAndHash { string, .. }) => *string,
        }
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> Display for TableKey<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            TableKey::String(string) => write!(f, "\"{}\"", string),
            #[cfg(all(feature = "bin", feature = "str_hash"))]
            TableKey::StringAndHash(string_and_hash) => string_and_hash.fmt(f),
        }
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> From<&'a str> for TableKey<'a> {
    fn from(other: &'a str) -> Self {
        Self::String(other.into())
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> From<String> for TableKey<'a> {
    fn from(other: String) -> Self {
        Self::String(other.into())
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
/// Key (in the [`table`]) or index (in the [`array`]) of a config element.
///
/// [`table`]: enum.Value.html#variant.Table
/// [`array`]: enum.Value.html#variant.Array
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ConfigKey<'a> {
    /// A (non-empty) string [`table`] key.
    ///
    /// [`table`]: enum.Value.html#variant.Table
    Table(TableKey<'a>),
    /// A (`0`-based) [`array`] index.
    ///
    /// [`array`]: enum.Value.html#variant.Array
    Array(u32),
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> From<&'a str> for ConfigKey<'a> {
    fn from(key: &'a str) -> Self {
        ConfigKey::Table(key.into())
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> From<NonEmptyStr<'a>> for ConfigKey<'a> {
    fn from(key: NonEmptyStr<'a>) -> Self {
        ConfigKey::Table(key.into_inner().into())
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> From<String> for ConfigKey<'a> {
    fn from(key: String) -> Self {
        ConfigKey::Table(key.into())
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> From<TableKey<'a>> for ConfigKey<'a> {
    fn from(key: TableKey<'a>) -> Self {
        ConfigKey::Table(key.into())
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> From<u32> for ConfigKey<'a> {
    fn from(index: u32) -> Self {
        ConfigKey::Array(index)
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> Display for ConfigKey<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ConfigKey::Table(key) => key.fmt(f),
            ConfigKey::Array(key) => key.fmt(f),
        }
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
/// Describes the full path to a config element.
/// Empty path means the root element.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConfigPath<'a>(pub Vec<ConfigKey<'a>>);

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> ConfigPath<'a> {
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
impl<'a> From<Vec<ConfigKey<'a>>> for ConfigPath<'a> {
    fn from(path: Vec<ConfigKey<'a>>) -> Self {
        Self(path)
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", feature = "ini"))]
impl<'a> Display for ConfigPath<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if self.0.is_empty() {
            "<root>".fmt(f)
        } else {
            for (key_index, key) in self.0.iter().enumerate() {
                let last = key_index == (self.0.len() - 1);

                key.fmt(f)?;

                if !last {
                    "/".fmt(f)?;
                }
            }

            Ok(())
        }
    }
}

#[cfg(all(test, any(feature = "bin", feature = "dyn", feature = "lua")))]
pub(crate) fn cmp_f64(l: f64, r: f64) -> bool {
    (l - r).abs() < 0.000_001
}
