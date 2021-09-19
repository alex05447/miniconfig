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
    ///     `'"'` (double quotes),
    ///     `'\''` (single quotes),
    ///     `'\0'` (null character),
    ///     `'\a'` (bell),
    ///     `'\b'` (backspace),
    ///     `'\t'` (horizontal tab),
    ///     `'\r'` (carriage return),
    ///     `'\n'` (new line / line feed),
    ///     `'\v'` (vertical tab),
    ///     `'\f'` (form feed),
    ///     `'\\'` (backslash / escape character),
    ///     `'\['` (`.ini` section/array open delimiter),
    ///     `'\]'` (`.ini` section/array close delimiter),
    ///     `'\;'` (`.ini` comment delimiter),
    ///     `'\#'` (optional `.ini` comment delimiter),
    ///     `'\='` (`.ini` key-value separator),
    ///     `'\:'` (optional `.ini` key-value separator),
    ///     `'\x??'` (where `?` are 2 hexadecimal digits) (ASCII escape sequence),
    ///     `'\u????'` (where `?` are 4 hexadecimal digits) (Unicode escape sequence).
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
    /// Maximum supported depth of nested sections.
    /// If `0`, sections are not supported at all.
    /// If `1`, one level of sections is supported; forward slashes (`'/'`) are treated as normal section name character.
    /// If `>1`, nested sections are supported; section names which contain forward slashes (`'/'`) are treated as paths.
    ///
    /// Default: `1`.
    pub(crate) nested_section_depth: u32,
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
            nested_section_depth: 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum StringQuote {
    Single,
    Double,
}

impl IniOptions {
    /// Is the character a supported comment delimiter?
    pub(super) fn is_comment_char(&self, val: char) -> bool {
        ((val == ';') && self.comments.contains(IniCommentDelimiter::Semicolon))
            || ((val == '#') && self.comments.contains(IniCommentDelimiter::NumberSign))
    }

    /// Are inline comments enabled and is the character a supported comment delimiter?
    pub(super) fn is_inline_comment_char(&self, val: char) -> bool {
        self.inline_comments && self.is_comment_char(val)
    }

    /// Is the character a supported key-value separator?
    pub(super) fn is_key_value_separator_char(&self, val: char) -> bool {
        ((val == '=')
            && self
                .key_value_separator
                .contains(IniKeyValueSeparator::Equals))
            || ((val == ':')
                && self
                    .key_value_separator
                    .contains(IniKeyValueSeparator::Colon))
    }

    /// Is the character a supported string quote?
    pub(super) fn is_string_quote_char(&self, val: char) -> Option<StringQuote> {
        if (val == '"') && self.string_quotes.contains(IniStringQuote::Double) {
            Some(StringQuote::Double)
        } else if (val == '\'') && self.string_quotes.contains(IniStringQuote::Single) {
            Some(StringQuote::Single)
        } else {
            None
        }
    }

    /// Is the character a supported string quote which matches `quote`?
    pub(super) fn is_matching_string_quote_char(&self, quote: StringQuote, other: char) -> bool {
        self.is_string_quote_char(other) == Some(quote)
    }

    /// Is the character a supported string quote which does not match `quote`?
    /// NOTE - only ever returns `true` if both single and double quotes are supported.
    pub(super) fn is_non_matching_string_quote_char(
        &self,
        quote: StringQuote,
        other: char,
    ) -> bool {
        if let Some(other) = self.is_string_quote_char(other) {
            other != quote
        } else {
            false
        }
    }

    /// Is the character a supported escape character?
    pub(super) fn is_escape_char(&self, val: char) -> bool {
        self.escape && (val == '\\')
    }

    /// Is the character a section start delimiter?
    pub(super) fn is_section_start(&self, val: char) -> bool {
        val == '['
    }

    /// Is the character a section end delimiter?
    pub(super) fn is_section_end(&self, val: char) -> bool {
        val == ']'
    }

    pub(super) fn nested_sections(&self) -> bool {
        self.nested_section_depth > 1
    }

    /// Is the character a nested section separator?
    pub(super) fn is_nested_section_separator(&self, val: char) -> bool {
        self.nested_sections() && val == '/'
    }

    /// Is the character an array start delimiter?
    pub(super) fn is_array_start(&self, val: char) -> bool {
        self.arrays && (val == '[')
    }

    /// Is the character an array end delimiter?
    pub(super) fn is_array_end(&self, val: char) -> bool {
        debug_assert!(self.arrays);
        val == ']'
    }

    /// Is the character an array value separator?
    pub(super) fn is_array_value_separator(&self, val: char) -> bool {
        val == ','
    }

    /// Is the character a recognized new line character?
    pub(super) fn is_new_line(&self, val: char) -> bool {
        matches!(val, '\n' | '\r')
    }

    /// Returns `true` if the `c` character is a valid key/value/section name character and does not have to be escaped.
    /// Otherwise, `c` must be escaped (preceded by a backslash) when used in keys/values/section names.
    pub(super) fn is_key_or_value_char(
        &self,
        c: char,
        in_section: bool,
        quote: Option<StringQuote>,
    ) -> bool {
        Self::is_key_or_value_char_impl(c, self.escape, self.nested_sections(), in_section, quote)
    }

