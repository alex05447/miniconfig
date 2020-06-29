use {
    crate::{
        IniCommentDelimiter, IniDuplicateKeys, IniDuplicateSections, IniError, IniErrorKind,
        IniKeyValueSeparator, IniOptions, IniStringQuote, ValueType,
    },
    std::str::Chars,
};

/// `.ini` parser FSM states.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum IniParserState {
    /// Accept whitespace (including new lines),
    /// section start delimiters ('[') (-> BeforeSection),
    /// valid key chars (-> Key),
    /// escape sequences (if supported) (-> Key),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedKey).
    /// comment delimiters (';' / '#') (if supported) (-> SkipLine).
    StartLine,
    /// Accept whitespace (except new lines),
    /// valid key chars (-> Section),
    /// escape sequences (if supported) (-> Section),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedSection).
    BeforeSection,
    /// Accept valid key chars,
    /// escape sequences (if supported),
    /// section end delimiters (']') (-> SkipLineWhitespaceOrComments),
    /// whitespace (except new lines) (-> AfterSection).
    Section,
    /// Accept valid key chars,
    /// escape sequences (if supported),
    /// non-matching string quotes (if supported),
    /// spaces (' '),
    /// matching string quotes ('"' / '\'') (-> AfterQuotedSection).
    QuotedSection,
    /// Accept whitespace (except new lines),
    /// section end delimiters (']') (-> SkipLineWhitespaceOrComments).
    AfterSection,
    /// Accept new lines (-> StartLine),
    /// everything else.
    SkipLine,
    /// Accept new lines (-> StartLine),
    /// whitespace,
    /// comment start delimiters (';' / '#') (if supported) (-> SkipLine).
    SkipLineWhitespaceOrComments,
    /// Accept valid key chars,
    /// escape sequences (if supported),
    /// key-value separators ('=' / ':') (-> BeforeValue),
    /// whitespace (except new lines) (-> KeyValueSeparator).
    Key,
    /// Accept valid key chars,
    /// escape sequences (if supported),
    /// non-matching string quotes (if supported),
    /// spaces (' '),
    /// matching string quotes ('"' / '\'') (-> KeyValueSeparator).
    QuotedKey,
    /// Accept key-value separators ('=' / ':') (-> BeforeValue),
    /// whitespace (except new lines).
    KeyValueSeparator,
    /// Accept whitespace (except new lines (->StartLine)),
    /// inline comment delimiters (';' / '#') (if supported) (-> SkipLine),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedValue),
    /// escape sequences (if supported) (-> Value),
    /// array start delimiters (if supported) (-> BeforeArrayValue),
    /// valid value chars (-> Value).
    BeforeValue,
    /// Accept whitespace (-> SkipLineWhitespaceOrComments)
    /// (including new lines (-> StartLine)),
    /// inline comment delimiters (';' / '#') (if supported) (-> SkipLine),
    /// escape sequences (if supported),
    /// valid value chars.
    Value,
    /// Accept matching string quotes ('"' / '\'') (-> SkipLineWhitespaceOrComments),
    /// spaces (' '),
    /// non-matching string quotes (if supported),
    /// escape sequences (if supported),
    /// valid value chars.
    QuotedValue,
    /// Accept whitespace (except new lines),
    /// array end delimiters (-> SkipLineWhitespaceOrComments),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedArrayValue),
    /// escape sequences (if supported) (-> ArrayValue),
    /// valid value chars (-> ArrayValue).
    BeforeArrayValue,
    /// Accept whitespace (except new lines) (-> AfterArrayValue),
    /// array value separators (-> BeforeArrayValue),
    /// array end delimiters (-> SkipLineWhitespaceOrComments),
    /// escape sequences (if supported) (-> ArrayValue),
    /// valid value chars (-> ArrayValue).
    ArrayValue,
    /// Accept matching string quotes ('"' / '\'') (-> AfterArrayValue),
    /// spaces (' '),
    /// non-matching string quotes (if supported),
    /// escape sequences (if supported),
    /// valid value chars.
    QuotedArrayValue,
    /// Accept whitespace (except new lines),
    /// array value separators (-> BeforeArrayValue),
    /// array end delimiters (-> SkipLineWhitespaceOrComments).
    AfterArrayValue,
}

/// Parses the `.ini` config string, using the user-provided parsing options
/// and the [`config`](trait.IniConfig.html) object.
pub struct IniParser<'s> {
    /// Source string reader.
    reader: Chars<'s>,

    /// Current position in the source string.
    line: u32,
    column: u32,
    new_line: bool,

    /// Current parser FSM state.
    state: IniParserState,

    /// Parsing options as provided by the user.
    options: IniOptions,
}

impl<'s> IniParser<'s> {
    /// Creates a new [`parser`](struct.IniParser.html) from an `.ini` config `string`
    /// using default parsing options.
    pub fn new(string: &'s str) -> Self {
        Self {
            reader: string.chars(),
            state: IniParserState::StartLine,
            line: 1,
            column: 0,
            new_line: false,
            options: Default::default(),
        }
    }

    /// Sets the valid comment delimiter character(s).
    /// If [`None`](struct.IniCommentDelimiter.html#associatedconstant.None), comments are not supported.
    ///
    /// Default: [`Semicolon`](struct.IniCommentDelimiter.html#associatedconstant.Semicolon).
    pub fn comments(mut self, comments: IniCommentDelimiter) -> Self {
        self.options.comments = comments;
        self
    }

    /// Sets whether inline comments (i.e. those which don't begin at the start of the line) are supported.
    /// If [`comments`](#method.comments) is [`None`](struct.IniCommentDelimiter.html#associatedconstant.None), this value is ignored.
    ///
    /// Default: `false`.
    pub fn inline_comments(mut self, inline_comments: bool) -> Self {
        self.options.inline_comments = inline_comments;
        self
    }

