use {
    super::util::IniPath,
    crate::{util::unwrap_unchecked, *},
    std::{iter::Iterator, str::Chars},
};

/// `.ini` parser FSM states.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum IniParserState {
    /// We started parsing a new line in the current section or the root.
    /// Accept whitespace (including new lines),
    /// section start delimiters ('[') (-> BeforeSection),
    /// valid key chars (-> Key),
    /// escape sequences (if supported) (-> Key),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedKey).
    /// comment delimiters (';' / '#') (if supported) (-> SkipLine).
    StartLine,
    /// We encountered a section start delimiter (or a nested section separator) and started parsing a (nested) section name.
    /// Accept whitespace (except new lines),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedSection),
    /// escape sequences (if supported) (-> Section),
    /// valid key chars (-> Section).
    BeforeSection,
    /// We started parsing an unquoted section name.
    /// Accept nested section separators ('/') (if supported),
    /// escape sequences (if supported),
    /// valid key chars,
    /// section end delimiters (']') (-> SkipLineWhitespaceOrComments),
    /// whitespace (except new lines) (-> AfterSection).
    Section,
    /// We started parsing a quoted section name.
    /// Accept matching string quotes ('"' / '\'') (-> AfterQuotedSection),
    /// escape sequences (if supported),
    /// non-matching string quotes (if supported),
    /// spaces (' '),
    /// valid key chars.
    QuotedSection,
    /// We finished parsing a (maybe quoted) section name and expect a nested section separator or a section end delimiter.
    /// Accept whitespace (except new lines),
    /// section end delimiters (']') (-> SkipLineWhitespaceOrComments),
    /// nested section separators ('/') (if supported) -> (BeforeSection),
    AfterSection,
    /// We encountered a comment delimiter and skip the rest of the line.
    /// Accept new lines (-> StartLine),
    /// everything else.
    SkipLine,
    /// We finished parsing a section name or a value and expect the next line or the comment delimiter.
    /// Accept new lines (-> StartLine),
    /// whitespace,
    /// comment start delimiters (';' / '#') (if supported) (-> SkipLine).
    SkipLineWhitespaceOrComments,
    /// We started parsing an unquoted key.
    /// Accept valid key chars,
    /// escape sequences (if supported),
    /// key-value separators ('=' / ':') (-> BeforeValue),
    /// whitespace (except new lines) (-> KeyValueSeparator).
    Key,
    /// We started parsing a quoted key.
    /// Accept valid key chars,
    /// escape sequences (if supported),
    /// non-matching string quotes (if supported),
    /// spaces (' '),
    /// matching string quotes ('"' / '\'') (-> KeyValueSeparator).
    QuotedKey,
    /// We finished parsing a key and expect a key-value separator.
    /// Accept key-value separators ('=' / ':') (-> BeforeValue),
    /// whitespace (except new lines).
    KeyValueSeparator,
    /// We finished parsing a key-value separator and expect a value (or a new line).
    /// Accept whitespace (except new lines (->StartLine)),
    /// inline comment delimiters (';' / '#') (if supported) (-> SkipLine),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedValue),
    /// escape sequences (if supported) (-> Value),
    /// array start delimiters (if supported) (-> BeforeArrayValue),
    /// valid value chars (-> Value).
    BeforeValue,
    /// We started parsing an unquoted value.
    /// Accept whitespace (-> SkipLineWhitespaceOrComments)
    /// (including new lines (-> StartLine)),
    /// inline comment delimiters (';' / '#') (if supported) (-> SkipLine),
    /// escape sequences (if supported),
    /// valid value chars.
    Value,
    /// We started parsing a quoted value.
    /// Accept matching string quotes ('"' / '\'') (-> SkipLineWhitespaceOrComments),
    /// spaces (' '),
    /// non-matching string quotes (if supported),
    /// escape sequences (if supported),
    /// valid value chars.
    QuotedValue,
    /// We started parsing an array, or finished parsing a previous array value and separator,
    /// and expect the new value or the end of the array.
    /// Accept whitespace (except new lines),
    /// array end delimiters (-> SkipLineWhitespaceOrComments),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedArrayValue),
    /// escape sequences (if supported) (-> ArrayValue),
    /// valid value chars (-> ArrayValue).
    BeforeArrayValue,
    /// We started parsing an unquoted array value.
    /// Accept whitespace (except new lines) (-> AfterArrayValue),
    /// array value separators (-> BeforeArrayValue),
    /// array end delimiters (-> SkipLineWhitespaceOrComments),
    /// escape sequences (if supported) (-> ArrayValue),
    /// valid value chars (-> ArrayValue).
    ArrayValue,
    /// We started parsing a quoted array value.
    /// Accept matching string quotes ('"' / '\'') (-> AfterArrayValue),
    /// spaces (' '),
    /// non-matching string quotes (if supported),
    /// escape sequences (if supported),
    /// valid value chars.
    QuotedArrayValue,
    /// We finished parsing a previous array value
    /// and expect the array value separator or the end of the array.
    /// Accept whitespace (except new lines),
    /// array value separators (-> BeforeArrayValue),
    /// array end delimiters (-> SkipLineWhitespaceOrComments).
    AfterArrayValue,
}

