use std::fmt::{Display, Formatter, Write};

use crate::{write_char, Value};

/// An error returned by `to_ini_string`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToINIStringError {
    /// Array values are not supported in INI configs.
    ArraysNotSupported,
    /// Tables nested within tables are not supported in INI configs.
    NestedTablesNotSupported,
    /// General write error (out of memory?).
    WriteError,
}

impl Display for ToINIStringError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use ToINIStringError::*;

        match self {
            ArraysNotSupported => write!(f, "Array values are not supported in INI configs."),
            NestedTablesNotSupported => write!(
                f,
                "Tables nested within tables are not supported in INI configs."
            ),
            WriteError => write!(f, "General write error (out of memory?)."),
        }
    }
}

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

        debug_assert!(level < 2);

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
            Value::Table(value) => value.fmt_ini(writer, level),

            Value::Array(_) => Err(ArraysNotSupported),
        }
    }
}

/// Writes the `string` to the writer `w`, escaping special characters
/// ('\\', '\'', '\"', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r')
/// and INI special characters
/// ('[', ']', ';', '#', '=', ':'),
/// and, if in addition `quoted` is `false`, spaces (' ').
pub(crate) fn write_ini_string<W: Write>(w: &mut W, string: &str, quoted: bool) -> std::fmt::Result {
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
