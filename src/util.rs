#[cfg(any(feature = "bin", feature = "dyn", feature = "ini", feature = "lua"))]
use std::fmt::Write;

#[cfg(feature = "lua")]
use std::convert::From;

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
use std::{
    borrow::Cow,
    fmt::{Display, Formatter},
};

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
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f')
/// double quotes ('"'),
/// and, if `quoted` is `false`, single quotes ('\'') and spaces (' ');
/// if additionally `ini` is `true` and `quoted` is `false`, also escapes `.ini` special characters
/// ('[', ']', ';', '#', '=', ':').
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

            '\'' => "\\\'",
            ' ' => "\\ ",

            '"' => "\\\"",

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
pub(crate) fn write_lua_key<W: Write>(w: &mut W, key: &str) -> std::fmt::Result {
    debug_assert!(!key.is_empty());

    if is_lua_identifier_key(key) {
        write!(w, "{}", key)
    } else {
        write!(w, "[")?;
        write_lua_string(w, key)?;
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
fn is_lua_identifier_key(key: &str) -> bool {
    debug_assert!(!key.is_empty());

    let mut chars = key.chars();

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

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
/// Key (in the [`table`]) or index (in the [`array`]) of a config element.
///
/// [`table`]: enum.Value.html#variant.Table
/// [`array`]: enum.Value.html#variant.Array
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ConfigKey<'a> {
    /// A (non-empty) string [`table`] key.
    ///
    /// [`table`]: enum.Value.html#variant.Table
    Table(Cow<'a, str>),
    /// A (`0`-based) [`array`] index.
    ///
    /// [`array`]: enum.Value.html#variant.Array
    Array(u32),
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
impl<'a> From<&'a str> for ConfigKey<'a> {
    fn from(key: &'a str) -> Self {
        ConfigKey::Table(key.into())
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
impl<'a> From<String> for ConfigKey<'a> {
    fn from(key: String) -> Self {
        ConfigKey::Table(key.into())
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
impl<'a> From<u32> for ConfigKey<'a> {
    fn from(index: u32) -> Self {
        ConfigKey::Array(index)
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
impl<'a> Display for ConfigKey<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ConfigKey::Table(key) => write!(f, "\"{}\"", key),
            ConfigKey::Array(key) => write!(f, "{}", key),
        }
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
/// Describes the full path to a config element.
/// Empty path means the root element.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ConfigPath<'a>(pub Vec<ConfigKey<'a>>);

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
impl<'a> ConfigPath<'a> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn from_key(key: ConfigKey<'a>) -> Self {
        Self(vec![key])
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
impl<'a> From<Vec<ConfigKey<'a>>> for ConfigPath<'a> {
    fn from(path: Vec<ConfigKey<'a>>) -> Self {
        Self(path)
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
impl<'a> Display for ConfigPath<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if self.0.is_empty() {
            "<root>".fmt(f)
        } else {
            for (key_index, key) in self.0.iter().enumerate() {
                let last = key_index == (self.0.len() - 1);

                key.fmt(f)?;

                if !last {
                    ".".fmt(f)?;
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
