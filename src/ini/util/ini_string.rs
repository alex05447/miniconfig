use ministr::{NonEmptyStr, NonEmptyString};

/// Type for (maybe empty) string values returned by the [`.ini parser`](struct.IniParser.html).
/// If not empty, either borrowed directly from the `.ini` source, if possible,
/// or contained in a temporary helper buffer in the parser.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IniStr<'s, 'a> {
    /// The (non-empty) string is borrowed directly from the `.ini` source.
    Borrowed(&'s NonEmptyStr),
    /// The (non-empty) string is owned and is contained in a temporary helper buffer in the [`.ini parser`](struct.IniParser.html)
    /// (i.e. the parsed string contained at least one escape sequence and thus could not be borrowed directly).
    Owned(&'a NonEmptyStr),
    /// The string is empty.
    Empty,
}

impl<'s, 'a> IniStr<'s, 'a> {
    pub fn as_str(&self) -> &str {
        match self {
            IniStr::Borrowed(_str) => _str.as_str(),
            IniStr::Owned(_str) => _str.as_str(),
            IniStr::Empty => "",
        }
    }
}

impl<'s, 'a> AsRef<str> for IniStr<'s, 'a> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'s, 'a> Into<String> for IniStr<'s, 'a> {
    fn into(self) -> String {
        self.as_str().into()
    }
}

/// Type for non-empty string keys / section names returned by the `.ini` parser.
/// Either borrowed directly from the `.ini` source, if possible,
/// or otherwise contained in a temporary helper buffer in the parser.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NonEmptyIniStr<'s, 'a> {
    /// The (non-empty) string is borrowed directly from the `.ini` source.
    Borrowed(&'s NonEmptyStr),
    /// The (non-empty) string is owned and is contained in a temporary helper buffer in the parser
    /// (i.e. the parsed string contained at least one escape sequence and thus could not be borrowed directly).
    Owned(&'a NonEmptyStr),
}

impl<'s, 'a> NonEmptyIniStr<'s, 'a> {
    pub fn as_ne_str(&self) -> &NonEmptyStr {
        match self {
            NonEmptyIniStr::Borrowed(_str) => _str,
            NonEmptyIniStr::Owned(_str) => _str,
        }
    }

    pub fn as_str(&self) -> &str {
        self.as_ne_str().as_str()
    }
}

impl<'s, 'a> AsRef<NonEmptyStr> for NonEmptyIniStr<'s, 'a> {
    fn as_ref(&self) -> &NonEmptyStr {
        self.as_ne_str()
    }
}

impl<'s, 'a> AsRef<str> for NonEmptyIniStr<'s, 'a> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'s, 'a> Into<NonEmptyString> for NonEmptyIniStr<'s, 'a> {
    fn into(self) -> NonEmptyString {
        self.as_ne_str().into()
    }
}

impl<'s, 'a> Into<String> for NonEmptyIniStr<'s, 'a> {
    fn into(self) -> String {
        self.as_ne_str().into()
    }
}
