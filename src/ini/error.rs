use std::fmt::{Display, Formatter};

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
    /// Unexpected new line encountered in section name.
    UnexpectedNewLineInSectionName,
    /// Unexpected end of file encountered in section name.
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
    /// Unexpected end of file encountered in an escape sequence.
    UnexpectedEndOfFileInEscapeSequence,
    /// Unexpected new line encountered in an escape sequence.
    UnexpectedNewLineInEscapeSequence,
    /// Invalid character in an escape sequence.
    /// Contains the invalid character.
    InvalidEscapeCharacter(char),
    /// Unexpected end of file encountered in a Unicode escape sequence.
    UnexpectedEndOfFileInUnicodeEscapeSequence,
    /// Unexpected new line encountered in a Unicode escape sequence.
    UnexpectedNewLineInUnicodeEscapeSequence,
    /// Invalid Unicode escape sequence.
    InvalidUnicodeEscapeSequence,
    /// Unexpected new line in a quoted string value.
    UnexpectedNewLineInQuotedValue,
    /// Unexpected end of file encountered in a quoted string value.
    UnexpectedEndOfFileInQuotedString,
    /// Encountered an unquoted string value, not allowed by options.
    UnquotedString,
    /// Unexpected new line encountered in an array.
    UnexpectedNewLineInArray,
    /// Mixed value types encountered when parsing an array.
    MixedArray,
    /// Invalid character in an array.
    /// Contains the invalid character.
    InvalidCharacterInArray(char),
    /// Unexpected end of file encountered in an array.
    UnexpectedEndOfFileInArray,
    /// Unexpected end of file encountered in a quoted array value.
    UnexpectedEndOfFileInQuotedArrayValue,
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

impl Display for IniErrorKind {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use IniErrorKind::*;

        match self {
            InvalidCharacterAtLineStart(c) => write!(
                f, "Invalid character ('{}') at the start of the line - expected a section name, key or line comment.", c
            ),
            InvalidCharacterInSectionName(c) => write!(
                f,
                "Invalid character ('{}') in section name.", c
            ),
            InvalidCharacterAfterSectionName(c) => write!(
                f,
                "Invalid character ('{}') after section name - expected whitespace or a section end delimiter.", c
            ),
            UnexpectedNewLineInSectionName => write!(
                f,
                "Unexpected new line encountered in section name."
            ),
            UnexpectedEndOfFileInSectionName => write!(
                f,
                "Unexpected end of file encountered in section name."
            ),
            EmptySectionName => write!(
                f,
                "Empty section names are invalid."
            ),
            DuplicateSection(s) => write!(
                f,
                "Duplicate section name (\"{}\") encountered and is not allowed by options.", s
            ),
            InvalidCharacterAtLineEnd(c) => write!(
                f,
                "Invalid character ('{}') at the end of the line - expected whitespace or an inline comment (if supported).", c
            ),
            InvalidCharacterInKey(c) => write!(
                f,
                "Invalid character ('{}') in the key name.", c
            ),
            UnexpectedNewLineInKey => write!(
                f,
                "Unexpected new line encountered before a key-value separator."
            ),
            EmptyKey => write!(
                f,
                "Empty keys are invalid."
            ),
            DuplicateKey(k) => write!(
                f,
                "Duplicate key (\"{}\") encountered and is not allowed by options.", k
            ),
            UnexpectedEndOfFileBeforeKeyValueSeparator => write!(
                f,
                "Unexpected character encountered when parsing a key-value separator."
            ),
            InvalidKeyValueSeparator(c) => write!(
                f,
                "Invalid character ('{}') encountered instead of the key-value separator.", c
            ),
            InvalidCharacterInValue(c) => write!(
                f,
                "Invalid character ('{}') in value.", c
            ),
            UnexpectedEndOfFileInEscapeSequence => write!(
                f,
                "Unexpected end of file encountered in an escape sequence."
            ),
            UnexpectedNewLineInEscapeSequence => write!(
                f,
                "Unexpected new line encountered in an escape sequence."
            ),
            InvalidEscapeCharacter(c) => write!(
                f,
                "Invalid character ('{}') in an escape sequence.", c
            ),
            UnexpectedEndOfFileInUnicodeEscapeSequence => write!(
                f,
                "Unexpected end of file encountered in a Unicode escape sequence."
            ),
            UnexpectedNewLineInUnicodeEscapeSequence => write!(
                f,
                "Unexpected new line encountered in a Unicode escape sequence."
            ),
            InvalidUnicodeEscapeSequence => write!(
                f,
                "Invalid Unicode escape sequence."
            ),
            UnexpectedNewLineInQuotedValue => write!(
                f,
                "Unexpected new line in a quoted string value."
            ),
            UnexpectedEndOfFileInQuotedString => write!(
                f,
                "Unexpected end of file encountered in a quoted string value."
            ),
            UnquotedString => write!(
                f,
                "Encountered an unquoted string value, not allowed by options."
            ),
            UnexpectedNewLineInArray => write!(
                f,
                "Unexpected new line encountered in an array."
            ),
            MixedArray => write!(
                f,
                "Mixed value types encountered when parsing an array."
            ),
            InvalidCharacterInArray(c) => write!(
                f,
                "Invalid character ('{}') in an array.", c
            ),
            UnexpectedEndOfFileInArray => write!(
                f,
                "Unexpected end of file encountered in an array.",
            ),
            UnexpectedEndOfFileInQuotedArrayValue => write!(
                f,
                "Unexpected end of file encountered in a quoted array value.",
            ),
        }
    }
}

impl Display for IniError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "`.ini` parse error. Line: {}, column: {}. {}",
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

impl Display for ToIniStringError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use ToIniStringError::*;

        match self {
            ArraysNotAllowed => write!(f, "Array values are not allowed by options."),
            InvalidArrayType => write!(f, "Only boolean, number and string arrays are supported."),
            NestedTablesNotSupported => write!(
                f,
                "Tables nested within tables are not supported in `.ini` configs."
            ),
            WriteError => write!(f, "General write error (out of memory?)."),
            EscapedCharacterNotAllowed(c) => write!(
                f,
                "Encountered an escaped character not allowed by options (\'{}\')",
                c
            ),
        }
    }
}
