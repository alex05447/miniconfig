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

        let needs_quotes = string_needs_quotes(section.as_ref(), nested_sections);

        if needs_quotes {
            write!(w, "\"").map_err(|_| WriteError)?;
        }

        write_ini_string(w, section.as_ref(), needs_quotes, escape)?;

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
pub(crate) fn write_ini_array<'k, W: Write, A: Iterator<Item = I>, I: Borrow<V>, V: DisplayIni>(
    w: &mut W,
    key: NonEmptyStr<'k>,
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
pub(crate) fn write_ini_table<'k, W: Write, V: DisplayIni>(
    w: &mut W,
    key: NonEmptyStr<'k>,
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

    path.push(key);

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
pub(crate) fn write_ini_value<'k, W: Write, V: DisplayIni>(
    w: &mut W,
    key: NonEmptyStr<'k>,
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
fn write_ini_key<'k, W: Write>(
    w: &mut W,
    key: NonEmptyStr<'k>,
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

/// A simple wrapper around the nested `.ini` section path,
/// used instead of `Vec<String>` to minimize the number of allocations.
pub(crate) struct IniPath {
    // Contains the contiguously stored nested section names.
    // |foo|bill|bob|
    path: String,
    // Contains the offsets past the last byte of each section name in the path.
    // | 3 |  7 | 10|
    offsets: Vec<u32>,
}

impl IniPath {
    pub(crate) fn new() -> Self {
        Self {
            path: String::new(),
            offsets: Vec::new(),
        }
    }

    /// Pushes a new section name to the end of the path.
    pub(crate) fn push<'s>(&mut self, section: NonEmptyStr<'s>) {
        let current_len = self.path.len() as u32;
        let len = section.as_ref().len() as u32;

        let offset = current_len + len;

        self.path.push_str(section.as_ref());
        self.offsets.push(offset);
    }

    /// Pops a section name off the end of the path.
    /// NOTE - the caller guarantees that the path is not empty.
    //#[cfg(any(feature = "bin", feature = "dyn", feature = "lua", test))]
    pub(crate) fn pop(&mut self) {
        debug_assert!(!self.offsets.is_empty());

        self.offsets.pop();

        if let Some(last) = self.offsets.last() {
            self.path.truncate(*last as _);
        } else {
            self.path.clear();
        }
    }

    pub(crate) fn last(&self) -> Option<NonEmptyStr<'_>> {
        if self.is_empty() {
            None
        } else {
            Some(unsafe { self.slice(self.len() - 1) })
        }
    }

    /// Returns the number of section names in the path.
    pub(crate) fn len(&self) -> u32 {
        self.offsets.len() as _
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over nested section path parts, from parent to child.
    pub(crate) fn iter(&self) -> impl std::iter::Iterator<Item = NonEmptyStr<'_>> {
        IniPathIter::new(self)
    }

    pub(crate) fn to_config_path<'a>(&self) -> ConfigPath<'a> {
        let mut path = ConfigPath::new();

        for section in self.iter() {
            path.0.push(ConfigKey::Table(TableKey::from(
                section.as_ref().to_owned(),
            )));
        }

        path
    }

    /// Returns the section name at `index` in the path.
    /// NOTE - the caller guarantees `index` is valid.
    /// Passing an invalid `index` is UB.
    unsafe fn slice(&self, index: u32) -> NonEmptyStr<'_> {
        debug_assert!(index < self.len());

        let end = *(self.offsets.get_unchecked(index as usize)) as _;

        debug_assert!(end > 0);

        let start = if index == 0 {
            0
        } else {
            *(self.offsets.get_unchecked((index - 1) as usize)) as _
        };

        debug_assert!(start < end);

        unwrap_unchecked_msg(
            NonEmptyStr::new(&self.path[start..end]),
            "empty section name",
        )
    }
}

/// Iterates over the `IniPath` nested section path parts, parent to child.
struct IniPathIter<'a> {
    path: &'a IniPath,
    index: u32,
}

impl<'a> IniPathIter<'a> {
    fn new(path: &'a IniPath) -> Self {
        Self { path, index: 0 }
    }
}

impl<'a> std::iter::Iterator for IniPathIter<'a> {
    type Item = NonEmptyStr<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.path.len() {
            None
        } else {
            let index = self.index;
            self.index += 1;
            Some(unsafe { self.path.slice(index) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(non_snake_case)]
    #[test]
    fn IniPath() {
        let mut path = IniPath::new();

        assert!(path.is_empty());
        assert!(path.len() == 0);

        path.push(nestr!("foo"));

        assert!(!path.is_empty());
        assert!(path.len() == 1);

        assert_eq!(unsafe { path.slice(0) }, nestr!("foo"));

        path.push(nestr!("bill"));

        assert!(!path.is_empty());
        assert!(path.len() == 2);

        assert_eq!(unsafe { path.slice(0) }, nestr!("foo"));
        assert_eq!(unsafe { path.slice(1) }, nestr!("bill"));

        path.push(nestr!("bob"));

        assert!(!path.is_empty());
        assert!(path.len() == 3);

        assert_eq!(unsafe { path.slice(0) }, nestr!("foo"));
        assert_eq!(unsafe { path.slice(1) }, nestr!("bill"));
        assert_eq!(unsafe { path.slice(2) }, nestr!("bob"));

        for (idx, path_part) in path.iter().enumerate() {
            match idx {
                0 => assert_eq!(path_part, nestr!("foo")),
                1 => assert_eq!(path_part, nestr!("bill")),
                2 => assert_eq!(path_part, nestr!("bob")),
                _ => unreachable!(),
            }
        }

        path.pop();

        assert!(!path.is_empty());
        assert!(path.len() == 2);

        assert_eq!(unsafe { path.slice(0) }, nestr!("foo"));
        assert_eq!(unsafe { path.slice(1) }, nestr!("bill"));

        for (idx, path_part) in path.iter().enumerate() {
            match idx {
                0 => assert_eq!(path_part, nestr!("foo")),
                1 => assert_eq!(path_part, nestr!("bill")),
                _ => unreachable!(),
            }
        }

        path.pop();

        assert!(!path.is_empty());
        assert!(path.len() == 1);

        assert_eq!(unsafe { path.slice(0) }, nestr!("foo"));

        for (idx, path_part) in path.iter().enumerate() {
            match idx {
                0 => assert_eq!(path_part, nestr!("foo")),
                _ => unreachable!(),
            }
        }

        path.pop();

        assert!(path.is_empty());
        assert!(path.len() == 0);
    }
}
