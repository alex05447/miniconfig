use {
    crate::*,
    std::{
        error::Error,
        fmt::{Display, Formatter},
    },
};

/// An actual concrete error kind returned by the [`.ini parser`](struct.IniParser.html).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum IniErrorKind {
    /// Invalid character at the start of the line -
    /// expected a key, section name (if supported), or line comment (if supported).
    /// Contains the invalid character.
    InvalidCharacterAtLineStart(char),
    /// Invalid character in section name.
    /// Contains the invalid character.
    InvalidCharacterInSectionName(char),
    /// Invalid character after section name - expected whitespace or a section end delimiter.
    /// Contains the invalid character.
    InvalidCharacterAfterSectionName(char),
    /// Unexpected new line in a section name.
    UnexpectedNewLineInSectionName,
    /// Unexpected end of file in a section name.
    UnexpectedEndOfFileInSectionName,
    /// Empty section names are invalid.
    EmptySectionName,
    /// Invalid (missing or not a section) parent section name.
    InvalidParentSection,
    /// Maximum allowed nested section depth exceeded.
    NestedSectionDepthExceeded,
    /// Duplicate section name encountered and is not allowed by options.
    DuplicateSection,
    /// Invalid character at the end of the line - expected whitespace or an inline comment (if supported).
    /// Contains the invalid character.
    InvalidCharacterAtLineEnd(char),
    /// Invalid character in the key name.
    /// Contains the invalid character.
    InvalidCharacterInKey(char),
    /// Unexpected new line encountered before a key-value separator.
    UnexpectedNewLineInKey,
    /// Empty keys are invalid.
    EmptyKey,
    /// Duplicate key encountered and is not allowed by options.
    DuplicateKey,
    /// Unexpected end of file encountered before a key-value separator.
    UnexpectedEndOfFileBeforeKeyValueSeparator,
    /// Invalid character encountered instead of the key-value separator.
    /// Contains the invalid character.
    InvalidKeyValueSeparator(char),
    /// Invalid character in value.
    /// Contains the invalid character.
    InvalidCharacterInValue(char),
    /// Unexpected end of file in an escape sequence.
    UnexpectedEndOfFileInEscapeSequence,
    /// Unexpected new line in an escape sequence.
    UnexpectedNewLineInEscapeSequence,
    /// Invalid character in an escape sequence.
    /// Contains the invalid character.
    InvalidEscapeCharacter(char),
    /// Unexpected end of file in an ASCII escape sequence.
    UnexpectedEndOfFileInASCIIEscapeSequence,
    /// Unexpected end of file in a Unicode escape sequence.
    UnexpectedEndOfFileInUnicodeEscapeSequence,
    /// Unexpected new line in an ASCII escape sequence.
    UnexpectedNewLineInASCIIEscapeSequence,
    /// Unexpected new line in a Unicode escape sequence.
    UnexpectedNewLineInUnicodeEscapeSequence,
    /// Invalid ASCII escape sequence.
    InvalidASCIIEscapeSequence,
    /// Invalid Unicode escape sequence.
    InvalidUnicodeEscapeSequence,
    /// Unexpected new line in a quoted string value.
    UnexpectedNewLineInQuotedValue,
    /// Unexpected end of file in a quoted string value.
    UnexpectedEndOfFileInQuotedString,
    /// Encountered an unquoted string value, not allowed by options.
    UnquotedString,
    /// Unexpected new line in an array.
    UnexpectedNewLineInArray,
    /// Mixed value types encountered in an array.
    MixedArray,
    /// Invalid character in an array.
    /// Contains the invalid character.
    InvalidCharacterInArray(char),
    /// Unexpected end of file in an array.
    UnexpectedEndOfFileInArray,
    /// Unexpected end of file in a quoted array value.
    UnexpectedEndOfFileInQuotedArrayValue,
}

impl Display for IniErrorKind {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use IniErrorKind::*;