    /// Sets the valid key-value separator character(s).
    /// If no flag is set, [`Equals`](struct.IniKeyValueSeparator.html#associatedconstant.Equals) is assumed.
    ///
    /// Default: [`Equals`](struct.IniKeyValueSeparator.html#associatedconstant.Equals).
    pub fn key_value_separator(mut self, key_value_separator: IniKeyValueSeparator) -> Self {
        self.options.key_value_separator = key_value_separator;
        self
    }

    /// Sets the valid string value quote character(s).
    /// If [`None`](struct.IniStringQuote.html#associatedconstant.None), quoted strings are not supported.
    /// In this case all values will be parsed as booleans / integers / floats / strings, in order.
    /// E.g., the value `true` is always interpreted as a boolean.
    ///
    /// Default: [`Double`](struct.IniStringQuote.html#associatedconstant.Double).
    pub fn string_quotes(mut self, string_quotes: IniStringQuote) -> Self {
        self.options.string_quotes = string_quotes;
        self
    }

    /// Sets whether unquoted string values are supported.
    /// If `false`, an unquoted value must parse as a boolean / integer / float, or an error will be raised.
    /// If [`string_quotes`](#method.string_quotes) is [`None`](struct.IniStringQuote.html#associatedconstant.None), this value is ignored.
    ///
    /// Default: `true`.
    pub fn unquoted_strings(mut self, unquoted_strings: bool) -> Self {
        self.options.unquoted_strings = unquoted_strings;
        self
    }

    /// Sets whether escape sequences (a character sequence following a backslash ('\'))
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
    pub fn escape(mut self, escape: bool) -> Self {
        self.options.escape = escape;
        self
    }

    /// Sets whether line ontinuation esacpe sequences (a backslash '\' followed by a newline '\n' / '\r')
    /// are supported in keys, section names and string values.
    /// If [`escape`](#method.escape) is `false`, this value is ignored.
    ///
    /// Default: `false`.
    pub fn line_continuation(mut self, line_continuation: bool) -> Self {
        self.options.line_continuation = line_continuation;
        self
    }

    /// Sets the duplicate section handling policy.
    ///
    /// Default: [`Merge`](enum.IniDuplicateSections.html#variant.Merge).
    pub fn duplicate_sections(mut self, duplicate_sections: IniDuplicateSections) -> Self {
        self.options.duplicate_sections = duplicate_sections;
        self
    }

    /// Sets the duplicate key handling policy.
    ///
    /// Default: [`Forbid`](enum.IniDuplicateKeys.html#variant.Forbid).
    pub fn duplicate_keys(mut self, duplicate_keys: IniDuplicateKeys) -> Self {
        self.options.duplicate_keys = duplicate_keys;
        self
    }

    /// Sets whether arrays are supported.
    /// If `true`, values enclosed in brackets `'['` \ `']'` are parsed as
    /// comma (`','`) delimited arrays of booleans / integers / floats / strings.
    /// Types may not be mixed in the array, except integers / floats.
    ///
    /// Default: `false`.
    pub fn arrays(mut self, arrays: bool) -> Self {
        self.options.arrays = arrays;
        self
    }

