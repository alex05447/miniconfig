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

/// Controls how duplicate sections, if any, are handled in the INI config.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IniDuplicateSections {
    /// Do not allow duplicate sections.
    Forbid,
    /// Use the first encountered instance of the section,
    /// skip all following ones.
    First,
    /// Use the last encountered instance of the section,
    /// overwriting all prior, if any.
    Last,
    /// Merge all encountered instances of the section into one.
    Merge,
}

/// Controls how duplicate keys, if any, are handled in the root / sections of the INI config.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IniDuplicateKeys {
    /// Do not allow duplicate keys.
    Forbid,
    /// Use the first encountered instance of the key in the root / section,
    /// skip all following ones.
    First,
    /// Use the last encountered instance of the key in the root / section,
    /// overwriting all prior, if any.
    Last,
}

impl IniDuplicateKeys {
    pub(crate) fn allow_non_unique(self) -> bool {
        match self {
            Self::Forbid => false,
            Self::First | Self::Last => true,
        }
    }
}

/// Configuration options for the INI parser.
#[derive(Clone, Copy, Debug)]
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
    /// in keys, section names and string values are supported.
    /// If `true`, the following escape sequences are supported:
    ///     `' '` (space),
    ///     `'"'`,
    ///     `'\''`,
    ///     `'\0'`,
    ///     `'\a'`,
    ///     `'\b'`,
    ///     `'\t'`,
    ///     `'\r'`,
    ///     `'\n'`,
    ///     `'\v'`,
    ///     `'\f'`,
    ///     `'\\'`,
    ///     `'\['`,
    ///     `'\]'`,
    ///     `'\;'`,
    ///     `'\#'`,
    ///     `'\='`,
    ///     `'\:'`,
    ///     `'\x????'` (where `?` are 4 hexadecimal digits).
    /// If `false`, backslash ('\') is treated as a normal section name / key / value character.
    /// Default: `true`.
    pub escape: bool,
    /// Whether line ontinuation esacpe sequences (a backslash followed by a newline)
    /// are supported in keys, section names and string values.
    /// If `escape` is `false`, this value is ignored.
    /// Default: `false`.
    pub line_continuation: bool,
    /// Duplicate section handling policy.
    /// Default: `Merge`.
    pub duplicate_sections: IniDuplicateSections,
    /// Duplicate key handling policy.
    /// Default: `Forbid`.
    pub duplicate_keys: IniDuplicateKeys,
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
            duplicate_sections: IniDuplicateSections::Merge,
            duplicate_keys: IniDuplicateKeys::Forbid,
        }
    }
}

/// Configuration options for serializing a config to an `.ini` string.
#[derive(Clone, Copy, Debug)]
pub struct ToIniStringOptions {
    /// See [`IniOptions`](struct.IniOptions.html)::`escape`.
    pub escape: bool,
}

impl Default for ToIniStringOptions {
    fn default() -> Self {
        Self { escape: true }
    }
}
