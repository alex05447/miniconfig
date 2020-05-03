use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IniErrorKind {
    /// Invalid character at the start of the line -
    /// expected a section name, key or line comment.
    InvalidCharacterAtLineStart,
    /// Invalid character encountered when parsing a section name.
    InvalidCharacterInSectionName,
    /// Invalid character encountered after section name - expected whitespace or a section end delimiter.
    InvalidCharacterAfterSectionName,
    /// Unexpected new line encountered when parsing a section name.
    UnexpectedNewLineInSectionName,
    /// Unexpected end of file encountered when parsing a section name.
    UnexpectedEndOfFileInSectionName,
    /// Empty section names are invalid.
    EmptySectionName,
    /// Duplicate section name encountered and is not allowed by options.
    DuplicateSection,
    /// Invalid character encountered at the end of the line.
    InvalidCharacterAtLineEnd,
    /// Invalid character encountered when parsing the key name.
    InvalidCharacterInKey,
    /// Unexpected new line encountered before a key-value separator.
    UnexpectedNewLineInKey,
    /// Empty keys are invalid.
    EmptyKey,
    /// Duplicate key name encountered and is not allowed by options.
    DuplicateKey,
    /// Unexpected end of file encountered before a key-value separator.
    UnexpectedEndOfFileBeforeKeyValueSeparator,
    /// Unexpected character encountered when parsing a key-value separator.
    UnexpectedCharacterInsteadOfKeyValueSeparator,
    /// Invalid character encountered when parsing a value.
    InvalidCharacterInValue,
    /// Unexpected end of file encountered when parsing an escape sequence.
    UnexpectedEndOfFileInEscapeSequence,
    /// Unexpected new line encountered when parsing an escape sequence;
    /// line continuations are not allowed by options.
    UnexpectedNewLineInEscapeSequence,
    /// Invalid character encountered when parsing an escape sequence.
    InvalidEscapeCharacter,
    /// Unexpected end of file encountered when parsing a Unicode escape sequence.
    UnexpectedEndOfFileInUnicodeEscapeSequence,
    /// Unexpected new line encountered when parsing a Unicode escape sequence.
    UnexpectedNewLineInUnicodeEscapeSequence,
    /// Invalid Unicode escape sequence.
    InvalidUnicodeEscapeSequence,
    /// Unexpected new line encountered when parsing a quoted string value.
    UnexpectedNewLineInQuotedValue,
    /// Unexpected end of file encountered when parsing a quoted string value.
    UnexpectedEndOfFileInQuotedString,
    /// Encountered an unquoted string value, not allowed by options.
    UnquotedString,
    /// Unexpected new line encountered when parsing an array.
    UnexpectedNewLineInArray,
    /// Mixed value types encountered when parsing an array.
    MixedArray,
    /// Invalid character in array.
    InvalidCharacterInArray,
    /// Unexpected end of file encountered when parsing an array.
    UnexpectedEndOfFileInArray,
    /// Unexpected end of file encountered when parsing a quoted array value.
    UnexpectedEndOfFileInQuotedArrayValue,
}

/// An error returned by the INI parser.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
            InvalidCharacterAtLineStart => write!(
                f, "Invalid character at the start of the line - expected a section name, key or line comment."
            ),
            InvalidCharacterInSectionName => write!(
                f,
                "Invalid character encountered when parsing a section name."
            ),
            InvalidCharacterAfterSectionName => write!(
                f,
                "Invalid character encountered after section name - expected whitespace or a section end delimiter."
            ),
            UnexpectedNewLineInSectionName => write!(
                f,
                "Unexpected new line encountered when parsing a section name."
            ),
            UnexpectedEndOfFileInSectionName => write!(
                f,
                "Unexpected end of file encountered when parsing a section name."
            ),
            EmptySectionName => write!(
                f,
                "Empty section names are invalid."
            ),
            DuplicateSection => write!(
                f,
                "Duplicate section name encountered and is not allowed by options."
            ),
            InvalidCharacterAtLineEnd => write!(
                f,
                "Invalid character at the end of the line."
            ),
            InvalidCharacterInKey => write!(
                f,
                "Invalid character encountered when parsing the key name."
            ),
            UnexpectedNewLineInKey => write!(
                f,
                "Unexpected new line encountered before a key-value separator."
            ),
            EmptyKey => write!(
                f,
                "Empty keys are invalid."
            ),
            DuplicateKey => write!(
                f,
                "Duplicate key name encountered and is not allowed by options."
            ),
            UnexpectedEndOfFileBeforeKeyValueSeparator => write!(
                f,
                "Unexpected character encountered when parsing a key-value separator."
            ),
            UnexpectedCharacterInsteadOfKeyValueSeparator => write!(
                f,
                "Unexpected character encountered - expected a key-value separator."
            ),
            InvalidCharacterInValue => write!(
                f,
                "Invalid character encountered when parsing a value."
            ),
            UnexpectedEndOfFileInEscapeSequence => write!(
                f,
                "Unexpected end of file encountered when parsing an escape sequence."
            ),
            UnexpectedNewLineInEscapeSequence => write!(
                f,
                "Unexpected new line encountered when parsing an escape sequence; line continuations are not allowed by options."
            ),
            InvalidEscapeCharacter => write!(
                f,
                "Invalid character encountered when parsing an escape sequence."
            ),
            UnexpectedEndOfFileInUnicodeEscapeSequence => write!(
                f,
                "Unexpected end of file encountered when parsing a Unicode escape sequence."
            ),
            UnexpectedNewLineInUnicodeEscapeSequence => write!(
                f,
                "Unexpected new line encountered when parsing a Unicode escape sequence."
            ),
            InvalidUnicodeEscapeSequence => write!(
                f,
                "Invalid Unicode escape sequence."
            ),
            UnexpectedNewLineInQuotedValue => write!(
                f,
                "Unexpected new line encountered when parsing a quoted string value."
            ),
            UnexpectedEndOfFileInQuotedString => write!(
                f,
                "Unexpected end of file encountered when parsing a quoted string value."
            ),
            UnquotedString => write!(
                f,
                "Encountered an unquoted string value, not allowed by options."
            ),
            UnexpectedNewLineInArray => write!(
                f,
                "Unexpected new line encountered when parsing an array."
            ),
            MixedArray => write!(
                f,
                "Mixed value types encountered when parsing an array."
            ),
            InvalidCharacterInArray => write!(
                f,
                "Invalid character in array.",
            ),
            UnexpectedEndOfFileInArray => write!(
                f,
                "Unexpected end of file encountered when parsing an array.",
            ),
            UnexpectedEndOfFileInQuotedArrayValue => write!(
                f,
                "Unexpected end of file encountered when parsing a quoted array value.",
            ),
        }
    }
}

impl Display for IniError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "INI parse error. Line: {}, column: {}. {}",
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
    /// Tables nested within tables are not supported in INI configs.
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
                "Tables nested within tables are not supported in INI configs."
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