    /// Consumes the parser and tries to parse the `.ini` config string, filling the passed `config`.
    pub fn parse<C: IniConfig>(mut self, config: &mut C) -> Result<(), IniError> {
        use IniErrorKind::*;

        self.validate_options();

        // Scratch buffer for sections / keys / values.
        let mut buffer = String::new();

        // Current section, if any.
        let mut section = String::new();

        // Current key, if any.
        let mut key = String::new();

        // Whether the key is unique in its table (root or section).
        let mut is_key_unique = true;

        // Current opening string quote, if any.
        let mut quote: Option<char> = None;

        // Whether we need to skip all key/value pairs in the current section
        // (i.e., when we encountered a duplicate section instance and we use the `First` duplicate section policy).
        let mut skip_section = false;

        // Whether we need to skip the current value
        // (i.e., when we encountered a duplicate key and we use the `First` duplicate key policy).
        let mut skip_value = false;

        // Scratch buffer for unicode escape sequences, if supported.
        let mut unicode_buffer = if self.options.escape {
            String::with_capacity(4)
        } else {
            String::new()
        };

        let mut array: Option<Vec<IniValue<String>>> = None;

        // Read the chars until EOF, process according to current state.
        while let Some(c) = self.next() {
            match self.state {
                IniParserState::StartLine => {
                    is_key_unique = true;
                    skip_value = false;

                    // Skip whitespace at the start of the line (including new lines).
                    if c.is_whitespace() {

                        // Section start delimiter - parse the section name.
                    } else if c == '[' {
                        skip_section = false;

                        self.state = IniParserState::BeforeSection;

                    // Line comment (if supported) - skip the rest of the line.
                    } else if self.is_comment_char(c) {
                        self.state = IniParserState::SkipLine;

                    // String quote - parse the key in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(c) {
                        debug_assert!(quote.is_none());
                        quote.replace(c);

                        self.state = IniParserState::QuotedKey;

                    // Escaped char (if supported) - parse the escape sequence as the key.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - start parsing the key.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                debug_assert!(buffer.is_empty());
                                buffer.push(c);

                                self.state = IniParserState::Key;
                            }
                            // Line continuation - error.
                            ParseEscapeSequenceResult::LineContinuation => {
                                return Err(self.error(UnexpectedNewLineInKey));
                            }
                        }

                    // Valid key start - parse the key.
                    } else if Self::is_key_or_value_char(c, self.options.escape, quote) {
                        debug_assert!(buffer.is_empty());
                        buffer.push(c);

                        self.state = IniParserState::Key;

                    // Key-value separator - it's an empty key.
                    } else if self.is_key_value_separator_char(c) {
                        return Err(self.error_offset(EmptyKey));

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterAtLineStart(c)));
                    }
                }
                IniParserState::BeforeSection => {
                    debug_assert!(buffer.is_empty());

                    // Skip whitespace.
                    if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(UnexpectedNewLineInSectionName));
                        }

                    // String quote - parse the section name in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(c) {
                        debug_assert!(quote.is_none());
                        quote.replace(c);

                        self.state = IniParserState::QuotedSection;

                    // Escaped char (if supported) - start parsing the section name.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the section name.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);

                                self.state = IniParserState::Section;
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid section name char (same rules as key chars) - start parsing the section name.
                    } else if Self::is_key_or_value_char(c, self.options.escape, quote) {
                        buffer.push(c);

                        self.state = IniParserState::Section;

                    // Section end delimiter - empty section names not allowed.
                    } else if c == ']' {
                        return Err(self.error(EmptySectionName));

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName(c)));
                    }
                }
                IniParserState::Section => {
                    debug_assert!(!buffer.is_empty());

                    // New line before the section delimiter - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInSectionName));

                    // Escaped char (if supported) - keep parsing the section name.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the section name.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid section name char (same rules as key chars) - keep parsing the section name.
                    } else if Self::is_key_or_value_char(c, self.options.escape, quote) {
                        buffer.push(c);

                    // Section end delimiter - finish the section name, skip the rest of the line.
                    } else if c == ']' {
                        // Empty section names not allowed.
                        if buffer.is_empty() {
                            return Err(self.error(EmptySectionName));
                        }

                        std::mem::swap(&mut section, &mut buffer);
                        buffer.clear();

                        // Try to add the section to the config.
                        skip_section = self.add_section(config, &section)?;

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Whitespace after section name (new lines handle above) - skip it, finish the section name, parse the section end delimiter.
                    // NOTE - section name is not empty if we got here.
                    } else if c.is_whitespace() {
                        std::mem::swap(&mut section, &mut buffer);
                        buffer.clear();

                        // Try to add the section to the config.
                        skip_section = self.add_section(config, &section)?;

                        self.state = IniParserState::AfterSection;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName(c)));
                    }
                }
                IniParserState::QuotedSection => {
                    let cur_quote = quote.unwrap();

                    // New line before the closing quotes - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInSectionName));

                    // Closing quotes - finish the quoted section, keep parsing until the section delimiter.
                    } else if c == cur_quote {
                        // Empty section names not allowed.
                        if buffer.is_empty() {
                            return Err(self.error(EmptySectionName));
                        }

                        quote.take();

                        std::mem::swap(&mut section, &mut buffer);
                        buffer.clear();

                        // Try to add the section to the config.
                        skip_section = self.add_section(config, &section)?;

                        self.state = IniParserState::AfterSection;

                    // Escaped char (if supported) - keep parsing the section name.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the section name.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Non-matching quotes - keep parsing the value.
                    } else if self.is_non_matching_string_quote_char(cur_quote, c) {
                        buffer.push(c);

                    // Space or valid value char - keep parsing the value.
                    } else if c == ' ' || Self::is_key_or_value_char(c, self.options.escape, quote)
                    {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName(c)));
                    }
                }
                IniParserState::AfterSection => {
                    debug_assert!(buffer.is_empty());
                    debug_assert!(!section.is_empty());

                    // Skip whitespace.
                    if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(UnexpectedNewLineInSectionName));
                        }

                    // Section end delimiter - skip the rest of the line.
                    } else if c == ']' {
                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterAfterSectionName(c)));
                    }
                }
                IniParserState::SkipLine => {
                    debug_assert!(buffer.is_empty());

                    // If it's a new line, start parsing the next line.
                    // Skip everything else.
                    if self.is_new_line(c) {
                        self.state = IniParserState::StartLine;
                    }
                }
                IniParserState::SkipLineWhitespaceOrComments => {
                    debug_assert!(buffer.is_empty());

                    // If it's a new line, start parsing the next line.
                    if self.is_new_line(c) {
                        self.state = IniParserState::StartLine;

                    // Skip other whitespace.
                    } else if c.is_whitespace() {
                        // continue

                        // Inline comment (if supported) - skip the rest of the line.
                    } else if self.is_inline_comment_char(c) {
                        self.state = IniParserState::SkipLine;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterAtLineEnd(c)));
                    }
                }
                IniParserState::Key => {
                    // We have at least one key character already parsed.
                    debug_assert!(!buffer.is_empty());
                    debug_assert!(key.is_empty());

                    // Key-value separator - finish the key, parse the value.
                    if self.is_key_value_separator_char(c) {
                        std::mem::swap(&mut key, &mut buffer);

                        self.check_is_key_duplicate(
                            config,
                            &section,
                            &key,
                            skip_section,
                            &mut skip_value,
                            &mut is_key_unique,
                        )?;

                        self.state = IniParserState::BeforeValue;

                    // Whitespace between the key and the separator - skip it, finish the key, parse the separator.
                    } else if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(UnexpectedNewLineInKey));
                        }

                        std::mem::swap(&mut key, &mut buffer);

                        self.check_is_key_duplicate(
                            config,
                            &section,
                            &key,
                            skip_section,
                            &mut skip_value,
                            &mut is_key_unique,
                        )?;

                        self.state = IniParserState::KeyValueSeparator;

                    // Escaped char (if supported) - keep parsing the key.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the key.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid key char - keep parsing the key.
                    } else if Self::is_key_or_value_char(c, self.options.escape, quote) {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInKey(c)));
                    }
                }
                IniParserState::QuotedKey => {
                    debug_assert!(key.is_empty());

                    let cur_quote = quote.unwrap();

                    // New line before the closing quotes - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInKey));

                    // Closing quotes - finish the key, parse the separator.
                    } else if c == cur_quote {
                        // Empty keys are not allowed.
                        if buffer.is_empty() {
                            return Err(self.error(EmptyKey));
                        }

                        quote.take();

                        std::mem::swap(&mut key, &mut buffer);

                        self.check_is_key_duplicate(
                            config,
                            &section,
                            &key,
                            skip_section,
                            &mut skip_value,
                            &mut is_key_unique,
                        )?;

                        self.state = IniParserState::KeyValueSeparator;

                    // Escaped char (if supported) - keep parsing the key.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the key.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Non-matching quotes - keep parsing the key.
                    } else if self.is_non_matching_string_quote_char(cur_quote, c) {
                        buffer.push(c);

                    // Space or valid key char - keep parsing the key.
                    } else if c == ' ' || Self::is_key_or_value_char(c, self.options.escape, quote)
                    {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInKey(c)));
                    }
                }
                IniParserState::KeyValueSeparator => {
                    debug_assert!(buffer.is_empty());

                    // Key-value separator - parse the value (key already finished).
                    if self.is_key_value_separator_char(c) {
                        self.state = IniParserState::BeforeValue;

                    // Skip the whitespace between the key and the separator.
                    } else if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(UnexpectedNewLineInKey));
                        }

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidKeyValueSeparator(c)));
                    }
                }
                IniParserState::BeforeValue => {
                    debug_assert!(buffer.is_empty());
                    debug_assert!(!key.is_empty());
                    debug_assert!(array.is_none());

                    // Skip the whitespace before the value.
                    if c.is_whitespace() {
                        // Unless it's a new line - the value is empty.
                        if self.is_new_line(c) {
                            self.add_value_to_config(
                                config,
                                &section,
                                &key,
                                "",
                                false,
                                skip_section | skip_value,
                                is_key_unique,
                            )?;
                            key.clear();

                            self.state = IniParserState::StartLine;
                        }

                    // Inline comment (if supported) - the value is empty, skip the rest of the line.
                    } else if self.is_inline_comment_char(c) {
                        self.add_value_to_config(
                            config,
                            &section,
                            &key,
                            "",
                            false,
                            skip_section | skip_value,
                            is_key_unique,
                        )?;
                        key.clear();

                        self.state = IniParserState::SkipLine;

                    // String quote - parse the string value in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(c) {
                        debug_assert!(quote.is_none());
                        quote.replace(c);

                        self.state = IniParserState::QuotedValue;

                    // Escaped char (if supported) - parse the escape sequence, start parsing the value.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - start parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);

                                self.state = IniParserState::Value;
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Array start delimiter (if supported) - start parsing the array.
                    } else if self.is_array_start(c) {
                        array.replace(Vec::new());
                        self.state = IniParserState::BeforeArrayValue;

                    // Valid value char - start parsing the unquoted value.
                    } else if Self::is_key_or_value_char(c, self.options.escape, quote) {
                        buffer.push(c);

                        self.state = IniParserState::Value;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::Value => {
                    debug_assert!(!buffer.is_empty());
                    debug_assert!(!key.is_empty());

                    // Whitespace - finish the value.
                    if c.is_whitespace() {
                        self.add_value_to_config(
                            config,
                            &section,
                            &key,
                            &buffer,
                            false,
                            skip_section | skip_value,
                            is_key_unique,
                        )?;
                        buffer.clear();
                        key.clear();

                        // New line - start a new line.
                        if self.is_new_line(c) {
                            self.state = IniParserState::StartLine;

                        // Not a new line - skip the rest of the line.
                        } else {
                            self.state = IniParserState::SkipLineWhitespaceOrComments;
                        }

                    // Inline comment (if supported) - finish the value, skip the rest of the line.
                    } else if self.is_inline_comment_char(c) {
                        self.add_value_to_config(
                            config,
                            &section,
                            &key,
                            &buffer,
                            false,
                            skip_section | skip_value,
                            is_key_unique,
                        )?;
                        buffer.clear();
                        key.clear();

                        self.state = IniParserState::SkipLine;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid value char - keep parsing the value.
                    } else if Self::is_key_or_value_char(c, self.options.escape, quote) {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::QuotedValue => {
                    debug_assert!(!key.is_empty());

                    let cur_quote = quote.unwrap();

                    // New line before the closing quotes - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInQuotedValue));

                    // Closing quotes - finish the value (may be empty), skip the rest of the line.
                    } else if c == cur_quote {
                        self.add_value_to_config(
                            config,
                            &section,
                            &key,
                            &buffer,
                            true,
                            skip_section | skip_value,
                            is_key_unique,
                        )?;
                        buffer.clear();
                        key.clear();

                        quote.take();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Non-matching quotes - keep parsing the value.
                    } else if self.is_non_matching_string_quote_char(cur_quote, c) {
                        buffer.push(c);

                    // Space or valid value char - keep parsing the value.
                    } else if c == ' ' || Self::is_key_or_value_char(c, self.options.escape, quote)
                    {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::BeforeArrayValue => {
                    debug_assert!(buffer.is_empty());
                    debug_assert!(!key.is_empty());

                    // Skip the whitespace before the array value.
                    if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(UnexpectedNewLineInArray));
                        }

                    // Array end delimiter - finish the array, skip the rest of the line.
                    } else if self.is_array_end(c) {
                        Self::add_array_to_config(
                            config,
                            &section,
                            &key,
                            array.take().unwrap(),
                            skip_section | skip_value,
                            is_key_unique,
                        );
                        key.clear();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // String quote - parse the string array value in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(c) {
                        let array = array.as_ref().unwrap();

                        // Make sure array is empty or contains strings (is not mixed).
                        if !array.is_empty() {
                            if !unsafe { array.get_unchecked(0) }
                                .get_type()
                                .is_compatible(ValueType::String)
                            {
                                return Err(self.error(IniErrorKind::MixedArray));
                            }
                        }

                        debug_assert!(quote.is_none());
                        quote.replace(c);

                        self.state = IniParserState::QuotedArrayValue;

                    // Escaped char (if supported) - parse the escape sequence, start parsing the array value.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);

                                self.state = IniParserState::ArrayValue;
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid value char - start parsing the unquoted array value.
                    } else if Self::is_key_or_value_char(c, self.options.escape, quote) {
                        buffer.push(c);

                        self.state = IniParserState::ArrayValue;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInArray(c)));
                    }
                }
                IniParserState::ArrayValue => {
                    debug_assert!(!buffer.is_empty());
                    debug_assert!(!key.is_empty());

                    let array_mut = array.as_mut().unwrap();

                    // Whitespace - finish the current array value,
                    // parse the array value separator / array end delimiter.
                    if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(IniErrorKind::UnexpectedNewLineInArray));
                        }

                        self.add_value_to_array(
                            array_mut,
                            &buffer,
                            false,
                            skip_value | skip_section,
                        )?;
                        buffer.clear();

                        self.state = IniParserState::AfterArrayValue;

                    // Array value separator - finish the current array value,
                    // parse the next array value / array end delimiter.
                    } else if self.is_array_value_separator(c) {
                        self.add_value_to_array(
                            array_mut,
                            &buffer,
                            false,
                            skip_value | skip_section,
                        )?;
                        buffer.clear();

                        self.state = IniParserState::BeforeArrayValue;

                    // Array end delimiter - add the value to the array, finish the array, skip the rest of the line.
                    } else if self.is_array_end(c) {
                        self.add_value_to_array(
                            array_mut,
                            &buffer,
                            false,
                            skip_value | skip_section,
                        )?;
                        buffer.clear();

                        Self::add_array_to_config(
                            config,
                            &section,
                            &key,
                            array.take().unwrap(),
                            skip_section | skip_value,
                            is_key_unique,
                        );
                        key.clear();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the array value.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid value char - keep parsing the array value.
                    } else if Self::is_key_or_value_char(c, self.options.escape, quote) {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::QuotedArrayValue => {
                    debug_assert!(!key.is_empty());

                    let cur_quote = quote.unwrap();
                    let array_mut = array.as_mut().unwrap();

                    // New line before the closing quotes - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInQuotedValue));

                    // Closing quotes - finish the array value (may be empty),
                    // parse the array value separator / array end delimiter.
                    } else if c == cur_quote {
                        self.add_value_to_array(
                            array_mut,
                            &buffer,
                            true,
                            skip_value | skip_section,
                        )?;
                        buffer.clear();

                        quote.take();

                        self.state = IniParserState::AfterArrayValue;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the array value.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Non-matching quotes - keep parsing the array value.
                    } else if self.is_non_matching_string_quote_char(cur_quote, c) {
                        buffer.push(c);

                    // Space or valid value char - keep parsing the array value.
                    } else if c == ' ' || Self::is_key_or_value_char(c, self.options.escape, quote)
                    {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::AfterArrayValue => {
                    debug_assert!(!key.is_empty());

                    // Skip whitespace.
                    if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(UnexpectedNewLineInArray));
                        }

                    // Array value separator - parse the next array value / array end delimiter.
                    } else if self.is_array_value_separator(c) {
                        self.state = IniParserState::BeforeArrayValue;

                    // Array end delimiter - finish the array, skip the rest of the line.
                    } else if self.is_array_end(c) {
                        Self::add_array_to_config(
                            config,
                            &section,
                            &key,
                            array.take().unwrap(),
                            skip_section | skip_value,
                            is_key_unique,
                        );
                        key.clear();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInArray(c)));
                    }
                }
            }
        }

        match self.state {
            IniParserState::BeforeSection
            | IniParserState::Section
            | IniParserState::QuotedSection
            | IniParserState::AfterSection => {
                return Err(self.error(UnexpectedEndOfFileInSectionName))
            }
            IniParserState::Key | IniParserState::QuotedKey | IniParserState::KeyValueSeparator => {
                return Err(self.error(UnexpectedEndOfFileBeforeKeyValueSeparator))
            }
            IniParserState::QuotedValue => {
                return Err(self.error(UnexpectedEndOfFileInQuotedString))
            }
            // Add the last value if we were parsing it right before EOF.
            IniParserState::Value | IniParserState::BeforeValue => {
                self.add_value_to_config(
                    config,
                    &section,
                    &key,
                    &buffer,
                    quote.is_some(),
                    skip_section | skip_value,
                    is_key_unique,
                )?;
            }
            IniParserState::BeforeArrayValue
            | IniParserState::ArrayValue
            | IniParserState::AfterArrayValue => return Err(self.error(UnexpectedEndOfFileInArray)),
            IniParserState::QuotedArrayValue => {
                return Err(self.error(IniErrorKind::UnexpectedEndOfFileInQuotedArrayValue))
            }
            IniParserState::StartLine
            | IniParserState::SkipLine
            | IniParserState::SkipLineWhitespaceOrComments => {}
        }

        Ok(())
    }

    fn validate_options(&mut self) {
        // Must have some key-value separator if none provided by the user - use `Equals`.
        if self.options.key_value_separator.is_empty() {
            self.options.key_value_separator = IniKeyValueSeparator::Equals;
        }

        // If not using quoted strings, unquoted strings must be supported.
        if self.options.string_quotes.is_empty() {
            self.options.unquoted_strings = true;
        }
    }

    /// Reads the next character from the source string reader.
    /// Increments the line/column counters.
    fn next(&mut self) -> Option<char> {
        let next = self.reader.next();

        if self.new_line {
            self.line += 1;
            self.column = 0;

            self.new_line = false;
        }

        match next {
            Some('\n') | Some('\r') => {
                self.column += 1;
                self.new_line = true;
            }
            Some(_) => {
                self.column += 1;
            }
            None => {}
        }

        next
    }

    /// Error helper method.
    fn error(&self, error: IniErrorKind) -> IniError {
        IniError {
            line: self.line,
            column: self.column,
            error,
        }
    }

    /// Error helper method.
    fn error_offset(&self, error: IniErrorKind) -> IniError {
        debug_assert!(self.column > 0);

        IniError {
            line: self.line,
            column: self.column - 1,
            error,
        }
    }

    /// Is the character a supported comment delimiter?
    fn is_comment_char(&self, val: char) -> bool {
        ((val == ';')
            && self
                .options
                .comments
                .contains(IniCommentDelimiter::Semicolon))
            || ((val == '#')
                && self
                    .options
                    .comments
                    .contains(IniCommentDelimiter::NumberSign))
    }

    /// Are inline comments enabled and is the character a supported comment delimiter?
    fn is_inline_comment_char(&self, val: char) -> bool {
        self.options.inline_comments && self.is_comment_char(val)
    }

    /// Is the character a supported key-value separator?
    fn is_key_value_separator_char(&self, val: char) -> bool {
        ((val == '=')
            && self
                .options
                .key_value_separator
                .contains(IniKeyValueSeparator::Equals))
            || ((val == ':')
                && self
                    .options
                    .key_value_separator
                    .contains(IniKeyValueSeparator::Colon))
    }

    /// Is the character a supported string quote?
    fn is_string_quote_char(&self, val: char) -> bool {
        ((val == '"') && self.options.string_quotes.contains(IniStringQuote::Double))
            || ((val == '\'') && self.options.string_quotes.contains(IniStringQuote::Single))
    }

    /// Is the character a supported string quote which does not match `quote`?
    /// NOTE - only ever returns `true` if both single and double quotes are supported.
    fn is_non_matching_string_quote_char(&self, quote: char, other: char) -> bool {
        self.is_string_quote_char(other) && (other != quote)
    }

    /// Is the character a supported escape character?
    fn is_escape_char(&self, val: char) -> bool {
        self.options.escape && (val == '\\')
    }

    /// Is the character an array start delimiter?
    fn is_array_start(&self, val: char) -> bool {
        self.options.arrays && (val == '[')
    }

    /// Is the character an array end delimiter?
    fn is_array_end(&self, val: char) -> bool {
        debug_assert!(self.options.arrays);
        val == ']'
    }

    /// Is the character an array value separator?
    fn is_array_value_separator(&self, val: char) -> bool {
        val == ','
    }

    /// Is the character a recognized new line character?
    fn is_new_line(&self, val: char) -> bool {
        matches!(val, '\n' | '\r')
    }

    /// Reads up to 4 following characters and tries to parses them as an escape sequence.
    fn parse_escape_sequence(
        &mut self,
        unicode_buffer: &mut String,
    ) -> Result<ParseEscapeSequenceResult, IniError> {
        use IniErrorKind::*;
        use ParseEscapeSequenceResult::*;

        debug_assert!(self.options.escape);

        match self.next() {
            None => Err(self.error(UnexpectedEndOfFileInEscapeSequence)),

            // Backslash followed by a new line is a line continuation, if supported.
            Some('\n') | Some('\r') => {
                if self.options.line_continuation {
                    Ok(LineContinuation)
                } else {
                    Err(self.error_offset(UnexpectedNewLineInEscapeSequence))
                }
            }

            // Standard escaped characters.
            Some('\\') => Ok(EscapedChar('\\')),
            Some('\'') => Ok(EscapedChar('\'')),
            Some('"') => Ok(EscapedChar('"')),
            Some('0') => Ok(EscapedChar('\0')),
            Some('a') => Ok(EscapedChar('\x07')),
            Some('b') => Ok(EscapedChar('\x08')),
            Some('t') => Ok(EscapedChar('\t')),
            Some('r') => Ok(EscapedChar('\r')),
            Some('n') => Ok(EscapedChar('\n')),
            Some('v') => Ok(EscapedChar('\x0b')),
            Some('f') => Ok(EscapedChar('\x0c')),

            // Escaped space.
            Some(' ') => Ok(EscapedChar(' ')),

            // Escaped `.ini` special characters, disallowed otherwise.
            Some('[') => Ok(EscapedChar('[')),
            Some(']') => Ok(EscapedChar(']')),
            Some(';') => Ok(EscapedChar(';')),
            Some('#') => Ok(EscapedChar('#')),
            Some('=') => Ok(EscapedChar('=')),
            Some(':') => Ok(EscapedChar(':')),

            // 4 hexadecimal Unicode values.
            Some('x') => {
                unicode_buffer.clear();

                for _ in 0..4 {
                    match self.next() {
                        None => return Err(self.error(UnexpectedEndOfFileInUnicodeEscapeSequence)),
                        Some('\n') | Some('\r') => {
                            return Err(self.error_offset(UnexpectedNewLineInUnicodeEscapeSequence))
                        }
                        Some(c) => unicode_buffer.push(c),
                    }
                }

                match u32::from_str_radix(&unicode_buffer, 16) {
                    Ok(val) => match std::char::from_u32(val) {
                        Some(val) => Ok(EscapedChar(val)),
                        None => Err(self.error(InvalidUnicodeEscapeSequence)),
                    },
                    Err(_) => Err(self.error(InvalidUnicodeEscapeSequence)),
                }
            }

            Some(c) => Err(self.error(InvalidEscapeCharacter(c))),
        }
    }

    /// Returns `Ok(true)` if we need to skip the current section;
    /// else returns `Ok(false)`.
    fn add_section<C: IniConfig>(&self, config: &mut C, section: &str) -> Result<bool, IniError> {
        // Section does not exist in the config - add it.
        if !config.contains_section(section) {
            config.add_section(section, false);
            Ok(false)

        // Section already exists.
        } else {
            match self.options.duplicate_sections {
                // We don't support duplicate sections - error.
                IniDuplicateSections::Forbid => {
                    return Err(self.error(IniErrorKind::DuplicateSection(section.into())))
                }
                // Skip this section.
                IniDuplicateSections::First => Ok(true),
                // Overwrite the previous instance section with the new one.
                IniDuplicateSections::Last => {
                    config.add_section(section, true);
                    Ok(false)
                }
                // Just add the new key/value pairs to the existing section.
                IniDuplicateSections::Merge => Ok(false),
            }
        }
    }

    /// Parses a string `value` and adds it to the config `section` at `key`.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn add_value_to_config<C: IniConfig>(
        &self,
        config: &mut C,
        section: &str,
        key: &str,
        value: &str,
        quoted: bool,
        skip: bool,
        is_key_unique: bool,
    ) -> Result<(), IniError> {
        debug_assert!(!key.is_empty());

        if skip {
            return Ok(());
        }

        let value = self.parse_value_string(value, quoted)?;

        config.add_value(
            if section.is_empty() {
                None
            } else {
                Some(section)
            },
            key,
            value,
            !is_key_unique,
        );

        Ok(())
    }

    /// Parses a string `value` and adds it to the `array`.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn add_value_to_array(
        &self,
        array: &mut Vec<IniValue<String>>,
        value: &'s str,
        quoted: bool,
        skip: bool,
    ) -> Result<(), IniError> {
        if skip {
            return Ok(());
        }

        let value = self.parse_value_string(value, quoted)?;

        // Make sure the array is not mixed.
        if !array.is_empty() {
            if !array
                .get(0)
                .unwrap()
                .get_type()
                .is_compatible(value.get_type())
            {
                return Err(self.error_offset(IniErrorKind::MixedArray));
            }
        }

        array.push(match value {
            IniValue::Bool(value) => IniValue::Bool(value),
            IniValue::I64(value) => IniValue::I64(value),
            IniValue::F64(value) => IniValue::F64(value),
            IniValue::String(value) => IniValue::String(value.into()),
        });

        Ok(())
    }

    /// Adds the `array` to the config `section` at `key`.
    fn add_array_to_config<C: IniConfig>(
        config: &mut C,
        section: &str,
        key: &str,
        array: Vec<IniValue<String>>,
        skip: bool,
        is_key_unique: bool,
    ) {
        debug_assert!(!key.is_empty());

        if skip {
            return;
        }

        config.add_array(
            if section.is_empty() {
                None
            } else {
                Some(section)
            },
            key,
            array,
            is_key_unique,
        );
    }

    /// Parses a string `value`.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn parse_value_string<'v>(
        &self,
        value: &'v str,
        quoted: bool,
    ) -> Result<IniValue<&'v str>, IniError> {
        use IniErrorKind::*;
        use IniValue::*;

        // Empty and quoted values are treated as strings.
        let value = if value.is_empty() || quoted {
            String(value)

        // Check if it's a bool.
        } else if value == "true" {
            Bool(true)
        } else if value == "false" {
            Bool(false)

        // Check if it's an integer.
        } else if let Ok(value) = value.parse::<i64>() {
            I64(value)

        // Else check if it's a float.
        } else if let Ok(value) = value.parse::<f64>() {
            F64(value)

        // Else it's a string.
        } else {
            // Unless we don't allow unquoted strings.
            if !self.options.unquoted_strings {
                return Err(self.error(UnquotedString));
            }

            String(value)
        };

        Ok(value)
    }

    /// Sets `skip_value` to `true` if we need to skip the current value;
    /// sets `is_key_unique` to `true` if the key is not contained in the root / section of the config.
    fn check_is_key_duplicate<C: IniConfig>(
        &self,
        config: &C,
        section: &str,
        key: &str,
        skip_section: bool,
        skip_value: &mut bool,
        is_key_unique: &mut bool,
    ) -> Result<(), IniError> {
        use IniErrorKind::*;

        debug_assert!(!key.is_empty());

        if skip_section {
            *skip_value = true;
            *is_key_unique = false;

            return Ok(());
        }

        let is_unique = !config.contains_key(
            if section.is_empty() {
                None
            } else {
                Some(section)
            },
            key,
        );

        match self.options.duplicate_keys {
            IniDuplicateKeys::Forbid => {
                if is_unique {
                    *skip_value = false;
                    *is_key_unique = true;

                    Ok(())
                } else {
                    Err(self.error_offset(DuplicateKey(key.into())))
                }
            }
            // If `is_unique == true`, it's the first key and we must process it -> return `false` (don't skip).
            IniDuplicateKeys::First => {
                *skip_value = !is_unique;
                *is_key_unique = is_unique;

                Ok(())
            }
            // Never skip keys when we're interested in the last one.
            IniDuplicateKeys::Last => {
                *skip_value = false;
                *is_key_unique = is_unique;

                Ok(())
            }
        }
    }

    fn is_key_or_value_char(val: char, escape: bool, quote: Option<char>) -> bool {
        if let Some(quote) = quote {
            debug_assert!(quote == '"' || quote == '\'');
        }

        match val {
            // Escape char must be escaped if escape sequences are supported.
            '\\' if escape => false,

            // Non-matching quotes don't need to be escaped in quoted strings.
            '"' => quote == Some('\''),
            '\'' => quote == Some('"'),

            // Space and special `.ini` characters in key/value/section strings
            // (except string quotes, handled above) don't need to be escaped in quoted strings.
            ' ' | '[' | ']' | '=' | ':' | ';' | '#' => quote.is_some(),

            val => (val.is_alphanumeric() || val.is_ascii_punctuation()),
        }
    }
}

