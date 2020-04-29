use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IniErrorKind {
    /// Invalid character at the start of the line -
    /// expected a key, section or comment.
    InvalidCharacterAtLineStart,
    /// Invalid character in section name - expected a valid string character or an escape sequence.
    InvalidCharacterInSectionName,
    /// Unexpected new line in section name.
    UnexpectedNewLineInSectionName,
    /// Unexpected end of file in section name.
    UnexpectedEndOfFileInSectionName,
    /// Empty section names are not allowed.
    EmptySectionName,
    /// Duplicate section name encountered.
    DuplicateSectionName,
    /// Invalid character at the end of the line - expected a new line (or an inline comment if supported).
    InvalidCharacterAtLineEnd,
    /// Invalid character in key name - expected a valid string character or an escape sequence.
    InvalidCharacterInKey,
    /// Unexpected new line encountered in key name before a key-value separator.
    UnexpectedNewLineInKey,
    /// Empty keys are not allowed.
    EmptyKey,
    /// Duplicate key name encountered in the current section.
    DuplicateKey,
    /// Unexpected end of file encountered before a key-value separator.
    UnexpectedEndOfFileBeforeKeyValueSeparator,
    /// Unexpected character encountered - expected a key-value separator.
    UnexpectedCharacterInsteadOfKeyValueSeparator,
    /// Invalid character in value - expected a valid string character or an escape sequence.
    InvalidCharacterInValue,
    /// Unexpected end of file encountered when parsing an escape sequence.
    UnexpectedEndOfFileInEscapeSequence,
    /// Unexpected new line encountered when parsing an escape sequence.
    ///
    /// NOTE: enable `line_continuation` in [`IniOptions`] to allow escaped new lines.
    /// [`IniOptions`]: struct.IniOptions.html
    UnexpectedNewLineInEscapeSequence,
    /// Invalid character encountered when parsing an escape sequence.
    ///
    /// NOTE: see notes for `escape` in [`IniOptions`] for a list of supported escape characters.
    /// [`IniOptions`]: struct.IniOptions.html
    InvalidEscapeCharacter,
    /// Unexpected end of file encountered when parsing a Unicode escape sequence.
    UnexpectedEndOfFileInUnicodeEscapeSequence,
    /// Unexpected new line encountered when parsing a Unicode escape sequence.
    UnexpectedNewLineInUnicodeEscapeSequence,
    /// Invalid Unicode escape sequence.
    InvalidUnicodeEscapeSequence,
    /// Unexpected new line encountered when parsing a quoted string value.
    UnexpectedNewLineInQuotedString,
    /// Unexpected end of file encountered when parsing a quoted string value.
    UnexpectedEndOfFileInQuotedString,
    /// Encountered an unsupported unquoted string value.
    ///
    /// NOTE: enable `unquoted_strings` in [`IniOptions`] to allow unquoted string values.
    /// [`IniOptions`]: struct.IniOptions.html
    UnquotedString,
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
                f, "Invalid character at the start of the line - expected a key, section or comment."
            ),
            InvalidCharacterInSectionName => write!(
                f,
                "Invalid character in section name - expected a valid string character or an escape sequence."
            ),
            UnexpectedNewLineInSectionName => write!(
                f,
                "Unexpected new line in section name."
            ),
            UnexpectedEndOfFileInSectionName => write!(
                f,
                "Unexpected end of file in section name."
            ),
            EmptySectionName => write!(
                f,
                "Empty section names are not allowed."
            ),
            DuplicateSectionName => write!(
                f,
                "Duplicate section name encountered."
            ),
            InvalidCharacterAtLineEnd => write!(
                f,
                "Invalid character at the end of the line - expected a new line (or an inline comment if supported)."
            ),
            InvalidCharacterInKey => write!(
                f,
                "Invalid character in key name - expected a valid string character or an escape sequence."
            ),
            UnexpectedNewLineInKey => write!(
                f,
                "Unexpected new line encountered in key name before a key-value separator."
            ),
            EmptyKey => write!(
                f,
                "Empty keys are not allowed."
            ),
            DuplicateKey => write!(
                f,
                "Duplicate key name encountered in the current section."
            ),
            UnexpectedEndOfFileBeforeKeyValueSeparator => write!(
                f,
                "Unexpected end of file encountered before a key-value separator."
            ),
            UnexpectedCharacterInsteadOfKeyValueSeparator => write!(
                f,
                "Unexpected character encountered - expected a key-value separator."
            ),
            InvalidCharacterInValue => write!(
                f,
                "Invalid character in value - expected a valid string character or an escape sequence."
            ),
            UnexpectedEndOfFileInEscapeSequence => write!(
                f,
                "Unexpected end of file encountered when parsing an escape sequence."
            ),
            UnexpectedNewLineInEscapeSequence => write!(
                f,
                "Unexpected new line encountered when parsing an escape sequence."
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
            UnexpectedNewLineInQuotedString => write!(
                f,
                "Unexpected new line encountered when parsing a quoted string value."
            ),
            UnexpectedEndOfFileInQuotedString => write!(
                f,
                "Unexpected end of file encountered when parsing a quoted string value."
            ),
            UnquotedString => write!(
                f,
                "Encountered an unsupported unquoted string value."
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
pub enum ToINIStringError {
    /// Array values are not supported in INI configs.
    ArraysNotSupported,
    /// Tables nested within tables are not supported in INI configs.
    NestedTablesNotSupported,
    /// General write error (out of memory?).
    WriteError,
}

impl Display for ToINIStringError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use ToINIStringError::*;

        match self {
            ArraysNotSupported => write!(f, "Array values are not supported in INI configs."),
            NestedTablesNotSupported => write!(
                f,
                "Tables nested within tables are not supported in INI configs."
            ),
            WriteError => write!(f, "General write error (out of memory?)."),
        }
    }
}
