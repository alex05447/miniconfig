use std::{
    error::Error,
    fmt::{Display, Formatter},
};

/// An actual concrete error kind returned by the [`.ini parser`](struct.IniParser.html).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum IniErrorKind {
    /// Invalid character at the start of the line -
    /// expected a section name, key or line comment.
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
    /// Duplicate section name encountered and is not allowed by options.
    /// Contains the duplicate section name.
    DuplicateSection(String),
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
    /// Contains the duplicate key.
    DuplicateKey(String),
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
    /// Unexpected end of file in a Unicode escape sequence.
    UnexpectedEndOfFileInUnicodeEscapeSequence,
    /// Unexpected new line in a Unicode escape sequence.
    UnexpectedNewLineInUnicodeEscapeSequence,
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
                f, "invalid character ('{}') at the start of the line - expected a section name, key or line comment", c
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
            DuplicateSection(s) => write!(
                f,
                "duplicate section name (\"{}\") encountered and is not allowed by options", s
            ),
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
            DuplicateKey(k) => write!(
                f,
                "duplicate key (\"{}\") encountered and is not allowed by options", k
            ),
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
            UnexpectedEndOfFileInUnicodeEscapeSequence => "unexpected end of file in a Unicode escape sequence".fmt(f),
            UnexpectedNewLineInUnicodeEscapeSequence => "unexpected new line in a Unicode escape sequence".fmt(f),
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
    /// Actual error.
    pub error: IniErrorKind,
}

impl Error for IniError {}

impl Display for IniError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "`.ini` parse error; line: {}, column: {}, error: {}",
            self.line, self.column, self.error
        )
    }
}

/// An error returned by `to_ini_string`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToIniStringError {
    /// Array values are not allowed by options.
    ArraysNotAllowed,
    /// Only boolean, number and string arrays are supported.
    InvalidArrayType,
    /// Tables nested within tables are not supported in `.ini` configs.
    NestedTablesNotSupported,
    /// General write error (out of memory?).
    WriteError,
    /// Encountered an escaped character not allowed by options.
    /// Contains the escaped character.
    EscapedCharacterNotAllowed(char),
}

impl Error for ToIniStringError {}

impl Display for ToIniStringError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use ToIniStringError::*;

        match self {
            ArraysNotAllowed => "array values are not allowed by options".fmt(f),
            InvalidArrayType => "only boolean, number and string arrays are supported".fmt(f),
            NestedTablesNotSupported => {
                "tables nested within tables are not supported in `.ini` configs".fmt(f)
            }
            WriteError => "general write error (out of memory?)".fmt(f),
            EscapedCharacterNotAllowed(c) => write!(
                f,
                "encountered an escaped character not allowed by options: \'{}\'",
                c
            ),
        }
    }
}