enum ParseEscapeSequenceResult {
    /// Parsed an escape sequence as a valid char.
    EscapedChar(char),
    /// Parsed an escape sequence as a line continuation.
    LineContinuation,
}

/// Represents an individual leaf-level `.ini` config value,
/// contained in the root of the config, config section or an array.
pub enum IniValue<S> {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(S),
}

impl<S> IniValue<S> {
    fn get_type(&self) -> ValueType {
        match self {
            IniValue::Bool(_) => ValueType::Bool,
            IniValue::I64(_) => ValueType::I64,
            IniValue::F64(_) => ValueType::F64,
            IniValue::String(_) => ValueType::String,
        }
    }
}

/// A trait which represents the config being filled by the [`.ini parser`](struct.IniParser.html)
/// during the call to [`parse`](struct.IniParser.html#method.parse).
pub trait IniConfig {
    /// Returns `true` if the config already contains the `section`
    /// (i.e. [`add_section`](#method.add_section) was called with this `section` name,
    /// regardless of `overwrite` value).
    /// Else returns `false`.
    ///
    /// NOTE - `section` name is not empty.
    ///
    /// NOTE - this is necessary because the `.ini` parser does not keep track internally of all previously parsed sections.
    fn contains_section(&self, section: &str) -> bool;

    /// Adds the `section` to the config.
    ///
    /// If `overwrite` is `true`, the section is duplicate (i.e. [`contains_section`](#method.contains_section) previously returned `true`)
    /// and the parser is [`configured`](enum.IniDuplicateSections.html) to [`overwrite`](enum.IniDuplicateSections.html#variant.Last)
    /// the `section`.
    /// If `overwrite` is `false`, the section is added for the first time.
    ///
    /// NOTE - `section` name is not empty.
    fn add_section(&mut self, section: &str, overwrite: bool);

