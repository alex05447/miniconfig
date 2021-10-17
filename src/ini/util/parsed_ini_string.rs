use {
    crate::{ini::Substr, *},
    ministr::NonEmptyStr,
};

/// Type of the string (non-empty key/section name, or potentially empty value) parsed from the `.ini` source.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ParsedIniStringKind {
    /// The string is in uninitialized state.
    Cleared,
    /// Borrowed directly from the `.ini` source string.
    /// Contains the (inclusive) byte range of the string in the source.
    /// The user guarantees the range is valid and corresponds to a valid UTF-8 substring.
    Borrowed((usize, usize)),
    /// The string is owned and is contained in a helper buffer
    /// (i.e. the parsed string contained at least one escape sequence and thus could not be borrowed directly).
    Owned,
}

/// Represents a string (non-empty key/section name, or potentially empty value) parsed from the `.ini` source.
pub(crate) struct ParsedIniString {
    /// The string's type - borrowed or owned.
    kind: ParsedIniStringKind,
    /// If `kind` is `IniStringKind::Owned`, the string is contained in this buffer.
    buffer: String,
}

impl ParsedIniString {
    fn new() -> Self {
        Self {
            kind: ParsedIniStringKind::Cleared,
            buffer: String::new(),
        }
    }

    /// Pushes the char `c` at byte index `i` in the source string to this string.
    /// Copies `c` to the inner buffer if it's an onwed string,
    /// or updates (increments) the byte range with `i` if it's a cleared or borrowed string.
    fn push(&mut self, c: char, i: usize) {
        use ParsedIniStringKind::*;

        match &mut self.kind {
            Cleared => {
                debug_assert!(self.buffer.is_empty());
                self.kind = Borrowed((i, i));
            }
            Borrowed(range) => {
                debug_assert!(self.buffer.is_empty());
                debug_assert!(
                    range.1 >= range.0,
                    "byte ranges for borrowed strings must be non-empty"
                );
                debug_assert!(
                    i == range.1 + 1,
                    "byte ranges for borrowed strings must be contiguous"
                );
                range.1 = i;
            }
            Owned => {
                debug_assert!(!self.buffer.is_empty());
                self.buffer.push(c);
            }
        }
    }

    /// Converts the string to the `Owned` variant.
    /// Copies the existing borrowed string range, if any, to the inner buffer via `substr`.
    /// Then pushes the char `c` to the inner buffer.
    fn push_owned<'s, S: Substr<'s>>(&mut self, c: char, substr: S) {
        // Become `Owned` even if empty.
        self.to_owned_impl(substr, true);
        self.buffer.push(c);
    }

    /// Converts the string to `Owned` variant, unless it is `Cleared`.
    /// Copies the existing `Borrowed` string range, if any, to the inner buffer via `substr`.
    fn to_owned<'s, S: Substr<'s>>(&mut self, substr: S) {
        self.to_owned_impl(substr, false);
    }

    fn to_owned_impl<'s, S: Substr<'s>>(&mut self, substr: S, force: bool) {
        use ParsedIniStringKind::*;

        match self.kind {
            // No need to become an owned string if the string is empty, unless forced.
            Cleared if force => {
                self.kind = Owned;
            }
            Cleared | Owned => {}
            Borrowed(range) => {
                debug_assert!(self.buffer.is_empty());
                debug_assert!(
                    range.1 >= range.0,
                    "byte ranges for borrowed strings must be non-empty"
                );
                self.buffer.push_str(substr(range.0..=range.1));
                self.kind = Owned;
            }
        }
    }