    /// Returns `true` if the `c` character is a valid key/value/section name character and does not have to be escaped.
    /// Otherwise, `c` must be escaped (preceded by a backslash) when used in keys/values/section names.
    pub(super) fn is_key_or_value_char_impl(
        c: char,
        escape: bool,
        nested_sections: bool,
        in_section: bool,
        quote: Option<StringQuote>,
    ) -> bool {
        match c {
            // Escape char (backslash) must be escaped if escape sequences are supported.
            '\\' if escape => false,

            // Non-matching quotes don't need to be escaped in quoted strings.
            '"' => quote == Some(StringQuote::Single),
            '\'' => quote == Some(StringQuote::Double),

            // Space and special `.ini` characters in key/value/section strings
            // (except string quotes, handled above) don't need to be escaped in quoted strings.
            ' ' | '[' | ']' | '=' | ':' | ';' | '#' => quote.is_some(),

            // Nested section separators, if supported, must be escaped in unquoted section names.
            '/' if (nested_sections && in_section) => quote.is_some(),

            val => (val.is_alphanumeric() || val.is_ascii_punctuation()),
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
    /// See [`nested_section_depth`](struct.IniParser.html#method.nested_section_depth).
    ///
    /// Default: `1`.
    pub nested_section_depth: u32,
}

impl Default for ToIniStringOptions {
    fn default() -> Self {
        Self {
            escape: true,
            arrays: false,
            nested_section_depth: 1,
        }
    }
}

impl ToIniStringOptions {
    pub(crate) fn nested_sections(&self) -> bool {
        self.nested_section_depth > 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_key_or_value_char() {
        // Alphanumeric characters are always valid key/value chars.

        // Digits.
        for c in (b'0'..b'9').map(|c| char::from(c)) {
            assert!(IniOptions::is_key_or_value_char_impl(
                c, /* escape */ false, /* nested_sections */ false,
                /* in_section */ false, /* quote */ None
            ));
        }

        // ASCII chars.
        for c in (b'a'..b'z').map(|c| char::from(c)) {
            assert!(IniOptions::is_key_or_value_char_impl(
                c, /* escape */ false, /* nested_sections */ false,
                /* in_section */ false, /* quote */ None
            ));
        }

        for c in (b'A'..b'Z').map(|c| char::from(c)) {
            assert!(IniOptions::is_key_or_value_char_impl(
                c, /* escape */ false, /* nested_sections */ false,
                /* in_section */ false, /* quote */ None
            ));
        }

        // Other alphabetic chars.
        assert!(IniOptions::is_key_or_value_char_impl(
            'á', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));
        assert!(IniOptions::is_key_or_value_char_impl(
            '愛', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));

        // Double quotes are only valid when single-quoted.
        assert!(!IniOptions::is_key_or_value_char_impl(
            '"', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));
        assert!(IniOptions::is_key_or_value_char_impl(
            '"',
            /* escape */ false,
            /* nested_sections */ false,
            /* in_section */ false,
            /* quote */ Some(StringQuote::Single)
        ));

        // Single quotes are only valid when double-quoted.
        assert!(!IniOptions::is_key_or_value_char_impl(
            '\'', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));
        assert!(IniOptions::is_key_or_value_char_impl(
            '\'',
            /* escape */ false,
            /* nested_sections */ false,
            /* in_section */ false,
            /* quote */ Some(StringQuote::Double)
        ));

        // .ini special chars are only valid when quoted.
        let assert_ini_char = |c| {
            assert!(!IniOptions::is_key_or_value_char_impl(
                c, /* escape */ false, /* nested_sections */ false,
                /* in_section */ false, None
            ));
            assert!(IniOptions::is_key_or_value_char_impl(
                c,
                /* escape */ false,
                /* nested_sections */ false,
                /* in_section */ false,
                /* quote */ Some(StringQuote::Double)
            ));
            assert!(IniOptions::is_key_or_value_char_impl(
                c,
                /* escape */ false,
                /* nested_sections */ false,
                /* in_section */ false,
                /* quote */ Some(StringQuote::Single)
            ));
        };

        assert_ini_char(' ');
        assert_ini_char('[');
        assert_ini_char(']');
        assert_ini_char('=');
        assert_ini_char(':');
        assert_ini_char(';');
        assert_ini_char('#');

        // `/` is a valid key/value char when not using nested sections.
        assert!(IniOptions::is_key_or_value_char_impl(
            '/', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));

        // Valid outside of section names ...
        assert!(IniOptions::is_key_or_value_char_impl(
            '/', /* escape */ false, /* nested_sections */ true,
            /* in_section */ false, /* quote */ None
        ));

        // ... but invalid in unquoted section names ...
        assert!(!IniOptions::is_key_or_value_char_impl(
            '/', /* escape */ false, /* nested_sections */ true,
            /* in_section */ true, /* quote */ None
        ));

        // ... and valid in quoted section names.
        assert!(IniOptions::is_key_or_value_char_impl(
            '/',
            /* escape */ false,
            /* nested_sections */ true,
            /* in_section */ true,
            /* quote */ Some(StringQuote::Double)
        ));
        assert!(IniOptions::is_key_or_value_char_impl(
            '/',
            /* escape */ false,
            /* nested_sections */ true,
            /* in_section */ true,
            /* quote */ Some(StringQuote::Single)
        ));
    }
}
