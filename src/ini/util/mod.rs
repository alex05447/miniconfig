mod ini_path;
mod ini_string;
mod parsed_ini_string;

pub use ini_string::*;

pub(crate) use {ini_path::*, parsed_ini_string::*};

use {
    crate::{
        util::{write_char, WriteCharError},
        *,
    },
    std::fmt::Write,
};

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
use std::{borrow::Borrow, iter::Iterator};

/// A trait implemented by configs serializable to an `.ini` string.
pub(crate) trait DisplayIni {
    fn fmt_ini<W: Write>(
        &self,
        writer: &mut W,
        level: u32,
        array: bool,
        path: &mut IniPath,
        options: ToIniStringOptions,
    ) -> Result<(), ToIniStringError>;
}

impl<S, A, T> DisplayIni for Value<S, A, T>
where
    S: AsRef<str>,
    T: DisplayIni,
{
    fn fmt_ini<W: Write>(
        &self,
        writer: &mut W,
        level: u32,
        array: bool,
        path: &mut IniPath,
        options: ToIniStringOptions,
    ) -> Result<(), ToIniStringError> {
        use ToIniStringError::*;

        match self {
            Value::Bool(value) => {
                write!(writer, "{}", if *value { "true" } else { "false" }).map_err(|_| WriteError)
            }
            Value::I64(value) => write!(writer, "{}", value).map_err(|_| WriteError),
            Value::F64(value) => write!(writer, "{}", value).map_err(|_| WriteError),
            Value::String(value) => {
                write!(writer, "\"").map_err(|_| WriteError)?;
                write_ini_string(writer, value.as_ref(), true, options.escape)?;
                write!(writer, "\"").map_err(|_| WriteError)
            }
            Value::Table(value) => {
                if array {
                    Err(InvalidArrayType)
                } else {
                    debug_assert!(options.nested_sections() || level < 2);
                    value.fmt_ini(writer, level, false, path, options)
                }
            }
            Value::Array(_) => {
                if array {
                    Err(InvalidArrayType)
                } else {
                    debug_unreachable!("array foramtting is handled by parent tables")
                }
            }
        }
    }
}

/// Writes the `string` to the writer `w`.
/// If `escape` is `true`, escapes special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f'),
/// double quotes ('"'),
/// and, if `quoted` is `false`, single quotes ('\'') and spaces (' ');
/// and `.ini` special characters ('[', ']', ';', '#', '=', ':').
/// If `quoted` is `true`, single quotes ('\'') are not escaped.
/// If `escape` is `false` and and the `string` contains a character which must be escaped, returns an error.
pub(crate) fn write_ini_string<W: Write>(
    w: &mut W,
    string: &str,
    quoted: bool,
    escape: bool,
) -> Result<(), ToIniStringError> {
    for c in string.chars() {
        write_char(w, c, true, quoted, escape).map_err(|err| match err {
            WriteCharError::WriteError => ToIniStringError::WriteError,
            WriteCharError::EscapedCharacter(c) => ToIniStringError::EscapedCharacterNotAllowed(c),
        })?;
    }

    Ok(())
}

/// Returns `true` if the string contains special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r'),
/// string quotes ('\'', '"'),
/// `.ini` special characters ('[', ']', ';', '#', '=', ':') or spaces (' '),
/// and if `escape_nested_section_separators` is `true`, nested section separators ('/').
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
fn string_needs_quotes(string: &str, escape_nested_section_separators: bool) -> bool {
    for c in string.chars() {
        match c {
            // Special characters.
            '\\' | '\0' | '\x07' /* '\a' */ | '\x08' /* '\b' */ | '\t' | '\n' | '\r' | '\x0b' /* '\v' */ | '\x0c' /* '\f' */ => { return true; },
            // Space.
            ' ' => { return true; },
            // `.ini` special characters.
            '[' | ']' | ';' | '#' | '=' | ':' => { return true; },
            // Quotes.
            '\'' | '"' => { return true; },
            '/' if escape_nested_section_separators => { return true; },
            _ => {},
        }
    }

    false
}

