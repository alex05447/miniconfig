use std::fmt::Write;

use crate::{write_char, ToINIStringError, Value};

pub(crate) trait DisplayINI {
    fn fmt_ini<W: Write>(&self, writer: &mut W, level: u32) -> Result<(), ToINIStringError>;
}

impl<S, A, T> DisplayINI for Value<S, A, T>
where
    S: AsRef<str>,
    T: DisplayINI,
{
    fn fmt_ini<W: Write>(&self, writer: &mut W, level: u32) -> Result<(), ToINIStringError> {
        use ToINIStringError::*;

        match self {
            Value::Bool(value) => {
                write!(writer, "{}", if *value { "true" } else { "false" }).map_err(|_| WriteError)
            }
            Value::I64(value) => write!(writer, "{}", value).map_err(|_| WriteError),
            Value::F64(value) => write!(writer, "{}", value).map_err(|_| WriteError),
            Value::String(value) => {
                write!(writer, "\"").map_err(|_| WriteError)?;
                write_ini_string(writer, value.as_ref(), true).map_err(|_| WriteError)?;
                write!(writer, "\"").map_err(|_| WriteError)
            }
            Value::Table(value) => {
                debug_assert!(level < 2);
                value.fmt_ini(writer, level)
            }

            Value::Array(_) => Err(ArraysNotSupported),
        }
    }
}

/// Writes the `string` to the writer `w`, escaping special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f')
/// and, if `quoted` is `false`, string quotes ('\'', '"'),
/// INI special characters ('[', ']', ';', '#', '=', ':') and spaces (' ').
/// If `quoted` is `true`, single quotes ('\'') are not escaped.
pub(crate) fn write_ini_string<W: Write>(
    w: &mut W,
    string: &str,
    quoted: bool,
) -> std::fmt::Result {
    for c in string.chars() {
        write_char(w, c, true, quoted)?;
    }

    Ok(())
}

fn section_needs_quotes(section: &str) -> bool {
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

/// Writes the `section` to the writer `w`, enclosing it in brackets.
/// If the section contains special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r'),
/// string quotes ('\'', '"'),
/// INI special characters ('[', ']', ';', '#', '=', ':') or spaces (' '),
/// it is additionally enclosed in quotes ('"').
pub(crate) fn write_ini_section<W: Write>(w: &mut W, section: &str) -> std::fmt::Result {
    debug_assert!(!section.is_empty());

    write!(w, "[")?;

    let needs_quotes = section_needs_quotes(section);

    if needs_quotes {
        write!(w, "\"")?;
    }

    write_ini_string(w, section, needs_quotes)?;

    if needs_quotes {
        write!(w, "\"")?;
    }

    write!(w, "]")
}
