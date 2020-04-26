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
/// ('\\', '\'', '\"', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r')
/// and INI special characters
/// ('[', ']', ';', '#', '=', ':'),
/// and, if in addition `quoted` is `false`, spaces (' ').
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

/// Writes the `section` to the writer `w`, enclosing it in brackets and escaping special characters
/// ('\\', '\'', '\"', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r')
/// and INI special characters
/// ('[', ']', ';', '#', '=', ':').
pub(crate) fn write_ini_section<W: Write>(w: &mut W, section: &str) -> std::fmt::Result {
    write!(w, "[")?;
    write_ini_string(w, section, false)?;
    write!(w, "]")
}