/// Writes the (non-empty) section `path` to the writer `w`, enclosing it in brackets ('[' / ']').
/// If the sections in `path` contain special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r'),
/// string quotes ('\'', '"'),
/// `.ini` special characters ('[', ']', ';', '#', '=', ':'),
/// spaces (' '),
/// or if `nested_sections` is `true`, nested section separators ('/'),
/// they are additionally enclosed in double quotes ('"').
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
fn write_ini_sections<W: Write>(
    w: &mut W,
    path: &IniPath,
    escape: bool,
    nested_sections: bool,
) -> Result<(), ToIniStringError> {
    use ToIniStringError::*;

    debug_assert!(!path.is_empty());
    let num_sections = path.len();
    debug_assert!(num_sections > 0);

    write!(w, "[").map_err(|_| WriteError)?;

    for (index, section) in path.iter().enumerate() {
        let last = (index as u32) == (num_sections - 1);

        let needs_quotes = string_needs_quotes(section.as_ne_str(), nested_sections);

        if needs_quotes {
            write!(w, "\"").map_err(|_| WriteError)?;
        }

        write_ini_string(w, section.as_ne_str(), needs_quotes, escape)?;

        if needs_quotes {
            write!(w, "\"").map_err(|_| WriteError)?;
        }

        if !last {
            debug_assert!(nested_sections);
            write!(w, "/").map_err(|_| WriteError)?;
        }
    }

    write!(w, "]").map_err(|_| WriteError)
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) fn write_ini_array<W: Write, A: Iterator<Item = I>, I: Borrow<V>, V: DisplayIni>(
    w: &mut W,
    key: &NonEmptyStr,
    array: A,
    array_len: usize,
    last: bool,
    level: u32,
    path: &mut IniPath,
    options: ToIniStringOptions,
) -> Result<(), ToIniStringError> {
    use ToIniStringError::*;

    if options.arrays {
        write_ini_key(w, key, options.escape)?;

        write!(w, " = [").map_err(|_| WriteError)?;

        for (array_index, array_value) in array.enumerate() {
            let last = array_index == array_len - 1;

            array_value
                .borrow()
                .fmt_ini(w, level + 1, true, path, options)?;

            if !last {
                write!(w, ", ").map_err(|_| WriteError)?;
            }
        }

        write!(w, "]").map_err(|_| WriteError)?;

        if !last {
            writeln!(w).map_err(|_| WriteError)?;
        }
    } else {
        return Err(ArraysNotAllowed);
    }

    Ok(())
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) fn write_ini_table<W: Write, V: DisplayIni>(
    w: &mut W,
    key: &NonEmptyStr,
    key_index: u32,
    value: &V,
    value_len: u32,
    last: bool,
    level: u32,
    path: &mut IniPath,
    options: ToIniStringOptions,
) -> Result<(), ToIniStringError> {
    use ToIniStringError::*;

    if level >= options.nested_section_depth {
        return Err(NestedSectionDepthExceeded);
    }

    if key_index > 0 {
        writeln!(w).map_err(|_| WriteError)?;
    }

    path.push(NonEmptyIniStr::Owned(key));

    write_ini_sections(w, path, options.escape, options.nested_sections())?;

    if value_len > 0 {
        writeln!(w).map_err(|_| WriteError)?;
        value.fmt_ini(w, level + 1, false, path, options)?;
    }

    if !last {
        writeln!(w).map_err(|_| WriteError)?;
    }

    path.pop();

    Ok(())
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
pub(crate) fn write_ini_value<W: Write, V: DisplayIni>(
    w: &mut W,
    key: &NonEmptyStr,
    value: &V,
    last: bool,
    level: u32,
    array: bool,
    path: &mut IniPath,
    options: ToIniStringOptions,
) -> Result<(), ToIniStringError> {
    use ToIniStringError::*;

    write_ini_key(w, key, options.escape)?;

    write!(w, " = ").map_err(|_| WriteError)?;

    value.fmt_ini(w, level + 1, array, path, options)?;

    if !last {
        writeln!(w).map_err(|_| WriteError)?;
    }

    Ok(())
}

/// Writes the `key` to the writer `w`.
/// If the `key` contains special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r'),
/// string quotes ('\'', '"'),
/// `.ini` special characters ('[', ']', ';', '#', '=', ':') or spaces (' '),
/// it is additionally enclosed in double quotes ('"').
#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
fn write_ini_key<W: Write>(
    w: &mut W,
    key: &NonEmptyStr,
    escape: bool,
) -> Result<(), ToIniStringError> {
    use ToIniStringError::*;

    let needs_quotes = string_needs_quotes(key.as_ref(), false);

    if needs_quotes {
        write!(w, "\"").map_err(|_| WriteError)?;
    }

    write_ini_string(w, key.as_ref(), needs_quotes, escape)?;

    if needs_quotes {
        write!(w, "\"").map_err(|_| WriteError)?;
    }

    Ok(())
}
