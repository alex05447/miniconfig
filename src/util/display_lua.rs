use {crate::*, std::fmt::Write};

pub(crate) trait DisplayLua {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result;

    fn do_indent<W: Write>(w: &mut W, indent: u32) -> std::fmt::Result {
        for _ in 0..indent {
            w.write_char('\t')?;
        }

        Ok(())
    }
}

/// Writes the `string` to the writer `w`, enclosing it in quotes and escaping special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f') and double quotes ('"').
pub(crate) fn write_lua_string<W: Write>(w: &mut W, string: &str) -> std::fmt::Result {
    w.write_char('"')?;

    for c in string.chars() {
        write_char(w, c, false, true, true).map_err(|err| match err {
            WriteCharError::WriteError => std::fmt::Error,
            WriteCharError::EscapedCharacter(_) => debug_unreachable!(
                "should never get an `EscapedCharacter` error when `escape` flag is `true`"
            ),
        })?;
    }

    w.write_char('"')
}

/// Writes the Lua table `key` to the writer `w`.
/// Writes the string as-is if it's a valid Lua identifier,
/// otherwise encloses it in brackets and quotes, and escapes special characters
/// ('\\', '\0', '\a', '\b', '\t', '\n', '\r', '\v', '\f') and quotes ('"').
pub(crate) fn write_lua_key<W: Write>(w: &mut W, key: &NonEmptyStr) -> std::fmt::Result {
    if is_lua_identifier_key(key) {
        write!(w, "{}", key)
    } else {
        w.write_char('[')?;
        write_lua_string(w, key.as_str())?;
        w.write_char(']')
    }
}

/// Returns `true` if the non-empty string `key` is a valid Lua identifier.
/// Lua identifiers start with an ASCII letter and may contain ASCII letters, digits and underscores.
fn is_lua_identifier_key(key: &NonEmptyStr) -> bool {
    for (idx, key_char) in key.as_str().chars().enumerate() {
        if !is_lua_identifier_char(key_char, idx == 0) {
            return false;
        }
    }

    true
}

/// Returns `true` if the char `c` is a valid Lua identifier character.
/// Lua identifiers start with an ASCII letter and may contain ASCII letters, digits and underscores.
fn is_lua_identifier_char(c: char, first: bool) -> bool {
    c.is_ascii_alphabetic() || (!first && ((c == '_') || c.is_ascii_digit()))
}
