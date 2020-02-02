#![allow(non_upper_case_globals)]

use bitflags::bitflags;

bitflags! {
    pub struct IniCommentSeparator: u8 {
        const None = 0b00;
        /// `;`
        const Semicolon = 0b01;
        /// `#`
        const NumberSign = 0b10;
    }
}

bitflags! {
    pub struct IniKeyValueSeparator: u8 {
        /// `=`
        const Equals = 0b01;
        /// `:`
        const Colon = 0b10;
    }
}

bitflags! {
    pub struct IniStringQuote: u8 {
        const None = 0b00;
        /// `'`
        const Single = 0b01;
        /// `"`
        const Double = 0b10;
    }
}

/// Configuration options for the INI parser.
pub struct IniOptions {
    /// Valid comment separator character(s).
    /// If `None`, comments are not supported.
    /// Default: `Semicolon`.
    pub comments: IniCommentSeparator,
    /// Whether inline (i.e. not beginning at the start of the line) comments are supported.
    /// If `comments` is `None`, this value is ignored.
    /// Default: `false`.
    pub inline_comments: bool,
    /// Valid key-value separator character(s).
    /// If no flag is set, `Equals` is assumed.
    /// Default: `Equals`.
    pub key_value_separator: IniKeyValueSeparator,
    /// Valid string value quote character(s).
    /// If `None`, quoted strings are not supported.
    /// In this case all values will be parsed as booleans / integers / floats / strings, in order.
    /// E.g., the value `true` is always interpreted as a boolean.
    /// Default: `Double`.
    pub string_quotes: IniStringQuote,
    /// Whether unquoted string values are supported.
    /// If `false`, an unquoted value must parse as a boolean / integer / float, or an error will be raised.
    /// If `string_quotes` is `None`, this value is ignored.
    /// Default: `true`.
    pub unquoted_strings: bool,
    /// Whether escape sequences (a character sequence following a backslash ('\'))
    /// in string values are supported.
    /// If `true`, the following escape sequences are supported:
    ///     `'\0'`
    ///     `'\a'`
    ///     `'\b'`
    ///     `'\t'`
    ///     `'\r'`
    ///     `'\n'`
    ///     `'\\'`
    ///     `'\['`
    ///     `'\]'`
    ///     `'\;'`
    ///     `'\#'`
    ///     `'\='`
    ///     `'\:'`
    ///     `'\x????'` (where `?` are 4 hexadecimal digits)
    /// Default: `true`.
    pub escape: bool,
    /// Whether line ontinuation esacpe sequences (a backslash followed by a newline)
    /// are supported in string values.
    /// If `escape` is `false`, this value is ignored.
    /// Default: `false`.
    pub line_continuation: bool,
    /// Whether to allow duplicate keys in sections.
    /// If `true`, later keys overwrite the prior.
    /// Default: `false`.
    pub duplicate_keys: bool,
}

impl Default for IniOptions {
    fn default() -> Self {
        Self {
            comments: IniCommentSeparator::Semicolon,
            inline_comments: false,
            key_value_separator: IniKeyValueSeparator::Equals,
            string_quotes: IniStringQuote::Double,
            unquoted_strings: true,
            escape: true,
            line_continuation: false,
            duplicate_keys: false,
        }
    }
}
