use std::fmt::Write;

use crate::{write_char, ToIniStringError, ToIniStringOptions, Value, WriteCharError};

pub(crate) trait DisplayIni {
    fn fmt_ini<W: Write>(
        &self,
        writer: &mut W,
        level: u32,
        array: bool,
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
                write_ini_string_impl(writer, value.as_ref(), true, options.escape)?;
                write!(writer, "\"").map_err(|_| WriteError)
            }
            Value::Table(value) => {
                if array {
                    Err(InvalidArrayType)
                } else {
                    debug_assert!(level < 2);
                    value.fmt_ini(writer, level, false, options)
                }
            }
            Value::Array(_) => {
                if array {
                    Err(InvalidArrayType)
                } else {
                    unreachable!(); // Handled by parent tables.
                }
            }
        }
    }
}

/// Writes the `string` to the writer `w`, escaping special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f')
/// and, if `quoted` is `false`, string quotes ('\'', '"'),
/// INI special characters ('[', ']', ';', '#', '=', ':') and spaces (' ').
/// If `quoted` is `true`, single quotes ('\'') are not escaped.
pub(crate) fn write_ini_string_impl<W: Write>(
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
/// INI special characters ('[', ']', ';', '#', '=', ':') or spaces (' '),
fn string_needs_quotes(section: &str) -> bool {
    for c in section.chars() {
        match c {
            // Special characters.
            '\\' | '\0' | '\x07' /* '\a' */ | '\x08' /* '\b' */ | '\t' | '\n' | '\r' | '\x0b' /* '\v' */ | '\x0c' /* '\f' */ => { return true; },
            // Space.
            ' ' => { return true; },
            // INI special characters.
            '[' | ']' | ';' | '#' | '=' | ':' => { return true; },
            // Quotes.
            '\'' | '"' => { return true; },
            _ => {},
        }
    }

    false
}

/// Writes the `section` to the writer `w`, enclosing it in brackets ('[' / ']').
/// If the `section` contains special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r'),
/// string quotes ('\'', '"'),
/// INI special characters ('[', ']', ';', '#', '=', ':') or spaces (' '),
/// it is additionally enclosed in double quotes ('"').
pub(crate) fn write_ini_section<W: Write>(
    w: &mut W,
    section: &str,
    escape: bool,
) -> Result<(), ToIniStringError> {
    use ToIniStringError::*;

    debug_assert!(!section.is_empty());

    write!(w, "[").map_err(|_| WriteError)?;

    let needs_quotes = string_needs_quotes(section);

    if needs_quotes {
        write!(w, "\"").map_err(|_| WriteError)?;
    }

    write_ini_string_impl(w, section, needs_quotes, escape)?;

    if needs_quotes {
        write!(w, "\"").map_err(|_| WriteError)?;
    }

    write!(w, "]").map_err(|_| WriteError)
}

/// Writes the `key` to the writer `w`.
/// If the `key` contains special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r'),
/// string quotes ('\'', '"'),
/// INI special characters ('[', ']', ';', '#', '=', ':') or spaces (' '),
/// it is additionally enclosed in double quotes ('"').
pub(crate) fn write_ini_key<W: Write>(
    w: &mut W,
    key: &str,
    escape: bool,
) -> Result<(), ToIniStringError> {
    use ToIniStringError::*;

    debug_assert!(!key.is_empty());

    let needs_quotes = string_needs_quotes(key);

    if needs_quotes {
        write!(w, "\"").map_err(|_| WriteError)?;
    }

    write_ini_string_impl(w, key, needs_quotes, escape)?;

    if needs_quotes {
        write!(w, "\"").map_err(|_| WriteError)?;
    }

    Ok(())
}
