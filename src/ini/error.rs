use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IniErrorKind {
    /// Invalid character at the start of the line -
    /// expected a key, section or comment.
    InvalidCharacterAtLineStart,
    /// Invalid character in section name - expected an alphanumeric character or an underscore.
    InvalidCharacterInSectionName,
    /// Empty section names are not allowed.
    EmptySectionName,
    /// Invalid character at the end of the line -
    /// expected a new line (or an inline comment if supported).
    InvalidCharacterAtLineEnd,
    /// Invalid character encountered in key name - expected an alphanumeric character or an underscore.
    InvalidCharacterInKey,
    /// Unexpected new line encountered in key name before a key-value separator.
    UnexpectedNewlineInKey,
    /// Unexpected character encountered - expected a key-value separator.
    UnexpectedCharacterInsteadOfKeyValueSeparator,
    /// Invalid character in value - expected an alphanumeric or punctuation character (except special INI characters).
    ///
    /// NOTE: these are the special INI characters:
    /// `'\''`
    /// `'"'`
    /// `'\\'`
    /// `'['`
    /// `']'`
    /// `';'`
    /// `'#'`
    /// `'='`
    /// `':'`
    InvalidCharacterInValue,
    /// Unexpected end of file encountered when parsing an escape sequence.
    UnexpectedEndOfFileInEscapeSequence,
    /// Unexpected new line encountered when parsing an escape sequence.
    ///
    /// NOTE: enable `line_continuation` in [`IniOptions`] to allow escaped new lines.
    /// [`IniOptions`]: struct.IniOptions.html
    UnexpectedNewlineInEscapeSequence,
    /// Invalid character encountered when parsing an escape sequence.
    ///
    /// NOTE: see notes for `escape` in [`IniOptions`] for a list of supported escape characters.
    /// [`IniOptions`]: struct.IniOptions.html
    InvalidEscapeCharacter,
    /// Unexpected end of file encountered when parsing a Unicode escape sequence.
    UnexpectedEndOfFileInUnicodeEscapeSequence,
    /// Unexpected new line encountered when parsing a Unicode escape sequence.
    UnexpectedNewlineInUnicodeEscapeSequence,
    /// Invalid Unicode escape sequence.
    InvalidUnicodeEscapeSequence,
    /// Duplicate key name encountered in the current section.
    DuplicateKey,
    /// Unexpected new line encountered when parsing a quoted string value.
    UnexpectedNewlineInQuotedString,
    /// Encountered an unsupported unquoted string value.
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
                "Invalid character in section name - expected an alphanumeric character or an underscore."
            ),
            EmptySectionName => write!(
                f,
                "Empty section names are not allowed."
            ),
            InvalidCharacterAtLineEnd => write!(
                f,
                "Invalid character at the end of the line - expected a new line (or an inline comment if supported)."
            ),
            InvalidCharacterInKey => write!(
                f,
                "Invalid character encountered in key name - expected an alphanumeric character or an underscore."
            ),
            UnexpectedNewlineInKey => write!(
                f,
                "Unexpected new line encountered in key name before a key-value separator."
            ),
            UnexpectedCharacterInsteadOfKeyValueSeparator => write!(
                f,
                "Unexpected character encountered - expected a key-value separator."
            ),
            InvalidCharacterInValue => write!(
                f,
                "Invalid character in value - expected an alphanumeric or punctuation character (except special INI characters)."
            ),
            UnexpectedEndOfFileInEscapeSequence => write!(
                f,
                "Unexpected end of file encountered when parsing an escape sequence."
            ),
            UnexpectedNewlineInEscapeSequence => write!(
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
            UnexpectedNewlineInUnicodeEscapeSequence => write!(
                f,
                "Unexpected new line encountered when parsing a Unicode escape sequence."
            ),
            InvalidUnicodeEscapeSequence => write!(
                f,
                "Invalid Unicode escape sequence."
            ),
            DuplicateKey => write!(
                f,
                "Duplicate key name encountered in the current section."
            ),
            UnexpectedNewlineInQuotedString => write!(
                f,
                "Unexpected new line encountered when parsing a quoted string value."
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