    /// Returns `true` if the `section` (if `Some`) or the config root (if `section` is `None`) already contains the `key`
    /// (i.e. [`add_value`](#method.add_value) was called with this `section` name and `key`).
    /// Else returns `false`.
    ///
    /// NOTE - `section` name, if `Some`, and `key` are not empty.
    ///
    /// NOTE - this is necessary because the `.ini` parser does not keep track internally of all previously parsed keys.
    fn contains_key(&self, section: Option<&str>, key: &str) -> bool;

    /// Adds the `key` / `value` pair to the `section` (if `Some`) or the config root (if `section` is `None`).
    ///
    /// If `overwrite` is `true`, the key is duplicate (i.e. [`contains_key`](#method.contains_key) previously returned `true`)
    /// and the parser is [`configured`](enum.IniDuplicateKeys.html) to [`overwrite`](enum.IniDuplicateKeys.html#variant.Last)
    /// the `key`.
    /// If `overwrite` is `false`, the `key` / `value` pair is added for the first time.
    ///
    /// NOTE - `section` name, if `Some`, and `key` are not empty.
    ///
    /// NOTE - [`add_section`](#method.add_section) was called with this same `section` before at least once.
    fn add_value(
        &mut self,
        section: Option<&str>,
        key: &str,
        value: IniValue<&str>,
        overwrite: bool,
    );

