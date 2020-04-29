use std::fmt::Write;

/// Writes the char `c` to the writer `w`, escaping special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f')
/// and, if `quoted` is `false`, string quotes ('\'', '"').
/// If `quoted` is `true`, single quotes ('\'') and spaces (' ') are not escaped.
/// If `ini` is `true` and `quoted` is `false`, also escapes INI special characters
/// ('[', ']', ';', '#', '=', ':').
pub(crate) fn write_char<W: Write>(
    w: &mut W,
    c: char,
    ini: bool,
    quoted: bool,
) -> std::fmt::Result {
    match c {
        '\\' => write!(w, "\\\\"),
        '\0' => write!(w, "\\0"),
        '\x07' => write!(w, "\\x07"), // \a
        '\x08' => write!(w, "\\x08"), // \b
        '\t' => write!(w, "\\t"),
        '\n' => write!(w, "\\n"),
        '\r' => write!(w, "\\r"),
        '\x0b' => write!(w, "\\x0b"), // \v
        '\x0c' => write!(w, "\\x0c"), // \f

        '\'' if !quoted => write!(w, "\\\'"),
        ' ' if !quoted => write!(w, "\\ "),

        '"' => write!(w, "\\\""),

        '[' if ini && !quoted => write!(w, "\\["),
        ']' if ini && !quoted => write!(w, "\\]"),
        ';' if ini && !quoted => write!(w, "\\;"),
        '#' if ini && !quoted => write!(w, "\\#"),
        '=' if ini && !quoted => write!(w, "\\="),
        ':' if ini && !quoted => write!(w, "\\:"),

        c => write!(w, "{}", c),
    }
}