        match self {
            InvalidCharacterAtLineStart(c) => write!(
                f, "invalid character ('{}') at the start of the line - expected a key, section name (if supported), or line comment (if supported)", c
            ),
            InvalidCharacterInSectionName(c) => write!(
                f,
                "invalid character ('{}') in section name", c
            ),
            InvalidCharacterAfterSectionName(c) => write!(
                f,
                "invalid character ('{}') after section name - expected whitespace or a section end delimiter", c
            ),
            UnexpectedNewLineInSectionName => "unexpected new line in a section name".fmt(f),
            UnexpectedEndOfFileInSectionName => "unexpected end of file in a section name".fmt(f),
            EmptySectionName => "empty section names are invalid".fmt(f),
            InvalidParentSection => "invalid (missing or not a section) parent section".fmt(f),
            NestedSectionDepthExceeded => write!(f, "maximum allowed nested section depth exceeded"),
            DuplicateSection => "duplicate section name encountered and is not allowed by options".fmt(f),
            InvalidCharacterAtLineEnd(c) => write!(
                f,
                "invalid character ('{}') at the end of the line - expected whitespace or an inline comment (if supported)", c
            ),
            InvalidCharacterInKey(c) => write!(
                f,
                "invalid character ('{}') in the key name", c
            ),
            UnexpectedNewLineInKey => "unexpected new line encountered before a key-value separator".fmt(f),
            EmptyKey => "empty keys are invalid".fmt(f),
            DuplicateKey => "duplicate key encountered and is not allowed by options".fmt(f),
            UnexpectedEndOfFileBeforeKeyValueSeparator => "unexpected end of file encountered before a key-value separator".fmt(f),
            InvalidKeyValueSeparator(c) => write!(
                f,
                "invalid character ('{}') encountered instead of the key-value separator", c
            ),
            InvalidCharacterInValue(c) => write!(
                f,
                "invalid character ('{}') in value", c
            ),
            UnexpectedEndOfFileInEscapeSequence => "unexpected end of file in an escape sequence".fmt(f),
            UnexpectedNewLineInEscapeSequence => "unexpected new line in an escape sequence".fmt(f),
            InvalidEscapeCharacter(c) => write!(
                f,
                "invalid character ('{}') in an escape sequence", c
            ),
            UnexpectedEndOfFileInASCIIEscapeSequence => "unexpected end of file in an ASCII escape sequence".fmt(f),
            UnexpectedEndOfFileInUnicodeEscapeSequence => "unexpected end of file in a Unicode escape sequence".fmt(f),
            UnexpectedNewLineInASCIIEscapeSequence => "unexpected new line in an ASCII escape sequence".fmt(f),
            UnexpectedNewLineInUnicodeEscapeSequence => "unexpected new line in a Unicode escape sequence".fmt(f),
            InvalidASCIIEscapeSequence => "invalid ASCII escape sequence".fmt(f),
            InvalidUnicodeEscapeSequence => "invalid Unicode escape sequence".fmt(f),
            UnexpectedNewLineInQuotedValue => "unexpected new line in a quoted string value".fmt(f),
            UnexpectedEndOfFileInQuotedString => "unexpected end of file in a quoted string value".fmt(f),
            UnquotedString => "encountered an unquoted string value, not allowed by options".fmt(f),
            UnexpectedNewLineInArray => "unexpected new line in an array".fmt(f),
            MixedArray => "mixed value types encountered in an array".fmt(f),
            InvalidCharacterInArray(c) => write!(
                f,
                "invalid character ('{}') in an array", c
            ),
            UnexpectedEndOfFileInArray => "unexpected end of file in an array".fmt(f),
            UnexpectedEndOfFileInQuotedArrayValue => "unexpected end of file in a quoted array value".fmt(f),
        }
    }
}

/// An error returned by the [`.ini parser`](struct.IniParser.html).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IniError {
    /// Line in the source string where the error occured.
    pub line: u32,
    /// Column in the source string where the error occured.
    pub column: u32,
    /// Path to the key / value in which the error happened, or an empty path for the root section.
    pub path: ConfigPath,
    /// Actual error.
    pub error: IniErrorKind,
}

impl Error for IniError {}

impl Display for IniError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "`.ini` parse error; line: {}, column: {}, path: {}, error: {}",
            self.line, self.column, self.path, self.error
        )
    }
}

/// An error returned by `to_ini_string` / `fmt_ini` methods on [`bin`], [`dyn`] and [`lua`] configs.
///
/// [`bin`]: struct.BinConfig.html#method.to_ini_string
/// [`dyn`]: struct.DynConfig.html#method.to_ini_string
/// [`lua`]: struct.LuaConfig.html#method.to_ini_string
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToIniStringError {
    /// Array values are not allowed by options.
    ArraysNotAllowed,
    /// Only boolean, number and string arrays are supported.
    InvalidArrayType,
    /// Maximum allowed nested section depth exceeded.
    NestedSectionDepthExceeded,
    /// Encountered an escaped character not allowed by options.
    /// Contains the escaped character.
    EscapedCharacterNotAllowed(char),
    /// General write error.
    WriteError,
}

impl From<std::io::Error> for ToIniStringError {
    fn from(_: std::io::Error) -> Self {
        Self::WriteError
    }
}

impl From<std::fmt::Error> for ToIniStringError {
    fn from(_: std::fmt::Error) -> Self {
        Self::WriteError
    }
}

impl Error for ToIniStringError {}

impl Display for ToIniStringError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use ToIniStringError::*;

        match self {
            ArraysNotAllowed => "array values are not allowed by options".fmt(f),
            InvalidArrayType => "only boolean, number and string arrays are supported".fmt(f),
            NestedSectionDepthExceeded => "maximum allowed nested section depth exceeded".fmt(f),
            WriteError => "general write error".fmt(f),
            EscapedCharacterNotAllowed(c) => write!(
                f,
                "encountered an escaped character not allowed by options: \'{}\'",
                c
            ),
        }
    }
}
