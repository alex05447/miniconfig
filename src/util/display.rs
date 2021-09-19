use {crate::*, std::fmt::Write};

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
            '\0' => r#"\0"#,
            '\x07' => r#"\x07"#, // \a
            '\x08' => r#"\x08"#, // \b
            '\t' => r#"\t"#,
            '\n' => r#"\n"#,
            '\r' => r#"\r"#,
            '\x0b' => r#"\x0b"#,  // \v
            '\x0c' => r#"\\x0c"#, // \f

            '"' => r#"\""#,

            '\'' => r#"\'"#,
            ' ' => r#"\ "#,

            '[' if ini => r#"\["#,
            ']' if ini => r#"\]"#,
            ';' if ini => r#"\;"#,
            '#' if ini => r#"\#"#,
            '=' if ini => r#"\="#,
            ':' if ini => r#"\:"#,

            _ => debug_unreachable!("unknown escaped character"),
        }
    };

    match c {
        // Don't escape the backslashes and just write them as-is if `escape` is false.
        '\\' if escape => w.write_str(r#"\\"#).map_err(|_| WriteError),

        // It's an error if it's a special character or the double quotes and `escape` is `false`.
        c @ '\0'
        | c @ '\x07' // \a
        | c @ '\x08' // \b
        | c @ '\t'
        | c @ '\n'
        | c @ '\r'
        | c @ '\x0b' // \v
        | c @ '\x0c' // \f
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

        c => w.write_char(c).map_err(|_| WriteError),
    }
}
