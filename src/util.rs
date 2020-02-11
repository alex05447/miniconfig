use std::fmt::Write;

/// Writes the char `c` to the writer `w`, escaping special characters
/// ('\\', '\'', '\"', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r').
/// If `ini` is `true`, also escapes INI special characters
/// ('[', ']', ';', '#', '=', ':')
/// and, if in addition `quoted` is `false`, spaces (' ').
pub(crate) fn write_char<W: Write>(
    w: &mut W,
    c: char,
    ini: bool,
    quoted: bool,
) -> std::fmt::Result {
    match c {
        '\\' => write!(w, "\\\\"),
        '\'' => write!(w, "\\\'"),
        '\"' => write!(w, "\\\""),
        '\0' => write!(w, "\\0"),
        '\x07' => write!(w, "\\x07"), // \a
        '\x08' => write!(w, "\\x08"), // \b
        '\t' => write!(w, "\\t"),
        '\n' => write!(w, "\\n"),
        '\x0b' => write!(w, "\\x0b"), // \v
        '\x0c' => write!(w, "\\x0c"), // \f
        '\r' => write!(w, "\\r"),

        ' ' if ini && !quoted => write!(w, "\\ "),

        '[' if ini => write!(w, "\\["),
        ']' if ini => write!(w, "\\]"),
        ';' if ini => write!(w, "\\;"),
        '#' if ini => write!(w, "\\#"),
        '=' if ini => write!(w, "\\="),
        ':' if ini => write!(w, "\\:"),

        c => write!(w, "{}", c),
    }
}
