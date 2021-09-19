use {
    crate::*,
    std::fmt::{Display, Formatter},
};

/// An `.ini` section non-empty string key.
/// Used in [`.ini parser`](struct.IniParser.html) [`errors`](struct.IniError.html).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IniSectionKey(pub NonEmptyString);

impl AsRef<str> for IniSectionKey {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Display for IniSectionKey {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

impl<'s> From<&'s NonEmptyStr> for IniSectionKey {
    fn from(other: &'s NonEmptyStr) -> Self {
        Self(other.into())
    }
}

impl From<NonEmptyString> for IniSectionKey {
    fn from(other: NonEmptyString) -> Self {
        Self(other)
    }
}

/// String key (in the section) or integer index (in the array) of an `.ini` config element.
/// Used in [`.ini parser`](struct.IniParser.html) [`errors`](struct.IniError.html).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum IniConfigKey {
    Section(IniSectionKey),
    Array(u32),
}

impl<'a> From<&'a NonEmptyStr> for IniConfigKey {
    fn from(key: &'a NonEmptyStr) -> Self {
        IniConfigKey::Section(key.into())
    }
}

impl From<NonEmptyString> for IniConfigKey {
    fn from(key: NonEmptyString) -> Self {
        IniConfigKey::Section(key.into())
    }
}

impl From<IniSectionKey> for IniConfigKey {
    fn from(key: IniSectionKey) -> Self {
        IniConfigKey::Section(key)
    }
}

impl From<u32> for IniConfigKey {
    fn from(index: u32) -> Self {
        IniConfigKey::Array(index)
    }
}

impl Display for IniConfigKey {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            IniConfigKey::Section(key) => key.fmt(f),
            IniConfigKey::Array(key) => key.fmt(f),
        }
    }
}

/// Describes the full path to an `.ini` config element.
/// Empty path means the root section.
/// Used in [`.ini parser`](struct.IniParser.html) [`errors`](struct.IniError.html).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IniConfigPath(pub Vec<IniConfigKey>);

impl IniConfigPath {
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }
}

impl From<Vec<IniConfigKey>> for IniConfigPath {
    fn from(path: Vec<IniConfigKey>) -> Self {
        Self(path)
    }
}

impl Display for IniConfigPath {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if self.0.is_empty() {
            "<root>".fmt(f)
        } else {
            for (key_index, key) in self.0.iter().enumerate() {
                key.fmt(f)?;

                if !key_index == (self.0.len() - 1) {
                    '/'.fmt(f)?;
                }
            }

            Ok(())
        }
    }
}
