#[cfg(any(all(feature = "dyn", feature = "ini"), feature = "lua"))]
use std::fmt::Write;

#[cfg(any(all(feature = "dyn", feature = "ini"), feature = "lua"))]
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
/// if additionally `ini` is `true` and `quoted` is `false`, also escapes INI special characters
/// ('[', ']', ';', '#', '=', ':').
#[cfg(any(all(feature = "dyn", feature = "ini"), feature = "lua"))]
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

        // Don't escape the INI special characters in quoted strings.
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