    fn key<'s, S: Substr<'s>>(&self, substr: S) -> Option<NonEmptyIniStr<'s, '_>> {
        use ParsedIniStringKind::*;

        match self.kind {
            Cleared => {
                debug_assert!(self.buffer.is_empty());
                None
            }
            Borrowed(range) => {
                debug_assert!(self.buffer.is_empty());
                Some(NonEmptyIniStr::Borrowed(substr(range.0..=range.1)))
            }
            Owned => {
                debug_assert!(
                    !self.buffer.is_empty(),
                    "owned `.ini` strings may not be empty"
                );
                Some(NonEmptyIniStr::Owned(unwrap_unchecked(
                    NonEmptyStr::new(&self.buffer),
                    "owned `.ini` string values may not be empty",
                )))
            }
        }
    }

    fn value<'s, S: Substr<'s>>(&self, substr: &S) -> IniStr<'s, '_> {
        use ParsedIniStringKind::*;

        match self.kind {
            Cleared => {
                debug_assert!(self.buffer.is_empty());
                IniStr::Empty
            }
            Borrowed(range) => {
                debug_assert!(self.buffer.is_empty());
                IniStr::Borrowed(substr(range.0..=range.1))
            }
            Owned => {
                debug_assert!(
                    !self.buffer.is_empty(),
                    "owned `.ini` strings may not be empty"
                );
                IniStr::Owned(unwrap_unchecked(
                    NonEmptyStr::new(&self.buffer),
                    "owned `.ini` strings may not be empty",
                ))
            }
        }
    }

    fn clear(&mut self) {
        self.kind = ParsedIniStringKind::Cleared;
        self.buffer.clear();
    }

    fn is_empty(&self) -> bool {
        use ParsedIniStringKind::*;

        match self.kind {
            Cleared => true,
            Borrowed(range) => {
                debug_assert!(
                    range.1 >= range.0,
                    "byte ranges for borrowed `.ini` strings must be non-empty"
                );
                false
            }
            Owned => {
                debug_assert!(
                    !self.buffer.is_empty(),
                    "owned `.ini` strings may not be empty"
                );
                false
            }
        }
    }
}

/// Represents a non-empty string key/section name parsed from the `.ini` source.
pub(crate) struct ParsedIniKey(ParsedIniString);

impl ParsedIniKey {
    pub(crate) fn new() -> Self {
        Self(ParsedIniString::new())
    }

    /// See `ParsedIniString::push()`.
    pub(crate) fn push(&mut self, c: char, idx: usize) {
        self.0.push(c, idx)
    }

    /// See `ParsedIniString::push_owned()`.
    pub(crate) fn push_owned<'s, S: Substr<'s>>(&mut self, c: char, substr: S) {
        self.0.push_owned(c, substr)
    }

    /// See `ParsedIniString::to_owned()`.
    pub(crate) fn to_owned<'s, S: Substr<'s>>(&mut self, substr: S) {
        self.0.to_owned(substr)
    }

    /// See `ParsedIniString::key()`.
    pub(crate) fn key<'s, S: Substr<'s>>(&self, substr: S) -> Option<NonEmptyIniStr<'s, '_>> {
        self.0.key(substr)
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Represents a potentially empty string value parsed from the `.ini` source.
pub(crate) struct ParsedIniValue(ParsedIniString);

impl ParsedIniValue {
    pub(crate) fn new() -> Self {
        Self(ParsedIniString::new())
    }

    /// See `ParsedIniString::push()`.
    pub(crate) fn push(&mut self, c: char, idx: usize) {
        self.0.push(c, idx)
    }

    /// See `ParsedIniString::push_owned()`.
    pub(crate) fn push_owned<'s, S: Substr<'s>>(&mut self, c: char, substr: S) {
        self.0.push_owned(c, substr)
    }

    /// See `ParsedIniString::to_owned()`.
    pub(crate) fn to_owned<'s, S: Substr<'s>>(&mut self, substr: S) {
        self.0.to_owned(substr)
    }

    /// See `ParsedIniString::value()`.
    pub(crate) fn value<'s, S: Substr<'s>>(&self, substr: &S) -> IniStr<'s, '_> {
        self.0.value(substr)
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