/// Parses the `.ini` config string, using the user-provided parsing options
/// and the [`event handler`](trait.IniConfig.html) object.
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
    /// Creates a new [`parser`](struct.IniParser.html) from the `.ini` config `string`
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

    /// Maximum supported depth of nested sections.
    /// If `0`, sections are not supported at all.
    /// If `1`, one level of sections is supported; forward slashes (`'/'`) are treated as normal section name character.
    /// If `>1`, nested sections are supported; section names which contain forward slashes (`'/'`) are treated as paths.
    ///
    /// Default: `1`.
    pub fn nested_section_depth(mut self, nested_section_depth: u32) -> Self {
        self.options.nested_section_depth = nested_section_depth;
        self
    }

    /// Consumes the parser and tries to parse the `.ini` config string, filling the passed `config` event handler.
    pub fn parse<'a, C: IniConfig>(mut self, config: &mut C) -> Result<(), IniError<'a>> {
        use IniErrorKind::*;

        self.validate_options();

        // Scratch buffer for sections / keys / values.
        let mut buffer = String::new();

        // Current nested section path, if any.
        // Contains at most one section name if nested sections are not supported.
        let mut path = IniPath::new();

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

        // Scratch buffer for ASCII / Unicode escape sequences, if supported.
        let mut escape_sequence_buffer = if self.options.escape {
            String::with_capacity(4)
        } else {
            String::new()
        };

        let mut array_type: Option<IniValueType> = None;

        // Read the chars until EOF, process according to current state.
        while let Some(c) = self.next() {
            match self.state {
                IniParserState::StartLine => {
                    is_key_unique = true;
                    skip_value = false;

                    // Skip whitespace at the start of the line (including new lines).
                    if c.is_whitespace() {

                        // Section start delimiter - parse the section name.
                    } else if self.is_section_start(c) {
                        // Return an error if we don't support sections.
                        if self.options.nested_section_depth == 0 {
                            return Err(self.error(IniErrorKind::NestedSectionDepthExceeded));
                        }

                        // Clear the current path.
                        while let Some(section) = path.last() {
                            // We didn't call `start_section()` if we skipped it, so don't call `end_section`.
                            if !skip_section {
                                config.end_section(section);
                            } else {
                                skip_section = false;
                            }
                            path.pop();
                        }

                        skip_section = false;

                        self.state = IniParserState::BeforeSection;

                    // Line comment (if supported) - skip the rest of the line.
                    } else if self.is_comment_char(c) {
                        self.state = IniParserState::SkipLine;

                    // String quote (if supported) - parse the key in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(c) {
                        debug_assert!(quote.is_none());
                        quote.replace(c);

                        self.state = IniParserState::QuotedKey;

                    // Escaped char (if supported) - parse the escape sequence as the key.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
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
                    } else if self.is_key_or_value_char(c, false, quote) {
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
                    debug_assert!(self.nested_sections() || path.is_empty());

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

                    // Nested section separator (if supported) - empty parent section names are not allowed.
                    } else if self.is_nested_section_separator(c) {
                        return Err(self.error(EmptySectionName(path.to_config_path())));

                    // Escaped char (if supported) - start parsing the section name.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
                            // Parsed an escaped char - keep parsing the section name.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);

                                self.state = IniParserState::Section;
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid section name char (same rules as key chars) - start parsing the section name.
                    } else if self.is_key_or_value_char(c, false, quote) {
                        buffer.push(c);

                        self.state = IniParserState::Section;

                    // Section end delimiter - empty section names not allowed.
                    } else if self.is_section_end(c) {
                        return Err(self.error(EmptySectionName(path.to_config_path())));

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName(c)));
                    }
                }
                IniParserState::Section => {
                    debug_assert!(quote.is_none());

                    // New line before the section delimiter - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInSectionName));

                    // Nested section separator (if supported) - finish the current section, keep parsing the nested section.
                    } else if self.is_nested_section_separator(c) {
                        // Empty section names are not allowed.
                        let section = NonEmptyStr::new(&buffer)
                            .ok_or(self.error(EmptySectionName(path.to_config_path())))?;

                        if path.len() + 1 >= self.options.nested_section_depth {
                            return Err(self.error(NestedSectionDepthExceeded));
                        }

                        path.push(section);

                        // The path must already exist in the config.
                        if config.contains_key(section) != Ok(true) {
                            return Err(
                                self.error_offset(InvalidParentSection(path.to_config_path()))
                            );
                        }

                        // Start the parent section in the config.
                        config.start_section(section, false);

                        buffer.clear();

                        self.state = IniParserState::BeforeSection;

                    // Escaped char (if supported) - keep parsing the section name.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(true, &mut escape_sequence_buffer)? {
                            // Parsed an escaped char - keep parsing the section name.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid section name char - keep parsing the section name.
                    } else if self.is_key_or_value_char(c, true, quote) {
                        buffer.push(c);

                    // Section end delimiter - finish the section name, skip the rest of the line.
                    } else if self.is_section_end(c) {
                        debug_assert!(path.len() <= self.options.nested_section_depth);

                        // Empty section names not allowed.
                        let section = NonEmptyStr::new(&buffer)
                            .ok_or(self.error(EmptySectionName(path.to_config_path())))?;

                        // Try to add the section to the config at the current path.
                        skip_section = self.start_section(config, section, &path)?;

                        path.push(section);
                        buffer.clear();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Whitespace after section name (new lines handled above) - skip it,
                    // parse the nested section separator or the section end delimiter.
                    } else if c.is_whitespace() {
                        self.state = IniParserState::AfterSection;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName(c)));
                    }
                }
                IniParserState::QuotedSection => {
                    // Must succeed - we only enter this state after encountering a quote.
                    let cur_quote = unwrap_unchecked(quote);

                    // New line before the closing quotes - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInSectionName));

                    // Closing quotes - keep parsing until the nested section separator or section end delimiter.
                    } else if c == cur_quote {
                        quote.take();

                        self.state = IniParserState::AfterSection;

                    // Escaped char (if supported) - keep parsing the section name.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
                            // Parsed an escaped char - keep parsing the section name.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Non-matching quotes - keep parsing the section.
                    } else if self.is_non_matching_string_quote_char(cur_quote, c) {
                        buffer.push(c);

                    // Space or valid value char - keep parsing the section.
                    } else if c == ' ' || self.is_key_or_value_char(c, true, quote) {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName(c)));
                    }
                }
                IniParserState::AfterSection => {
                    // Skip whitespace.
                    if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(UnexpectedNewLineInSectionName));
                        }

                    // Section end delimiter - skip the rest of the line.
                    } else if self.is_section_end(c) {
                        // Empty section names are not allowed.
                        let section = NonEmptyStr::new(&buffer)
                            .ok_or(self.error_offset(EmptySectionName(path.to_config_path())))?;

                        // Try to add the section to the config at the current path.
                        skip_section = self.start_section(config, section, &path)?;

                        path.push(section);
                        buffer.clear();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Nested section separator (if supported) - start parsing the nested section name.
                    } else if self.is_nested_section_separator(c) {
                        if (path.len() + 1) >= self.options.nested_section_depth {
                            return Err(self.error(NestedSectionDepthExceeded));
                        }

                        // Empty section names are not allowed.
                        let section = NonEmptyStr::new(&buffer)
                            .ok_or(self.error(EmptySectionName(path.to_config_path())))?;

                        path.push(section);

                        // The path must already exist in the config.
                        if config.contains_key(section) != Ok(true) {
                            return Err(
                                self.error_offset(InvalidParentSection(path.to_config_path()))
                            );
                        }

                        // Start the parent section in the config.
                        config.start_section(section, false);

                        buffer.clear();

                        self.state = IniParserState::BeforeSection;

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
                            // Must succeed.
                            unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key"),
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
                            // Must succeed.
                            unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key"),
                            skip_section,
                            &mut skip_value,
                            &mut is_key_unique,
                        )?;

                        self.state = IniParserState::KeyValueSeparator;

                    // Escaped char (if supported) - keep parsing the key.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
                            // Parsed an escaped char - keep parsing the key.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid key char - keep parsing the key.
                    } else if self.is_key_or_value_char(c, false, quote) {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInKey(c)));
                    }
                }
                IniParserState::QuotedKey => {
                    debug_assert!(key.is_empty());

                    // Must succeed - we only enter this state after encountering a quote.
                    let cur_quote = unwrap_unchecked(quote);

                    // New line before the closing quotes - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInKey));

                    // Closing quotes - finish the key, parse the separator.
                    } else if c == cur_quote {
                        quote.take();

                        std::mem::swap(&mut key, &mut buffer);

                        // Empty keys are not allowed.
                        let key = NonEmptyStr::new(&key).ok_or(self.error(EmptyKey))?;

                        self.check_is_key_duplicate(
                            config,
                            key,
                            skip_section,
                            &mut skip_value,
                            &mut is_key_unique,
                        )?;

                        self.state = IniParserState::KeyValueSeparator;

                    // Escaped char (if supported) - keep parsing the key.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
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
                    } else if c == ' ' || self.is_key_or_value_char(c, false, quote) {
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
                    // We have at least one key character already parsed.
                    debug_assert!(!key.is_empty());

                    // Skip the whitespace before the value.
                    if c.is_whitespace() {
                        // Unless it's a new line - the value is empty.
                        if self.is_new_line(c) {
                            self.add_value_to_config(
                                config,
                                // Must succeed.
                                unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key"),
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
                            // Must succeed.
                            unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key"),
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
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
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
                        // Must succeed.
                        let array_key = unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key");

                        Self::add_array_to_config(
                            config,
                            array_key,
                            skip_section | skip_value,
                            is_key_unique,
                        );

                        path.push(array_key);

                        self.state = IniParserState::BeforeArrayValue;

                    // Valid value char - start parsing the unquoted value.
                    } else if self.is_key_or_value_char(c, false, quote) {
                        buffer.push(c);

                        self.state = IniParserState::Value;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::Value => {
                    debug_assert!(!buffer.is_empty());
                    // We have at least one key character already parsed.
                    debug_assert!(!key.is_empty());

                    // Whitespace - finish the value.
                    if c.is_whitespace() {
                        self.add_value_to_config(
                            config,
                            // Must succeed.
                            unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key"),
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
                            // Must succeed.
                            unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key"),
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
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
                            // Parsed an escaped char - keep parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid value char - keep parsing the value.
                    } else if self.is_key_or_value_char(c, false, quote) {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::QuotedValue => {
                    // We have at least one key character already parsed.
                    debug_assert!(!key.is_empty());

                    // Must succeed - we only enter this state after encountering a quote.
                    let cur_quote = unwrap_unchecked(quote);

                    // New line before the closing quotes - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInQuotedValue));

                    // Closing quotes - finish the value (may be empty), skip the rest of the line.
                    } else if c == cur_quote {
                        self.add_value_to_config(
                            config,
                            // Must succeed.
                            unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key"),
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
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
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
                    } else if c == ' ' || self.is_key_or_value_char(c, false, quote) {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::BeforeArrayValue => {
                    debug_assert!(buffer.is_empty());
                    // We have at least one key character already parsed.
                    debug_assert!(!key.is_empty());
                    // We have at least the array key in the path.
                    debug_assert!(!path.is_empty());

                    // Skip the whitespace before the array value.
                    if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(UnexpectedNewLineInArray));
                        }

                    // Array end delimiter - finish the array, skip the rest of the line.
                    } else if self.is_array_end(c) {
                        config.end_array(unwrap_unchecked(path.last()));

                        // Pop the array key off the path, reset the array type.
                        path.pop();
                        array_type.take();

                        key.clear();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // String quote - parse the string array value in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(c) {
                        // Make sure the array is empty or contains strings (is not mixed).
                        if let Some(array_type) = array_type {
                            if !array_type.is_compatible(IniValueType::String) {
                                return Err(self.error(IniErrorKind::MixedArray));
                            }
                        }

                        debug_assert!(quote.is_none());
                        quote.replace(c);

                        self.state = IniParserState::QuotedArrayValue;

                    // Escaped char (if supported) - parse the escape sequence, start parsing the array value.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
                            // Parsed an escaped char - keep parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);

                                self.state = IniParserState::ArrayValue;
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid value char - start parsing the unquoted array value.
                    } else if self.is_key_or_value_char(c, false, quote) {
                        buffer.push(c);

                        self.state = IniParserState::ArrayValue;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInArray(c)));
                    }
                }
                IniParserState::ArrayValue => {
                    debug_assert!(!buffer.is_empty());
                    // We have at least one key character already parsed.
                    debug_assert!(!key.is_empty());
                    // We have at least the array key in the path.
                    debug_assert!(!path.is_empty());

                    // Whitespace - finish the current array value,
                    // parse the array value separator / array end delimiter.
                    if c.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(c) {
                            return Err(self.error_offset(IniErrorKind::UnexpectedNewLineInArray));
                        }

                        self.add_value_to_array(
                            config,
                            &mut array_type,
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
                            config,
                            &mut array_type,
                            &buffer,
                            false,
                            skip_value | skip_section,
                        )?;

                        buffer.clear();

                        self.state = IniParserState::BeforeArrayValue;

                    // Array end delimiter - add the value to the array, finish the array, skip the rest of the line.
                    } else if self.is_array_end(c) {
                        self.add_value_to_array(
                            config,
                            &mut array_type,
                            &buffer,
                            false,
                            skip_value | skip_section,
                        )?;

                        buffer.clear();

                        // Must succeed.
                        config.end_array(unwrap_unchecked_msg(path.last(), "empty key"));

                        // Pop the array key off the path, reset the array type.
                        path.pop();
                        array_type.take();

                        key.clear();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
                            // Parsed an escaped char - keep parsing the array value.
                            ParseEscapeSequenceResult::EscapedChar(c) => {
                                buffer.push(c);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid value char - keep parsing the array value.
                    } else if self.is_key_or_value_char(c, false, quote) {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::QuotedArrayValue => {
                    // Must succeed - we only enter this state after encountering a quote.
                    let cur_quote = unwrap_unchecked(quote);

                    // New line before the closing quotes - error.
                    if self.is_new_line(c) {
                        return Err(self.error_offset(UnexpectedNewLineInQuotedValue));

                    // Closing quotes - finish the array value (may be empty),
                    // parse the array value separator / array end delimiter.
                    } else if c == cur_quote {
                        self.add_value_to_array(
                            config,
                            &mut array_type,
                            &buffer,
                            true,
                            skip_value | skip_section,
                        )?;
                        buffer.clear();

                        quote.take();

                        self.state = IniParserState::AfterArrayValue;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(c) {
                        match self.parse_escape_sequence(false, &mut escape_sequence_buffer)? {
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
                    } else if c == ' ' || self.is_key_or_value_char(c, false, quote) {
                        buffer.push(c);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue(c)));
                    }
                }
                IniParserState::AfterArrayValue => {
                    // We have at least one key character already parsed.
                    debug_assert!(!key.is_empty());
                    // We have at least the array key in the path.
                    debug_assert!(!path.is_empty());

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
                        // Must succeed.
                        config.end_array(unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key"));

                        key.clear();

                        // Pop the array key off the path, reset the array type.
                        path.pop();
                        array_type.take();

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
                // We have at least one key character already parsed.
                debug_assert!(!key.is_empty());

                self.add_value_to_config(
                    config,
                    // Must succeed.
                    unwrap_unchecked_msg(NonEmptyStr::new(&key), "empty key"),
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

        while let Some(section) = path.last() {
            // We didn't call `start_section()` if we skipped it, so don't call `end_section`.
            if !skip_section {
                config.end_section(section);
            } else {
                skip_section = false;
            }
            path.pop();
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
    fn error<'a>(&self, error: IniErrorKind<'a>) -> IniError<'a> {
        IniError {
            line: self.line,
            column: self.column,
            error,
        }
    }

    /// Error helper method.
    fn error_offset<'a>(&self, error: IniErrorKind<'a>) -> IniError<'a> {
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

    /// Is the character a section start delimiter?
    fn is_section_start(&self, val: char) -> bool {
        val == '['
    }

    /// Is the character a section end delimiter?
    fn is_section_end(&self, val: char) -> bool {
        val == ']'
    }

    fn nested_sections(&self) -> bool {
        self.options.nested_section_depth > 1
    }

    /// Is the character a nested section separator?
    fn is_nested_section_separator(&self, val: char) -> bool {
        self.nested_sections() && val == '/'
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

    /// Reads up to 4 following characters and tries to parse them as an escape sequence.
    /// `in_unquoted_section` is `true` if we are parsing an unquoted ini section.
    fn parse_escape_sequence<'a>(
        &mut self,
        in_unquoted_section: bool,
        escape_sequence_buffer: &mut String,
    ) -> Result<ParseEscapeSequenceResult, IniError<'a>> {
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

            // `.ini` nested section separator, if supported, must be escaped in unquoted section names.
            // If nested sections are unsupported, or if in a key, value or a quoted section,
            // it's just a normal character and must not be escaped.
            Some('/') if (self.nested_sections() && in_unquoted_section) => Ok(EscapedChar('/')),

            // 2 hexadecimal ASCII values.
            Some('x') => {
                escape_sequence_buffer.clear();

                for _ in 0..2 {
                    match self.next() {
                        None => return Err(self.error(UnexpectedEndOfFileInASCIIEscapeSequence)),
                        Some('\n') | Some('\r') => {
                            return Err(self.error_offset(UnexpectedNewLineInASCIIEscapeSequence))
                        }
                        Some(c) => escape_sequence_buffer.push(c),
                    }
                }

                Ok(EscapedChar(
                    std::char::from_u32(
                        u32::from_str_radix(&escape_sequence_buffer, 16)
                            .map_err(|_| self.error(InvalidASCIIEscapeSequence))?,
                    )
                    .ok_or(self.error(InvalidASCIIEscapeSequence))?,
                ))
            }

            // 4 hexadecimal Unicode values.
            Some('u') => {
                escape_sequence_buffer.clear();

                for _ in 0..4 {
                    match self.next() {
                        None => return Err(self.error(UnexpectedEndOfFileInUnicodeEscapeSequence)),
                        Some('\n') | Some('\r') => {
                            return Err(self.error_offset(UnexpectedNewLineInUnicodeEscapeSequence))
                        }
                        Some(c) => escape_sequence_buffer.push(c),
                    }
                }

                Ok(EscapedChar(
                    std::char::from_u32(
                        u32::from_str_radix(&escape_sequence_buffer, 16)
                            .map_err(|_| self.error(InvalidUnicodeEscapeSequence))?,
                    )
                    .ok_or(self.error(InvalidUnicodeEscapeSequence))?,
                ))
            }

            Some(c) => Err(self.error(InvalidEscapeCharacter(c))),
        }
    }

    /// Returns `Ok(true)` if we need to skip the current section;
    /// else returns `Ok(false)`.
    fn start_section<'a, C: IniConfig>(
        &self,
        config: &mut C,
        section: NonEmptyStr<'_>,
        path: &IniPath,
    ) -> Result<bool, IniError<'a>> {
        let key_already_exists = config.contains_key(section);

        // Section already exists.
        if let Ok(true) = key_already_exists {
            match self.options.duplicate_sections {
                // We don't support duplicate sections - error.
                IniDuplicateSections::Forbid => {
                    let mut path = path.to_config_path();
                    path.0.push(section.as_ref().to_owned().into());

                    return Err(self.error(IniErrorKind::DuplicateSection(path)));
                }
                // Skip this section.
                IniDuplicateSections::First => Ok(true),
                // Overwrite the previous instance of the section with the new one.
                IniDuplicateSections::Last => {
                    config.start_section(section, true);
                    Ok(false)
                }
                // Just add the new key/value pairs to the existing section.
                IniDuplicateSections::Merge => {
                    config.start_section(section, false);
                    Ok(false)
                }
            }

        // Section does not exist in the config.
        } else {
            // Key already exists.
            if let Ok(false) = key_already_exists {
                match self.options.duplicate_keys {
                    // We don't support duplicate keys - error.
                    IniDuplicateKeys::Forbid => {
                        let mut path = path.to_config_path();
                        path.0.push(section.as_ref().to_owned().into());

                        return Err(self.error_offset(IniErrorKind::DuplicateKey(
                            section.as_ref().to_owned().into(),
                        )));
                    }
                    // Skip this section.
                    IniDuplicateKeys::First => Ok(true),
                    // Overwrite the previous value with the new one.
                    IniDuplicateKeys::Last => {
                        config.start_section(section, true);
                        Ok(false)
                    }
                }
            // Key does not exist - add the section.
            } else {
                config.start_section(section, false);
                Ok(false)
            }
        }
    }

    /// Parses a string `value` and adds it to the `config`'s current section at `key`.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn add_value_to_config<'a, C: IniConfig>(
        &self,
        config: &mut C,
        key: NonEmptyStr<'_>,
        value: &str,
        quoted: bool,
        skip: bool,
        is_key_unique: bool,
    ) -> Result<(), IniError<'a>> {
        if !skip {
            config.add_value(key, self.parse_value_string(value, quoted)?, !is_key_unique);
        }

        Ok(())
    }

    /// Adds an empty array to the `config`'s current section at `key`.
    fn add_array_to_config<C: IniConfig>(
        config: &mut C,
        key: NonEmptyStr<'_>,
        skip: bool,
        is_key_unique: bool,
    ) {
        if !skip {
            config.start_array(key, !is_key_unique);
        }
    }

    /// Parses a string `value` and adds it to the `config`'s current array.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn add_value_to_array<'a, C: IniConfig>(
        &self,
        config: &mut C,
        array_type: &mut Option<IniValueType>,
        value: &'s str,
        quoted: bool,
        skip: bool,
    ) -> Result<(), IniError<'a>> {
        if skip {
            return Ok(());
        }

        let value = self.parse_value_string(value, quoted)?;
        let value_type = value.get_ini_type();

        // Make sure the array is not mixed.
        if let Some(array_type) = array_type {
            if !array_type.is_compatible(value_type) {
                return Err(self.error_offset(IniErrorKind::MixedArray));
            }
        } else {
            array_type.replace(value_type);
        }

        config.add_array_value(value);

        Ok(())
    }

    fn try_parse_integer(value: &str) -> Option<i64> {
        if value.is_empty() {
            None
        } else {
            // Explicit sign.
            let (sign, value) = {
                if value.starts_with("+") {
                    (1, &value[1..])
                } else if value.starts_with("-") {
                    (-1, &value[1..])
                } else {
                    (1, value)
                }
            };

            // Radix.
            let (radix, value) = {
                // Hexadecimal.
                if let Some(value) = value.strip_prefix("0x") {
                    (16, value)
                // Octal.
                } else if let Some(value) = value.strip_prefix("0o") {
                    (8, value)
                // Else assume decimal.
                } else {
                    (10, value)
                }
            };

            i64::from_str_radix(value, radix).ok().map(|int| sign * int)
        }
    }

    /// Parses a string `value`.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn parse_value_string<'a, 'v>(
        &self,
        value: &'v str,
        quoted: bool,
    ) -> Result<IniValue<&'v str>, IniError<'a>> {
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
        } else if let Some(value) = { Self::try_parse_integer(value) } {
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
    /// sets `is_key_unique` to `true` if the key is not contained in `config`'s current section.
    fn check_is_key_duplicate<'a, 'k, C: IniConfig>(
        &self,
        config: &C,
        key: NonEmptyStr<'k>,
        skip_section: bool,
        skip_value: &mut bool,
        is_key_unique: &mut bool,
    ) -> Result<(), IniError<'a>> {
        use IniErrorKind::*;

        if skip_section {
            *skip_value = true;
            *is_key_unique = false;

            return Ok(());
        }

        let is_unique = config.contains_key(key).is_err();

        match self.options.duplicate_keys {
            IniDuplicateKeys::Forbid => {
                if is_unique {
                    *skip_value = false;
                    *is_key_unique = true;

                    Ok(())
                } else {
                    Err(self.error_offset(DuplicateKey(key.as_ref().to_string().into())))
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

    fn is_key_or_value_char(&self, val: char, in_section: bool, quote: Option<char>) -> bool {
        Self::is_key_or_value_char_impl(
            val,
            self.options.escape,
            self.nested_sections(),
            in_section,
            quote,
        )
    }

    /// Returns `true` if the `val` character is a valid key/value/section name character and does not have to be escaped.
    /// Otherwise, `val` must be escaped (preceded by a backslash) when used in keys/values/section names.
    fn is_key_or_value_char_impl(
        val: char,
        escape: bool,
        nested_sections: bool,
        in_section: bool,
        quote: Option<char>,
    ) -> bool {
        if let Some(quote) = quote {
            debug_assert!(quote == '"' || quote == '\'', "invalid quote character");
        }

        match val {
            // Escape char (backslash) must be escaped if escape sequences are supported.
            '\\' if escape => false,

            // Non-matching quotes don't need to be escaped in quoted strings.
            '"' => quote == Some('\''),
            '\'' => quote == Some('"'),

            // Space and special `.ini` characters in key/value/section strings
            // (except string quotes, handled above) don't need to be escaped in quoted strings.
            ' ' | '[' | ']' | '=' | ':' | ';' | '#' => quote.is_some(),

            // Nested section separators, if supported, must be escaped in unquoted section names.
            '/' if (nested_sections && in_section) => quote.is_some(),

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
#[derive(Clone, Debug)]
pub enum IniValue<S> {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(S),
}

impl<S> IniValue<S> {
    fn get_ini_type(&self) -> IniValueType {
        match self {
            IniValue::Bool(_) => IniValueType::Bool,
            IniValue::I64(_) => IniValueType::I64,
            IniValue::F64(_) => IniValueType::F64,
            IniValue::String(_) => IniValueType::String,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum IniValueType {
    Bool,
    I64,
    F64,
    String,
}

impl IniValueType {
    pub(crate) fn is_compatible(self, other: IniValueType) -> bool {
        use IniValueType::*;

        match self {
            Bool => other == Bool,
            I64 => (other == I64) || (other == F64),
            F64 => (other == I64) || (other == F64),
            String => other == String,
        }
    }
}

/// A trait which represents the config being filled by the [`.ini parser`](struct.IniParser.html)
/// during the call to [`parse`](struct.IniParser.html#method.parse).
/// Handles the events generated by the [`.ini parser`](struct.IniParser.html).
pub trait IniConfig {
    /// Returns `Ok(_)` if the current section already contains the `key`.
    /// The returned result value is `true` if the value is a section, `false` otherwise.
    /// Else returns an `Err(())` if the current section does not contain the `key`.
    ///
    /// NOTE - this is necessary because the [`.ini parser`](struct.IniParser.html) does not keep track internally of all previously parsed keys.
    fn contains_key(&self, key: NonEmptyStr<'_>) -> Result<bool, ()>;

    /// Adds the `key` / `value` pair to the current section.
    ///
    /// If `overwrite` is `true`, the key is duplicate (i.e. [`contains_key`](#method.contains_key)
    /// previously returned `Ok(_)` for this `key`)
    /// and the parser is [`configured`](enum.IniDuplicateKeys.html)
    /// to [`overwrite`](enum.IniDuplicateKeys.html#variant.Last) the `key`.
    /// If `overwrite` is `false`, the `key` / `value` pair is added for the first time.
    fn add_value(&mut self, key: NonEmptyStr<'_>, value: IniValue<&str>, overwrite: bool);

    /// Adds the `section` to the current section and makes it the current section for the following calls to
    /// [`contains_key`](#method.contains_key), [`add_value`](#method.add_value), [`start_array`](#method.start_array),
    /// [`end_section`](#method.end_section).
    /// Pushes it onto the LIFO stack of sections.
    ///
    /// If `overwrite` is `true`, the section is duplicate (i.e. [`contains_key`](#method.contains_key)
    /// previously returned `Ok(_)` for this `section`)
    /// and the parser is [`configured`](enum.IniDuplicateSections.html)
    /// to [`overwrite`](enum.IniDuplicateSections.html#variant.Last) the `section`.
    /// If `overwrite` is `false`, the `section` is added for the first time.
    ///
    /// Will be eventually followed by a call to [`end_section`](#method.end_section) with the same `section` name.
    fn start_section(&mut self, section: NonEmptyStr<'_>, overwrite: bool);

    /// Finishes the current `section`, popping it off the LIFO stack of sections,
    /// making the previous section (if any, or the root section) the current section for the following calls to
    /// [`contains_key`](#method.contains_key), [`add_value`](#method.add_value), [`start_array`](#method.start_array).
    ///
    /// `section` name is guaranteed to match the name used in the previous call to [`start_section`](#method.start_section).
    fn end_section(&mut self, section: NonEmptyStr<'_>);

    /// Adds an empty `array` to the current section, making it the current array for the following calls to
    /// [`add_array_value`](#method.add_array_value), [`end_array`](#method.end_array).
    ///
    /// If `overwrite` is `true`, the `array` key is duplicate (i.e. [`contains_key`](#method.contains_key)
    /// previously returned `Ok(_)` for this `array`) and the parser is [`configured`](enum.IniDuplicateKeys.html)
    /// to [`overwrite`](enum.IniDuplicateKeys.html#variant.Last) the `path`.
    /// If `overwrite` is `false`, the array is added for the first time.
    ///
    /// Will be eventually followed by a call to [`end_array`](#method.end_array) with the same `array` name.
    fn start_array(&mut self, array: NonEmptyStr<'_>, overwrite: bool);

    /// Adds a new `value` to the current array.
    /// `value` is guaranteed to be of valid type (i.e. not mixed) for the array.
    fn add_array_value(&mut self, value: IniValue<&str>);

    /// Finishes the current `array`.
    /// `array` name is guaranteed to match the name used in the previous call to [`start_array`](#method.start_array).
    fn end_array(&mut self, array: NonEmptyStr<'_>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_key_or_value_char() {
        // Alphanumeric characters are always valid key/value chars.

        // Digits.
        for c in (b'0'..b'9').map(|c| char::from(c)) {
            assert!(IniParser::is_key_or_value_char_impl(
                c, /* escape */ false, /* nested_sections */ false,
                /* in_section */ false, /* quote */ None
            ));
        }

        // ASCII chars.
        for c in (b'a'..b'z').map(|c| char::from(c)) {
            assert!(IniParser::is_key_or_value_char_impl(
                c, /* escape */ false, /* nested_sections */ false,
                /* in_section */ false, /* quote */ None
            ));
        }

        for c in (b'A'..b'Z').map(|c| char::from(c)) {
            assert!(IniParser::is_key_or_value_char_impl(
                c, /* escape */ false, /* nested_sections */ false,
                /* in_section */ false, /* quote */ None
            ));
        }

        // Other alphabetic chars.
        assert!(IniParser::is_key_or_value_char_impl(
            'á', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));
        assert!(IniParser::is_key_or_value_char_impl(
            '愛', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));

        // Double quotes are only valid when single-quoted.
        assert!(!IniParser::is_key_or_value_char_impl(
            '"', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));
        assert!(IniParser::is_key_or_value_char_impl(
            '"',
            /* escape */ false,
            /* nested_sections */ false,
            /* in_section */ false,
            /* quote */ Some('\'')
        ));

        // Single quotes are only valid when double-quoted.
        assert!(!IniParser::is_key_or_value_char_impl(
            '\'', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));
        assert!(IniParser::is_key_or_value_char_impl(
            '\'',
            /* escape */ false,
            /* nested_sections */ false,
            /* in_section */ false,
            /* quote */ Some('"')
        ));

        // .ini special chars are only valid when quoted.
        let assert_ini_char = |c| {
            assert!(!IniParser::is_key_or_value_char_impl(
                c, /* escape */ false, /* nested_sections */ false,
                /* in_section */ false, None
            ));
            assert!(IniParser::is_key_or_value_char_impl(
                c,
                /* escape */ false,
                /* nested_sections */ false,
                /* in_section */ false,
                /* quote */ Some('"')
            ));
            assert!(IniParser::is_key_or_value_char_impl(
                c,
                /* escape */ false,
                /* nested_sections */ false,
                /* in_section */ false,
                /* quote */ Some('\'')
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
        assert!(IniParser::is_key_or_value_char_impl(
            '/', /* escape */ false, /* nested_sections */ false,
            /* in_section */ false, /* quote */ None
        ));

        // Valid outside of section names ...
        assert!(IniParser::is_key_or_value_char_impl(
            '/', /* escape */ false, /* nested_sections */ true,
            /* in_section */ false, /* quote */ None
        ));

        // ... but invalid in unquoted section names ...
        assert!(!IniParser::is_key_or_value_char_impl(
            '/', /* escape */ false, /* nested_sections */ true,
            /* in_section */ true, /* quote */ None
        ));

        // ... and valid in quoted section names.
        assert!(IniParser::is_key_or_value_char_impl(
            '/',
            /* escape */ false,
            /* nested_sections */ true,
            /* in_section */ true,
            /* quote */ Some('"')
        ));
        assert!(IniParser::is_key_or_value_char_impl(
            '/',
            /* escape */ false,
            /* nested_sections */ true,
            /* in_section */ true,
            /* quote */ Some('\'')
        ));
    }

    #[test]
    fn try_parse_integer() {
        assert_eq!(IniParser::try_parse_integer("7").unwrap(), 7);
        assert_eq!(IniParser::try_parse_integer("+7").unwrap(), 7);
        assert_eq!(IniParser::try_parse_integer("-7").unwrap(), -7);

        assert_eq!(IniParser::try_parse_integer("0x17").unwrap(), 23);
        assert_eq!(IniParser::try_parse_integer("+0x17").unwrap(), 23);
        assert_eq!(IniParser::try_parse_integer("-0x17").unwrap(), -23);

        assert_eq!(IniParser::try_parse_integer("0o17").unwrap(), 15);
        assert_eq!(IniParser::try_parse_integer("+0o17").unwrap(), 15);
        assert_eq!(IniParser::try_parse_integer("-0o17").unwrap(), -15);

        assert!(IniParser::try_parse_integer("-").is_none());
        assert!(IniParser::try_parse_integer("+").is_none());
        assert!(IniParser::try_parse_integer("0x").is_none());
        assert!(IniParser::try_parse_integer("+0x").is_none());
        assert!(IniParser::try_parse_integer("-0x").is_none());
        assert!(IniParser::try_parse_integer("0o").is_none());
        assert!(IniParser::try_parse_integer("+0o").is_none());
        assert!(IniParser::try_parse_integer("-0o").is_none());

        assert!(IniParser::try_parse_integer("+7.").is_none());
        assert!(IniParser::try_parse_integer("-7.").is_none());
        assert!(IniParser::try_parse_integer("7.").is_none());
        assert!(IniParser::try_parse_integer(".0").is_none());
        assert!(IniParser::try_parse_integer("+.0").is_none());
        assert!(IniParser::try_parse_integer("-.0").is_none());
        assert!(IniParser::try_parse_integer("7e2").is_none());
        assert!(IniParser::try_parse_integer("7e+2").is_none());
        assert!(IniParser::try_parse_integer("7e-2").is_none());
        assert!(IniParser::try_parse_integer("7.0e2").is_none());
        assert!(IniParser::try_parse_integer("7.0e+2").is_none());
        assert!(IniParser::try_parse_integer("7.0e-2").is_none());
    }
}
