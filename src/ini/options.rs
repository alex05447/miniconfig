#![allow(non_upper_case_globals)]

use bitflags::bitflags;

bitflags! {
    /// Flags which specify which characters are valid `.ini` config comment delimiters.
    pub struct IniCommentDelimiter: u8 {
        /// Comments not supported.
        const None = 0b00;
        /// `;`
        const Semicolon = 0b01;
        /// `#`
        const NumberSign = 0b10;
    }
}

bitflags! {
    /// Flags which specify which characters are valid `.ini` config key / value separators.
    pub struct IniKeyValueSeparator: u8 {
        /// `=`
        const Equals = 0b01;
        /// `:`
        const Colon = 0b10;
    }
}

bitflags! {
    /// Flags which specify which characters are valid `.ini` config quoted string delimiters.
    pub struct IniStringQuote: u8 {
        /// Quoted strings not supported.
        const None = 0b00;
        /// `'`
        const Single = 0b01;
        /// `"`
        const Double = 0b10;
    }
}

/// Controls how duplicate sections, if any, are handled in the `.ini` config.
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

/// Controls how duplicate keys, if any, are handled in the root / sections of the `.ini` config.
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

/// Configuration options for the `.ini` parser.
#[derive(Clone, Copy, Debug)]
pub(crate) struct IniOptions {
    /// Valid comment delimiter character(s).
    /// If [`None`](struct.IniCommentDelimiter.html#associatedconstant.None), comments are not supported.
    ///
    /// Default: [`Semicolon`](struct.IniCommentDelimiter.html#associatedconstant.Semicolon).
    pub(crate) comments: IniCommentDelimiter,
    /// Whether inline comments (i.e. those which don't begin at the start of the line) are supported.
    /// If `comments` is [`None`](struct.IniCommentDelimiter.html#associatedconstant.None), this value is ignored.
    ///
    /// Default: `false`.
    pub(crate) inline_comments: bool,
    /// Valid key-value separator character(s).
    /// If no flag is set, [`Equals`](struct.IniKeyValueSeparator.html#associatedconstant.Equals) is assumed.
    ///
    /// Default: [`Equals`](struct.IniKeyValueSeparator.html#associatedconstant.Equals).
    pub(crate) key_value_separator: IniKeyValueSeparator,
    /// Valid string value quote character(s).
    /// If [`None`](struct.IniStringQuote.html#associatedconstant.None), quoted strings are not supported.
    /// In this case all values will be parsed as booleans / integers / floats / strings, in order.
    /// E.g., the value `true` is always interpreted as a boolean.
    ///
    /// Default: [`Double`](struct.IniStringQuote.html#associatedconstant.Double).
    pub(crate) string_quotes: IniStringQuote,
    /// Whether unquoted string values are supported.
    /// If `false`, an unquoted value must parse as a boolean / integer / float, or an error will be raised.
    /// If `string_quotes` is [`None`](struct.IniStringQuote.html#associatedconstant.None), this value is ignored.
    ///
    /// Default: `true`.
    pub(crate) unquoted_strings: bool,
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
    ///
    /// Default: `true`.
    pub(crate) escape: bool,
    /// Whether line ontinuation esacpe sequences (a backslash '\' followed by a newline '\n' / '\r')
    /// are supported in keys, section names and string values.
    /// If `escape` is `false`, this value is ignored.
    ///
    /// Default: `false`.
    pub(crate) line_continuation: bool,
    /// Duplicate section handling policy.
    ///
    /// Default: [`Merge`](enum.IniDuplicateSections.html#variant.Merge).
    pub(crate) duplicate_sections: IniDuplicateSections,
    /// Duplicate key handling policy.
    ///
    /// Default: [`Forbid`](enum.IniDuplicateKeys.html#variant.Forbid).
    pub(crate) duplicate_keys: IniDuplicateKeys,
    /// Whether arrays are supported.
    /// If `true`, values enclosed in brackets `'['` \ `']'` are parsed as
    /// comma (`','`) delimited arrays of booleans / integers / floats / strings.
    /// Types may not be mixed in the array, except integers / floats.
    ///
    /// Default: `false`.
    pub(crate) arrays: bool,
}

impl Default for IniOptions {
    fn default() -> Self {
        Self {
            comments: IniCommentDelimiter::Semicolon,
            inline_comments: false,
            key_value_separator: IniKeyValueSeparator::Equals,
            string_quotes: IniStringQuote::Double,
            unquoted_strings: true,
            escape: true,
            line_continuation: false,
            duplicate_sections: IniDuplicateSections::Merge,
            duplicate_keys: IniDuplicateKeys::Forbid,
            arrays: false,
        }
    }
}

/// Configuration options for serializing a config to an `.ini` string.
#[derive(Clone, Copy, Debug)]
pub struct ToIniStringOptions {
    /// See [`escape`](struct.IniParser.html#method.escape).
    ///
    /// Default: `true`.
    pub escape: bool,
    /// See [`arrays`](struct.IniParser.html#method.arrays).
    ///
    /// Default: `false`.
    pub arrays: bool,
}

impl Default for ToIniStringOptions {
    fn default() -> Self {
        Self {
            escape: true,
            arrays: false,
        }
    }
}