    /// Adds the `key` / value pair to the `section` (if `Some`) or the config root (if `section` is `None`),
    /// where the value is a homogenous `array` of parsed [`.ini values`](enum.IniValue.html).
    ///
    /// If `overwrite` is `true`, the key is duplicate (i.e. [`contains_key`](#method.contains_key) previously returned `true`)
    /// and the parser is [`configured`](enum.IniDuplicateKeys.html) to [`overwrite`](enum.IniDuplicateKeys.html#variant.Last)
    /// the `key`.
    /// If `overwrite` is `false`, the `key` / value pair is added for the first time.
    ///
    /// NOTE - `section` name, if `Some`, and `key` are not empty.
    ///
    /// NOTE - [`add_section`](#method.add_section) was called with this same `section` before at least once.
    fn add_array(
        &mut self,
        section: Option<&str>,
        key: &str,
        array: Vec<IniValue<String>>,
        overwrite: bool,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_key_or_value_char() {
        for c in (b'0'..b'9').map(|c| char::from(c)) {
            assert!(IniParser::is_key_or_value_char(c, false, None));
        }

        for c in (b'a'..b'z').map(|c| char::from(c)) {
            assert!(IniParser::is_key_or_value_char(c, false, None));
        }

        for c in (b'A'..b'Z').map(|c| char::from(c)) {
            assert!(IniParser::is_key_or_value_char(c, false, None));
        }

        assert!(!IniParser::is_key_or_value_char('"', false, None));
        assert!(IniParser::is_key_or_value_char('"', false, Some('\'')));

        assert!(!IniParser::is_key_or_value_char('\'', false, None));
        assert!(IniParser::is_key_or_value_char('\'', false, Some('"')));

        let assert_ini_char = |c| {
            assert!(!IniParser::is_key_or_value_char(c, false, None));
            assert!(IniParser::is_key_or_value_char(c, false, Some('"')));
            assert!(IniParser::is_key_or_value_char(c, false, Some('\'')));
        };

        assert_ini_char(' ');
        assert_ini_char('[');
        assert_ini_char(']');
        assert_ini_char('=');
        assert_ini_char(':');
        assert_ini_char(';');
        assert_ini_char('#');
    }
}
