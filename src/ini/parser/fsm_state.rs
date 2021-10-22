use {super::*, crate::*};

/// `.ini` parser FSM states.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum IniParserFSMState {
    /// We started parsing a new line in the current section or the root.
    /// Accept whitespace (including new lines),
    /// section start delimiters (`'['`) (-> BeforeSection),
    /// valid key chars (-> Key),
    /// escape sequences (if supported) (-> Key),
    /// string quotes (`'"'` / `'\'`') (if supported) (-> QuotedKey).
    /// comment delimiters (`';'` / `'#'`) (if supported) (-> SkipLine).
    StartLine,
    /// We encountered a section start delimiter (or a nested section separator) and started parsing a (nested) section name.
    /// Accept whitespace (except new lines),
    /// string quotes (`'"'` / `'\'`') (if supported) (-> QuotedSection),
    /// escape sequences (if supported) (-> Section),
    /// valid key chars (-> Section).
    BeforeSection,
    /// We started parsing an unquoted section name.
    /// Accept nested section separators (`'/'`) (if supported),
    /// escape sequences (if supported),
    /// valid key chars,
    /// section end delimiters (`']'`) (-> SkipLineWhitespaceOrComments),
    /// whitespace (except new lines) (-> AfterSection).
    Section,
    /// We started parsing a quoted section name.
    /// Accept matching string quotes (`'"'` / `'\'`') (-> AfterQuotedSection),
    /// escape sequences (if supported),
    /// non-matching string quotes (if supported),
    /// spaces (`' '`),
    /// valid key chars.
    /// Contains the opening quote.
    QuotedSection(StringQuote),
    /// We finished parsing a (maybe quoted) section name and expect a nested section separator or a section end delimiter.
    /// Accept whitespace (except new lines),
    /// section end delimiters (`']'`) (-> SkipLineWhitespaceOrComments),
    /// nested section separators (`'/'`) (if supported) -> (BeforeSection),
    AfterSection,
    /// We encountered a comment delimiter and skip the rest of the line.
    /// Accept new lines (-> StartLine),
    /// skip everything else.
    SkipLine,
    /// We finished parsing a section name or a value and expect the next line or the comment delimiter.
    /// Accept new lines (-> StartLine),
    /// whitespace,
    /// comment start delimiters (`';'` / `'#'`) (if supported) (-> SkipLine).
    SkipLineWhitespaceOrComments,
    /// We started parsing an unquoted key.
    /// Accept valid key chars,
    /// escape sequences (if supported),
    /// key-value separators (`'='` / `':'`) (-> BeforeValue),
    /// whitespace (except new lines) (-> KeyValueSeparator).
    Key,
    /// We started parsing a quoted key.
    /// Accept valid key chars,
    /// escape sequences (if supported),
    /// non-matching string quotes (if supported),
    /// spaces (`' '`),
    /// matching string quotes (`'"'` / `'\'`') (-> KeyValueSeparator).
    /// Contains the opening quote.
    QuotedKey(StringQuote),
    /// We finished parsing a key and expect a key-value separator.
    /// Accept key-value separators (`'='` / `':'`) (-> BeforeValue),
    /// whitespace (except new lines).
    KeyValueSeparator,
    /// We finished parsing a key-value separator and expect a value (or a new line).
    /// Accept whitespace (except new lines (->StartLine)),
    /// inline comment delimiters (`';'` / `'#'`) (if supported) (-> SkipLine),
    /// string quotes (`'"'` / `'\'`') (if supported) (-> QuotedValue),
    /// escape sequences (if supported) (-> Value),
    /// array start delimiters (if supported) (-> BeforeArrayValue),
    /// valid value chars (-> Value).
    BeforeValue,
    /// We started parsing an unquoted value.
    /// Accept whitespace (-> SkipLineWhitespaceOrComments)
    /// (including new lines (-> StartLine)),
    /// inline comment delimiters (`';'` / `'#'`) (if supported) (-> SkipLine),
    /// escape sequences (if supported),
    /// valid value chars.
    Value,
    /// We started parsing a quoted value.
    /// Accept matching string quotes (`'"'` / `'\'`') (-> SkipLineWhitespaceOrComments),
    /// spaces (`' '`),
    /// non-matching string quotes (if supported),
    /// escape sequences (if supported),
    /// valid value chars.
    /// Contains the opening quote.
    QuotedValue(StringQuote),
    /// We started parsing an array, or finished parsing a previous array value and separator,
    /// and expect the new value or the end of the array.
    /// Accept whitespace (except new lines),
    /// array end delimiters (-> SkipLineWhitespaceOrComments),
    /// string quotes (`'"'` / `'\'`') (if supported) (-> QuotedArrayValue),
    /// escape sequences (if supported) (-> ArrayValue),
    /// valid value chars (-> ArrayValue).
    /// Contains the current array type, if any.
    BeforeArrayValue(Option<IniValueType>),
    /// We started parsing an unquoted array value.
    /// Accept whitespace (except new lines) (-> AfterArrayValue),
    /// array value separators (-> BeforeArrayValue),
    /// array end delimiters (-> SkipLineWhitespaceOrComments),
    /// escape sequences (if supported) (-> ArrayValue),
    /// valid value chars (-> ArrayValue).
    /// Contains the current array type, if any.
    ArrayValue(Option<IniValueType>),
    /// We started parsing a quoted string array value.
    /// Accept matching string quotes (`'"'` / `'\'`') (-> AfterArrayValue),
    /// spaces (`' '`),
    /// non-matching string quotes (if supported),
    /// escape sequences (if supported),
    /// valid value chars.
    /// Contains the opening quote.
    QuotedArrayValue(StringQuote),
    /// We finished parsing a previous array value
    /// and expect the array value separator or the end of the array.
    /// Accept whitespace (except new lines),
    /// array value separators (-> BeforeArrayValue),
    /// array end delimiters (-> SkipLineWhitespaceOrComments).
    /// Contains the current array type.
    AfterArrayValue(IniValueType),
}

impl IniParserFSMState {
    /// Processes the next char `c`.
    /// May call `next` to request up to 4 new chars if ASCII / Unicode hex escape sequences are supported.
    /// Returns the new parser state or an error.
    /// The error tuple contains a boolean, which, if `true`, indicates the error location column must be offset
    /// one character back from the current location in the source.
    pub(super) fn process<'s, C, N, S>(
        self,
        c: char,
        idx: usize,
        next: N,
        substr: S,
        config: &mut C,
        state: &mut IniParserPersistentState<'s>,
        options: &IniOptions,
    ) -> Result<IniParserFSMState, (IniErrorKind, bool)>
    where
        C: IniConfig<'s>,
        N: NextChar,
        S: Substr<'s>,
    {
        use IniErrorKind::*;

        Ok(match self {
            IniParserFSMState::StartLine => {
                debug_assert!(state.key.is_empty());
                debug_assert!(state.value.is_empty());

                state.is_key_unique = true;
                state.skip_value = false;

                // Skip whitespace at the start of the line (including new lines).
                if c.is_whitespace() {
                    self

                // Section start delimiter - parse the section name.
                } else if options.is_section_start(c) {
                    // Return an error if we don't support sections.
                    if options.nested_section_depth == 0 {
                        return Err((NestedSectionDepthExceeded, false));
                    }

                    // Clear the current path.
                    state.clear_path(config);

                    state.skip_section = false;

                    IniParserFSMState::BeforeSection

                // Line comment (if supported) - skip the rest of the line.
                } else if options.is_comment_char(c) {
                    IniParserFSMState::SkipLine

                // String quote (if supported) - parse the key in quotes, expecting the matching quotes.
                } else if let Some(quote) = options.is_string_quote_char(c) {
                    IniParserFSMState::QuotedKey(quote)

                // Escaped char (if supported) - parse the escape sequence as the key.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - start parsing the (now owned) key.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.key.push_owned(c, substr);
                            IniParserFSMState::Key
                        }
                        // Line continuation at the start of the line - error.
                        ParseEscapeSequenceResult::LineContinuation => {
                            return Err((UnexpectedNewLineInKey, false))
                        }
                    }

                // Valid key start - start parsing the key.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.key.push(c, idx);
                    IniParserFSMState::Key

                // Key-value separator - empty keys are not allowed.
                } else if options.is_key_value_separator_char(c) {
                    return Err((EmptyKey, true));

                // Else an error.
                } else {
                    return Err((InvalidCharacterAtLineStart(c), false));
                }
            }
            IniParserFSMState::BeforeSection => {
                debug_assert!(state.key.is_empty());
                debug_assert!(state.value.is_empty());
                debug_assert!(options.nested_sections() || state.path.is_empty());

                // Skip whitespace.
                if c.is_whitespace() {
                    // Unless it's a new line.
                    if options.is_new_line(c) {
                        return Err((UnexpectedNewLineInSectionName, true));
                    } else {
                        self
                    }

                // String quote - parse the section name in quotes, expecting the matching quotes.
                } else if let Some(quote) = options.is_string_quote_char(c) {
                    IniParserFSMState::QuotedSection(quote)

                // Nested section separator (if supported) - empty parent section names are not allowed.
                } else if options.is_nested_section_separator(c) {
                    return Err((EmptySectionName, false));

                // Escaped char (if supported).
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - start parsing the (now owned) section name.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.key.push_owned(c, substr);
                            IniParserFSMState::Section
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => {
                            state.key.to_owned(substr);
                            self
                        }
                    }

                // Valid section name char (same rules as keys) - start parsing the section name.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.key.push(c, idx);
                    IniParserFSMState::Section

                // Section end delimiter - empty section names are not allowed.
                } else if options.is_section_end(c) {
                    return Err((EmptySectionName, false));

                // Else an error.
                } else {
                    return Err((InvalidCharacterInSectionName(c), false));
                }
            }
            IniParserFSMState::Section => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(state.value.is_empty());

                // New line before the section delimiter - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInSectionName, true));

                // Nested section separator (if supported) - finish the current section, keep parsing the nested section.
                } else if options.is_nested_section_separator(c) {
                    // Must succeed.
                    let section = unwrap_unchecked(state.key.key(&substr), "empty section name");

                    state.path.push(section);

                    // Make sure we've not exceeded the nested section depth limit.
                    if state.path.len() >= options.nested_section_depth {
                        return Err((NestedSectionDepthExceeded, false));
                    }

                    // The parent section must already exist in the config, unless we allow implicit parent sections,
                    // in which case we start a new empty sections.
                    match config.contains_key(section) {
                        // Parent section already exists.
                        Ok(true) => {}
                        // Parent section doesn't exist, but we allow it.
                        Err(_) if options.implicit_parent_sections => {}
                        // Parent section doesn't exist and we don't allow it, or it's not a section.
                        Ok(false) | Err(_) => {
                            return Err((InvalidParentSection, true));
                        }
                    }

                    // Start the parent section in the config.
                    config.start_section(section, false);

                    state.key.clear();

                    IniParserFSMState::BeforeSection

                // Escaped char (if supported) - keep parsing the section name.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, true, options)? {
                        // Parsed an escaped char - keep parsing the (now owned) section name.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.key.push_owned(c, substr);
                        }
                        // Line continuation - keep parsing (now owned) section name.
                        ParseEscapeSequenceResult::LineContinuation => {
                            state.key.to_owned(substr);
                        }
                    }

                    self

                // Valid section name char - keep parsing the section name.
                } else if options.is_key_or_value_char(c, true, None) {
                    state.key.push(c, idx);
                    self

                // Section end delimiter - finish the section name, skip the rest of the line.
                } else if options.is_section_end(c) {
                    debug_assert!(state.path.len() <= options.nested_section_depth);

                    // Must succeed.
                    let section = unwrap_unchecked(state.key.key(&substr), "empty section name");

                    // Try to add the section to the config at the current path.
                    state.path.push(section);
                    state.skip_section = start_section(config, section, options)?;
                    state.key.clear();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // Whitespace after section name (new lines handled above) - skip it,
                // parse the nested section separator or the section end delimiter.
                } else if c.is_whitespace() {
                    IniParserFSMState::AfterSection

                // Else an error.
                } else {
                    return Err((InvalidCharacterInSectionName(c), false));
                }
            }
            IniParserFSMState::QuotedSection(quote) => {
                debug_assert!(state.value.is_empty());

                // New line before the closing quotes - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInSectionName, true));

                // Closing quotes - keep parsing until the nested section separator or section end delimiter.
                } else if options.is_matching_string_quote_char(quote, c) {
                    IniParserFSMState::AfterSection

                // Escaped char (if supported) - keep parsing the section name.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - keep parsing the (now owned) section name.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.key.push_owned(c, substr);
                        }
                        // Line continuation - keep parsing the (now owned) section name.
                        ParseEscapeSequenceResult::LineContinuation => {
                            state.key.to_owned(substr);
                        }
                    }

                    self

                // Non-matching quotes - keep parsing the section.
                } else if options.is_non_matching_string_quote_char(quote, c) {
                    state.key.push(c, idx);
                    self

                // Space or valid value char - keep parsing the section.
                } else if c == ' ' || options.is_key_or_value_char(c, true, Some(quote)) {
                    state.key.push(c, idx);
                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInSectionName(c), false));
                }
            }
            IniParserFSMState::AfterSection => {
                debug_assert!(state.value.is_empty());

                // Skip whitespace.
                if c.is_whitespace() {
                    // Unless it's a new line.
                    if options.is_new_line(c) {
                        return Err((UnexpectedNewLineInSectionName, true));
                    } else {
                        self
                    }

                // Section end delimiter - skip the rest of the line.
                } else if options.is_section_end(c) {
                    // Empty section names are not allowed.
                    let section = state
                        .key
                        .key(&substr)
                        .ok_or_else(|| (EmptySectionName, true))?;

                    // Try to add the section to the config at the current path.
                    state.path.push(section);
                    state.skip_section = start_section(config, section, options)?;
                    state.key.clear();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // Nested section separator (if supported) - start parsing the nested section name.
                } else if options.is_nested_section_separator(c) {
                    // Empty section names are not allowed.
                    let section = state
                        .key
                        .key(&substr)
                        .ok_or_else(|| (EmptySectionName, true))?;

                    state.path.push(section);

                    if state.path.len() >= options.nested_section_depth {
                        return Err((NestedSectionDepthExceeded, false));
                    }

                    // The parent section must already exist in the config, unless we allow implicit parent sections,
                    // in which case we start a new empty sections.
                    match config.contains_key(section) {
                        // Parent section already exists.
                        Ok(true) => {}
                        // Parent section doesn't exist, but we allow it.
                        Err(_) if options.implicit_parent_sections => {}
                        // Parent section doesn't exist and we don't allow it, or it's not a section.
                        Ok(false) | Err(_) => {
                            return Err((InvalidParentSection, true));
                        }
                    }

                    // Start the parent section in the config.
                    config.start_section(section, false);

                    state.key.clear();

                    IniParserFSMState::BeforeSection

                // Else an error.
                } else {
                    // Empty section names are not allowed.
                    let section = state
                        .key
                        .key(&substr)
                        .ok_or_else(|| (EmptySectionName, true))?;

                    state.path.push(section);
                    return Err((InvalidCharacterAfterSectionName(c), false));
                }
            }
            IniParserFSMState::SkipLine => {
                debug_assert!(state.key.is_empty());
                debug_assert!(state.value.is_empty());

                // If it's a new line, start parsing the next line.
                // Skip everything else.
                if options.is_new_line(c) {
                    IniParserFSMState::StartLine
                } else {
                    self
                }
            }
            IniParserFSMState::SkipLineWhitespaceOrComments => {
                debug_assert!(state.key.is_empty());
                debug_assert!(state.value.is_empty());

                // If it's a new line, start parsing the next line.
                if options.is_new_line(c) {
                    IniParserFSMState::StartLine

                // Skip other whitespace.
                } else if c.is_whitespace() {
                    self

                // Inline comment (if supported) - skip the rest of the line.
                } else if options.is_inline_comment_char(c) {
                    IniParserFSMState::SkipLine

                // Else an error.
                } else {
                    return Err((InvalidCharacterAtLineEnd(c), false));
                }
            }
            IniParserFSMState::Key => {
                // We have at least one key character already parsed.
                debug_assert!(!state.key.is_empty());
                debug_assert!(state.value.is_empty());

                // Key-value separator - finish the key, parse the value.
                if options.is_key_value_separator_char(c) {
                    // Must succeed.
                    let key = unwrap_unchecked(state.key.key(&substr), "empty key");
                    state.path.push(key);

                    check_is_key_duplicate(
                        config,
                        key,
                        state.skip_section,
                        &mut state.skip_value,
                        &mut state.is_key_unique,
                        options.duplicate_keys,
                    )?;

                    IniParserFSMState::BeforeValue

                // Whitespace between the key and the separator - skip it, finish the key, parse the separator.
                } else if c.is_whitespace() {
                    // Unless it's a new line.
                    if options.is_new_line(c) {
                        return Err((UnexpectedNewLineInKey, true));
                    }

                    // Must succeed.
                    let key = unwrap_unchecked(state.key.key(&substr), "empty key");
                    state.path.push(key);

                    check_is_key_duplicate(
                        config,
                        key,
                        state.skip_section,
                        &mut state.skip_value,
                        &mut state.is_key_unique,
                        options.duplicate_keys,
                    )?;

                    IniParserFSMState::KeyValueSeparator

                // Escaped char (if supported) - keep parsing the key.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - keep parsing the (now owned) key.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.key.push_owned(c, substr)
                        }
                        // Line continuation - keep parsing the (now owned) key.
                        ParseEscapeSequenceResult::LineContinuation => {
                            state.key.to_owned(substr);
                        }
                    }

                    self

                // Valid key char - keep parsing the key.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.key.push(c, idx);
                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInKey(c), false));
                }
            }
            IniParserFSMState::QuotedKey(quote) => {
                debug_assert!(state.value.is_empty());

                // New line before the closing quotes - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInKey, true));

                // Closing quotes - finish the key, parse the separator.
                } else if options.is_matching_string_quote_char(quote, c) {
                    // Empty keys are not allowed.
                    let key = state.key.key(&substr).ok_or_else(|| (EmptyKey, false))?;
                    state.path.push(key);

                    check_is_key_duplicate(
                        config,
                        key,
                        state.skip_section,
                        &mut state.skip_value,
                        &mut state.is_key_unique,
                        options.duplicate_keys,
                    )?;

                    IniParserFSMState::KeyValueSeparator

                // Escaped char (if supported) - keep parsing the key.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - keep parsing the (now owned) key.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.key.push_owned(c, substr);
                        }
                        // Line continuation - keep parsing the (now owned) key.
                        ParseEscapeSequenceResult::LineContinuation => {
                            state.key.to_owned(substr);
                        }
                    }

                    self

                // Non-matching quotes - keep parsing the key.
                } else if options.is_non_matching_string_quote_char(quote, c) {
                    state.key.push(c, idx);
                    self

                // Space or valid key char - keep parsing the key.
                } else if c == ' ' || options.is_key_or_value_char(c, false, Some(quote)) {
                    state.key.push(c, idx);
                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInKey(c), false));
                }
            }
            IniParserFSMState::KeyValueSeparator => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(state.value.is_empty());
                debug_assert!(!state.path.is_empty());

                // Key-value separator - parse the value (key already finished).
                if options.is_key_value_separator_char(c) {
                    IniParserFSMState::BeforeValue

                // Skip the whitespace between the key and the separator.
                } else if c.is_whitespace() {
                    // Unless it's a new line.
                    if options.is_new_line(c) {
                        return Err((UnexpectedNewLineInKey, true));
                    } else {
                        self
                    }

                // Else an error.
                } else {
                    return Err((InvalidKeyValueSeparator(c), false));
                }
            }
            IniParserFSMState::BeforeValue => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(state.value.is_empty());
                debug_assert!(!state.path.is_empty());

                // Skip the whitespace before the value.
                if c.is_whitespace() {
                    // Unless it's a new line - the value is empty.
                    if options.is_new_line(c) {
                        add_value_to_config(
                            config,
                            // Must succeed.
                            unwrap_unchecked(state.key.key(&substr), "empty key"),
                            IniStr::Empty,
                            false,
                            state.skip_section | state.skip_value,
                            state.is_key_unique,
                            options.unquoted_strings,
                        )
                        .map_err(|error_kind| (error_kind, false))?;

                        state.key.clear();
                        state.path.pop();

                        IniParserFSMState::StartLine
                    } else {
                        self
                    }

                // Inline comment (if supported) - the value is empty, skip the rest of the line.
                } else if options.is_inline_comment_char(c) {
                    add_value_to_config(
                        config,
                        // Must succeed.
                        unwrap_unchecked(state.key.key(&substr), "empty key"),
                        IniStr::Empty,
                        false,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                        options.unquoted_strings,
                    )
                    .map_err(|error_kind| (error_kind, false))?;

                    state.key.clear();
                    state.path.pop();

                    IniParserFSMState::SkipLine

                // String quote - parse the string value in quotes, expecting the matching quotes.
                } else if let Some(quote) = options.is_string_quote_char(c) {
                    IniParserFSMState::QuotedValue(quote)

                // Escaped char (if supported) - parse the escape sequence, start parsing the value.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - start parsing the (now owned) value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.value.push_owned(c, substr);
                            IniParserFSMState::Value
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => self,
                    }

                // Array start delimiter (if supported) - start parsing the array.
                } else if options.is_array_start(c) {
                    // Must succeed.
                    let array_key = unwrap_unchecked(state.key.key(&substr), "empty key");

                    add_array_to_config(
                        config,
                        array_key,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                    );

                    //state.path.push(array_key);

                    IniParserFSMState::BeforeArrayValue(None)

                // Valid value char - start parsing the unquoted value.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.value.push(c, idx);
                    IniParserFSMState::Value

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::Value => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(!state.value.is_empty());
                debug_assert!(!state.path.is_empty());

                // Whitespace - finish the value.
                if c.is_whitespace() {
                    add_value_to_config(
                        config,
                        // Must succeed.
                        unwrap_unchecked(state.key.key(&substr), "empty key"),
                        state.value.value(&substr),
                        false,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                        options.unquoted_strings,
                    )
                    .map_err(|error_kind| (error_kind, false))?;

                    state.key.clear();
                    state.value.clear();
                    state.path.pop();

                    // New line - start a new line.
                    if options.is_new_line(c) {
                        IniParserFSMState::StartLine

                    // Not a new line - skip the rest of the line.
                    } else {
                        IniParserFSMState::SkipLineWhitespaceOrComments
                    }

                // Inline comment (if supported) - finish the value, skip the rest of the line.
                } else if options.is_inline_comment_char(c) {
                    add_value_to_config(
                        config,
                        // Must succeed.
                        unwrap_unchecked(state.key.key(&substr), "empty key"),
                        state.value.value(&substr),
                        false,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                        options.unquoted_strings,
                    )
                    .map_err(|error_kind| (error_kind, false))?;

                    state.key.clear();
                    state.value.clear();
                    state.path.pop();

                    IniParserFSMState::SkipLine

                // Escaped char (if supported) - parse the escape sequence.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - keep parsing the (now owned) value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.value.push_owned(c, substr);
                        }
                        // Line continuation - keep parsing the (now owned) value.
                        ParseEscapeSequenceResult::LineContinuation => {
                            state.value.to_owned(substr);
                        }
                    }

                    self

                // Valid value char - keep parsing the value.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.value.push(c, idx);
                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::QuotedValue(quote) => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(!state.path.is_empty());

                // New line before the closing quotes - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInQuotedValue, true));

                // Closing quotes - finish the quoted value (which may be empty), skip the rest of the line.
                } else if options.is_matching_string_quote_char(quote, c) {
                    add_value_to_config(
                        config,
                        // Must succeed.
                        unwrap_unchecked(state.key.key(&substr), "empty key"),
                        state.value.value(&substr),
                        true,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                        options.unquoted_strings,
                    )
                    .map_err(|error_kind| (error_kind, false))?;

                    state.value.clear();
                    state.key.clear();
                    state.path.pop();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // Escaped char (if supported) - parse the escape sequence.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - keep parsing the (now owned) quoted value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.value.push_owned(c, substr);
                        }
                        // Line continuation - keep parsing the (now owned) quoted value.
                        ParseEscapeSequenceResult::LineContinuation => {
                            state.value.to_owned(substr);
                        }
                    }

                    self

                // Non-matching quotes - keep parsing the quoted value.
                } else if options.is_non_matching_string_quote_char(quote, c) {
                    state.value.push(c, idx);
                    self

                // Space or valid value char - keep parsing the quoted value.
                } else if c == ' ' || options.is_key_or_value_char(c, false, Some(quote)) {
                    state.value.push(c, idx);
                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::BeforeArrayValue(array_type) => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(state.value.is_empty());
                // We have at least the array key in the path.
                debug_assert!(!state.path.is_empty());

                // Skip the whitespace before the array value.
                if c.is_whitespace() {
                    // Unless it's a new line.
                    if options.is_new_line(c) {
                        return Err((UnexpectedNewLineInArray, true));
                    } else {
                        self
                    }

                // Array end delimiter - finish the array, skip the rest of the line.
                } else if options.is_array_end(c) {
                    // Must succeed.
                    let array_key = unwrap_unchecked(state.key.key(&substr), "empty array key");

                    config.end_array(array_key);

                    // Pop the array key off the path.
                    state.path.pop();
                    state.key.clear();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // String quote - parse the string array value in quotes, expecting the matching quotes.
                } else if let Some(quote) = options.is_string_quote_char(c) {
                    // Make sure the array is empty or contains strings (is not mixed).
                    if let Some(array_type) = array_type {
                        if !array_type.is_compatible(IniValueType::String) {
                            return Err((IniErrorKind::MixedArray, false));
                        }
                    }

                    IniParserFSMState::QuotedArrayValue(quote)

                // Escaped char (if supported) - parse the escape sequence, start parsing the array value.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - start parsing the (owned) unquoted array value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.value.push_owned(c, substr);
                            IniParserFSMState::ArrayValue(array_type)
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => self,
                    }

                // Valid value char - start parsing the unquoted array value.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.value.push(c, idx);
                    IniParserFSMState::ArrayValue(array_type)

                // Else an error.
                } else {
                    return Err((InvalidCharacterInArray(c), false));
                }
            }
            IniParserFSMState::ArrayValue(mut array_type) => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(!state.value.is_empty());
                // We have at least the array key in the path.
                debug_assert!(!state.path.is_empty());

                // Whitespace - finish the current array value,
                // parse the array value separator / array end delimiter.
                if c.is_whitespace() {
                    // Unless it's a new line.
                    if options.is_new_line(c) {
                        return Err((IniErrorKind::UnexpectedNewLineInArray, true));
                    }

                    add_value_to_array(
                        config,
                        state.value.value(&substr),
                        false,
                        state.skip_value | state.skip_section,
                        &mut array_type,
                        options.unquoted_strings,
                    )?;

                    state.value.clear();

                    IniParserFSMState::AfterArrayValue(unwrap_unchecked(
                        array_type,
                        "array type must be known at this point",
                    ))

                // Array value separator - finish the current array value,
                // parse the next array value / array end delimiter.
                } else if options.is_array_value_separator(c) {
                    add_value_to_array(
                        config,
                        state.value.value(&substr),
                        false,
                        state.skip_value | state.skip_section,
                        &mut array_type,
                        options.unquoted_strings,
                    )?;

                    state.value.clear();

                    IniParserFSMState::BeforeArrayValue(array_type)

                // Array end delimiter - add the value to the array, finish the array, skip the rest of the line.
                } else if options.is_array_end(c) {
                    add_value_to_array(
                        config,
                        state.value.value(&substr),
                        false,
                        state.skip_value | state.skip_section,
                        &mut array_type,
                        options.unquoted_strings,
                    )?;

                    state.value.clear();

                    // Must succeed.
                    let array_key = unwrap_unchecked(state.key.key(&substr), "empty array key");

                    config.end_array(array_key);

                    // Pop the array key off the path.
                    state.path.pop();
                    state.key.clear();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // Escaped char (if supported) - parse the escape sequence.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - keep parsing the (now owned) unquoted array value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.value.push_owned(c, substr);
                        }
                        // Line continuation - keep parsing the (now owned) unquoted array value.
                        ParseEscapeSequenceResult::LineContinuation => {
                            state.value.to_owned(substr);
                        }
                    }

                    self

                // Valid value char - keep parsing the unquoted array value.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.value.push(c, idx);
                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::QuotedArrayValue(quote) => {
                debug_assert!(!state.key.is_empty());
                // We have at least the array key in the path.
                debug_assert!(!state.path.is_empty());

                // New line before the closing quotes - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInQuotedValue, true));

                // Closing quotes - finish the array value (may be empty),
                // parse the array value separator / array end delimiter.
                } else if options.is_matching_string_quote_char(quote, c) {
                    let mut dummy_array_type = None;
                    add_value_to_array(
                        config,
                        state.value.value(&substr),
                        true,
                        state.skip_value | state.skip_section,
                        &mut dummy_array_type,
                        options.unquoted_strings,
                    )?;
                    debug_assert_eq!(dummy_array_type, Some(IniValueType::String));

                    state.value.clear();

                    IniParserFSMState::AfterArrayValue(IniValueType::String)

                // Escaped char (if supported) - parse the escape sequence.
                } else if options.is_escape_char(c) {
                    match try_parse_escape_sequence(next, false, options)? {
                        // Parsed an escaped char - keep parsing the (now owned) quoted array value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.value.push_owned(c, substr);
                        }
                        // Line continuation - keep parsing the (now owned) quoted array value.
                        ParseEscapeSequenceResult::LineContinuation => {
                            state.value.to_owned(substr);
                        }
                    }

                    self

                // Non-matching quotes - keep parsing the quoted array value.
                } else if options.is_non_matching_string_quote_char(quote, c) {
                    state.value.push(c, idx);
                    self

                // Space or valid value char - keep parsing the quoted array value.
                } else if c == ' ' || options.is_key_or_value_char(c, false, Some(quote)) {
                    state.value.push(c, idx);
                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::AfterArrayValue(array_type) => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(state.value.is_empty());
                // We have at least the array key in the path.
                debug_assert!(!state.path.is_empty());

                // Skip whitespace.
                if c.is_whitespace() {
                    // Unless it's a new line.
                    if options.is_new_line(c) {
                        return Err((UnexpectedNewLineInArray, true));
                    } else {
                        self
                    }

                // Array value separator - parse the next array value / array end delimiter.
                } else if options.is_array_value_separator(c) {
                    IniParserFSMState::BeforeArrayValue(Some(array_type))

                // Array end delimiter - finish the array, skip the rest of the line.
                } else if options.is_array_end(c) {
                    // Must succeed.
                    let array_key = unwrap_unchecked(state.key.key(&substr), "empty array key");

                    config.end_array(array_key);

                    // Pop the array key off the path.
                    state.path.pop();
                    state.key.clear();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // Else an error.
                } else {
                    return Err((InvalidCharacterInArray(c), false));
                }
            }
        })
    }

    /// Called after EOF for cleanup and error reporting if the EOF was unexpected for the current parser state.
    pub(super) fn finish<'s, C, S>(
        self,
        substr: S,
        config: &mut C,
        state: &mut IniParserPersistentState,
        options: &IniOptions,
    ) -> Result<(), IniErrorKind>
    where
        C: IniConfig<'s>,
        S: Substr<'s>,
    {
        use {IniErrorKind::*, IniParserFSMState::*};

        match self {
            BeforeSection | Section | QuotedSection(_) | AfterSection => {
                return Err(UnexpectedEndOfFileInSectionName)
            }
            Key | QuotedKey(_) | KeyValueSeparator => {
                return Err(UnexpectedEndOfFileBeforeKeyValueSeparator)
            }
            QuotedValue(_) => return Err(UnexpectedEndOfFileInQuotedString),
            // Add the last value if we were parsing it right before EOF.
            Value | BeforeValue => {
                // We have at least one key character already parsed.
                debug_assert!(!state.key.is_empty());
                debug_assert!(!state.path.is_empty());

                add_value_to_config(
                    config,
                    // Must succeed.
                    unwrap_unchecked(state.key.key(&substr), "empty key"),
                    state.value.value(&substr),
                    false,
                    state.skip_section | state.skip_value,
                    state.is_key_unique,
                    options.unquoted_strings,
                )?;

                state.path.pop();

                Ok(())
            }
            BeforeArrayValue(_) | ArrayValue(_) | AfterArrayValue(_) => {
                return Err(UnexpectedEndOfFileInArray)
            }
            QuotedArrayValue(_) => return Err(UnexpectedEndOfFileInQuotedArrayValue),
            StartLine | SkipLine | SkipLineWhitespaceOrComments => Ok(()),
        }
    }
}

fn try_char_to_hex_digit(c: char) -> Option<u8> {
    Some(match c {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'a' | 'A' => 10,
        'b' | 'B' => 11,
        'c' | 'C' => 12,
        'd' | 'D' => 13,
        'e' | 'E' => 14,
        'f' | 'F' => 15,
        _ => return None,
    })
}

fn hex_digits_to_number(digits: &[u8]) -> u32 {
    const RADIX: u32 = 16;

    let mut result = 0;

    for d in digits {
        debug_assert!(*d < RADIX as _);

        result *= RADIX;
        result += *d as u32;
    }

    result
}

/// Reads up to 4 following characters and tries to parse them as an escape sequence.
/// `in_unquoted_section` is `true` if we are parsing an unquoted `.ini` section name.
fn try_parse_escape_sequence<F: FnMut() -> Option<char>>(
    mut next: F,
    in_unquoted_section: bool,
    options: &IniOptions,
) -> Result<ParseEscapeSequenceResult, (IniErrorKind, bool)> {
    use IniErrorKind::*;
    use ParseEscapeSequenceResult::*;

    debug_assert!(options.escape);

    const MAX_NUM_UNICODE_ESCAPE_HEX_DIGITS: usize = 6;
    type HexDigits = [u8; MAX_NUM_UNICODE_ESCAPE_HEX_DIGITS];
    let mut hex_digits: HexDigits = [0, 0, 0, 0, 0, 0];
    let mut hex_digits_len: usize = 0;

    // Return `Ok(false)` to stop parsing when the closing bracket (`}`) reached.
    let mut parse_unicode_escape_hex_digit =
        |c: Option<char>, bracketed: bool| -> Result<bool, (IniErrorKind, bool)> {
            match c {
                None => return Err((UnexpectedEndOfFileInUnicodeEscapeSequence, false)),
                Some('\n') | Some('\r') => {
                    return Err((UnexpectedNewLineInUnicodeEscapeSequence, true))
                }
                Some(c) => {
                    if bracketed && c == '}' {
                        return Ok(false);
                    } else if let Some(digit) = try_char_to_hex_digit(c) {
                        debug_assert!(hex_digits_len < MAX_NUM_UNICODE_ESCAPE_HEX_DIGITS);
                        *(unsafe { hex_digits.get_unchecked_mut(hex_digits_len) }) = digit;
                        hex_digits_len += 1;
                    } else {
                        return Err((InvalidCharacterInUnicodeEscapeSequence(c), false));
                    }
                }
            }

            Ok(true)
        };

    match next() {
        None => Err((UnexpectedEndOfFileInEscapeSequence, false)),

        // Backslash followed by a new line is a line continuation, if supported.
        Some('\n') | Some('\r') => {
            if options.line_continuation {
                Ok(LineContinuation)
            } else {
                Err((UnexpectedNewLineInEscapeSequence, true))
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
        Some('/') if (options.nested_sections() && in_unquoted_section) => Ok(EscapedChar('/')),

        // Exactly 2 hexadecimal digits corresponding to a Unicode scalar value up to 0xff.
        Some('x') => {
            for _ in 0..2 {
                parse_unicode_escape_hex_digit(next(), false)?;
            }

            debug_assert!(hex_digits_len < MAX_NUM_UNICODE_ESCAPE_HEX_DIGITS);
            Ok(EscapedChar(
                std::char::from_u32(hex_digits_to_number(unsafe {
                    hex_digits.get_unchecked(0..hex_digits_len)
                }))
                .ok_or_else(|| (InvalidUnicodeEscapeSequence, false))?,
            ))
        }

        // Exactly 2 hexadecimal digits corresponding to a Unicode scalar value up to 0xffff (i.e. within the BMP),
        // OR 1 to 6 (inclusive) hexadecimal digits in brackets (`{` / `}`) corresponding to a Unicode scalar value up to 0x10ffff
        // (i.e. may represent any valid Unicode scalar value).
        Some('u') => {
            match next() {
                None => return Err((UnexpectedEndOfFileInUnicodeEscapeSequence, false)),
                Some('\n') | Some('\r') => {
                    return Err((UnexpectedNewLineInUnicodeEscapeSequence, true))
                }
                // Start parsing the bracketed Unicode escape sequence.
                Some('{') => {
                    for _ in 0..MAX_NUM_UNICODE_ESCAPE_HEX_DIGITS {
                        if !parse_unicode_escape_hex_digit(next(), true)? {
                            break;
                        }
                    }
                }
                Some(c) => {
                    // Start parsing a normal 4-digit Unicode escape sequence (now only 3 digits remain).
                    parse_unicode_escape_hex_digit(Some(c), false)?;
                    for _ in 0..3 {
                        parse_unicode_escape_hex_digit(next(), false)?;
                    }
                }
            }

            if hex_digits_len == 0 {
                Err((InvalidUnicodeEscapeSequence, false))
            } else {
                debug_assert!(hex_digits_len < MAX_NUM_UNICODE_ESCAPE_HEX_DIGITS);
                Ok(EscapedChar(
                    std::char::from_u32(hex_digits_to_number(unsafe {
                        hex_digits.get_unchecked(0..hex_digits_len)
                    }))
                    .ok_or_else(|| (InvalidUnicodeEscapeSequence, false))?,
                ))
            }
        }

        Some(c) => Err((InvalidEscapeCharacter(c), false)),
    }
}

/// Returns `Ok(true)` if we need to skip the current section;
/// else returns `Ok(false)`.
fn start_section<'s, C: IniConfig<'s>>(
    config: &mut C,
    section: NonEmptyIniStr<'s, '_>,
    options: &IniOptions,
) -> Result<bool, (IniErrorKind, bool)> {
    let key_already_exists = config.contains_key(section);

    // Section already exists.
    if let Ok(true) = key_already_exists {
        match options.duplicate_sections {
            // We don't support duplicate sections - error.
            IniDuplicateSections::Forbid => {
                return Err((IniErrorKind::DuplicateSection, false));
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
            match options.duplicate_keys {
                // We don't support duplicate keys - error.
                IniDuplicateKeys::Forbid => {
                    return Err((IniErrorKind::DuplicateKey, true));
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

/// Sets `skip_value` to `true` if we need to skip the current value;
/// sets `is_key_unique` to `true` if the key is not contained in `config`'s current section.
fn check_is_key_duplicate<'s, C: IniConfig<'s>>(
    config: &C,
    key: NonEmptyIniStr<'s, '_>,
    skip_section: bool,
    skip_value: &mut bool,
    is_key_unique: &mut bool,
    duplicate_keys: IniDuplicateKeys,
) -> Result<(), (IniErrorKind, bool)> {
    use IniErrorKind::*;

    if skip_section {
        *skip_value = true;
        *is_key_unique = false;

        return Ok(());
    }

    let is_unique = config.contains_key(key).is_err();

    match duplicate_keys {
        IniDuplicateKeys::Forbid => {
            if is_unique {
                *skip_value = false;
                *is_key_unique = true;

                Ok(())
            } else {
                Err((DuplicateKey, true))
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

/// Parses a string `value` and adds it to the `config`'s current section at `key`.
/// If `quoted` is `true`, `value` is always treated as a string,
/// else it is first interpreted as a bool / integer / float.
/// Empty `value`'s are treated as strings.
fn add_value_to_config<'s, C: IniConfig<'s>>(
    config: &mut C,
    key: NonEmptyIniStr<'s, '_>,
    value: IniStr<'s, '_>,
    quoted: bool,
    skip: bool,
    is_key_unique: bool,
    unquoted_strings: bool,
) -> Result<(), IniErrorKind> {
    if !skip {
        config.add_value(
            key,
            parse_value_string(value, quoted, unquoted_strings)?,
            !is_key_unique,
        );
    }

    Ok(())
}

/// Adds an empty array to the `config`'s current section at `key`.
fn add_array_to_config<'s, C: IniConfig<'s>>(
    config: &mut C,
    key: NonEmptyIniStr<'s, '_>,
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
/// Updates the `array_type`.
fn add_value_to_array<'s, C: IniConfig<'s>>(
    config: &mut C,
    value: IniStr<'s, '_>,
    quoted: bool,
    skip: bool,
    array_type: &mut Option<IniValueType>,
    unquoted_strings: bool,
) -> Result<(), (IniErrorKind, bool)> {
    if skip {
        return Ok(());
    }

    let value = parse_value_string(value, quoted, unquoted_strings)
        .map_err(|error_kind| (error_kind, false))?;
    let value_type = value.get_ini_type();

    // Make sure the array is not mixed.
    if let Some(array_type) = array_type {
        if !array_type.is_compatible(value_type) {
            return Err((IniErrorKind::MixedArray, true));
        }
    } else {
        array_type.replace(value_type);
    }

    config.add_array_value(value);

    Ok(())
}

/// Parses a string `value`.
/// If `quoted` is `true`, `value` is always treated as a string,
/// else it is first interpreted as a bool / integer / float.
/// Empty `value`'s are treated as strings.
fn parse_value_string<'s, 'a>(
    value: IniStr<'s, 'a>,
    quoted: bool,
    unquoted_strings: bool,
) -> Result<IniValue<'s, 'a>, IniErrorKind> {
    use IniErrorKind::*;
    use IniValue::*;

    // Empty and quoted values are treated as strings.
    let value = if value.as_str().is_empty() || quoted {
        String(value)

    // Check if it's a bool.
    } else if value.as_str() == "true" {
        Bool(true)
    } else if value.as_str() == "false" {
        Bool(false)

    // Check if it's an integer.
    } else if let Some(value) = try_parse_integer(value.as_str()) {
        I64(value)

    // Else check if it's a float.
    } else if let Ok(value) = value.as_str().parse::<f64>() {
        F64(value)

    // Else we assume it's an unquoted string.
    } else {
        // Unless we don't allow unquoted strings.
        if !unquoted_strings {
            return Err(UnquotedString);
        }

        String(value)
    };

    Ok(value)
}

fn try_parse_integer(value: &str) -> Option<i64> {
    if value.is_empty() {
        None
    } else {
        // Explicit sign.
        let (sign, value) = {
            if let Some(value) = value.strip_prefix("+") {
                (1, value)
            } else if let Some(value) = value.strip_prefix("-") {
                (-1, value)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_integer_test() {
        assert_eq!(try_parse_integer("7").unwrap(), 7);
        assert_eq!(try_parse_integer("+7").unwrap(), 7);
        assert_eq!(try_parse_integer("-7").unwrap(), -7);

        assert_eq!(try_parse_integer("0x17").unwrap(), 23);
        assert_eq!(try_parse_integer("+0x17").unwrap(), 23);
        assert_eq!(try_parse_integer("-0x17").unwrap(), -23);

        assert_eq!(try_parse_integer("0o17").unwrap(), 15);
        assert_eq!(try_parse_integer("+0o17").unwrap(), 15);
        assert_eq!(try_parse_integer("-0o17").unwrap(), -15);

        assert!(try_parse_integer("-").is_none());
        assert!(try_parse_integer("+").is_none());
        assert!(try_parse_integer("0x").is_none());
        assert!(try_parse_integer("+0x").is_none());
        assert!(try_parse_integer("-0x").is_none());
        assert!(try_parse_integer("0o").is_none());
        assert!(try_parse_integer("+0o").is_none());
        assert!(try_parse_integer("-0o").is_none());

        assert!(try_parse_integer("+7.").is_none());
        assert!(try_parse_integer("-7.").is_none());
        assert!(try_parse_integer("7.").is_none());
        assert!(try_parse_integer(".0").is_none());
        assert!(try_parse_integer("+.0").is_none());
        assert!(try_parse_integer("-.0").is_none());
        assert!(try_parse_integer("7e2").is_none());
        assert!(try_parse_integer("7e+2").is_none());
        assert!(try_parse_integer("7e-2").is_none());
        assert!(try_parse_integer("7.0e2").is_none());
        assert!(try_parse_integer("7.0e+2").is_none());
        assert!(try_parse_integer("7.0e-2").is_none());
    }

    #[test]
    fn try_parse_escape_sequence_test() {
        let parse_hex = |src: &str, res: char| {
            let mut src = std::iter::once('x').chain(src.chars());
            assert_eq!(
                try_parse_escape_sequence(|| src.next(), false, &Default::default()).unwrap(),
                ParseEscapeSequenceResult::EscapedChar(res)
            );
        };

        parse_hex("20", ' ');
        parse_hex("24", '$');
        parse_hex("2c", ',');
        parse_hex("59", 'Y');
        parse_hex("66", 'f');

        parse_hex("b5", 'µ');
        parse_hex("b6", '¶');
        parse_hex("c6", 'Æ');
        parse_hex("e9", 'é');

        let parse_unicode = |src: &str, res: char| {
            let mut src = std::iter::once('u').chain(src.chars());
            assert_eq!(
                try_parse_escape_sequence(|| src.next(), false, &Default::default()).unwrap(),
                ParseEscapeSequenceResult::EscapedChar(res)
            );
        };

        parse_unicode("0020", ' ');
        parse_unicode("0024", '$');
        parse_unicode("002c", ',');
        parse_unicode("0059", 'Y');
        parse_unicode("0066", 'f');

        parse_unicode("00b5", 'µ');
        parse_unicode("00b6", '¶');
        parse_unicode("00c6", 'Æ');
        parse_unicode("00e9", 'é');

        parse_unicode("0117", 'ė');
        parse_unicode("0133", 'ĳ');
        parse_unicode("017D", 'Ž');
        parse_unicode("2030", '‰');

        let parse_bracketed_unicode = |src: &str, res: char| {
            let mut src = std::iter::once('u')
                .chain(std::iter::once('{'))
                .chain(src.chars())
                .chain(std::iter::once('}'));
            assert_eq!(
                try_parse_escape_sequence(|| src.next(), false, &Default::default()).unwrap(),
                ParseEscapeSequenceResult::EscapedChar(res)
            );
        };

        parse_bracketed_unicode("1f639", '😹');
        parse_bracketed_unicode("1f607", '😇');
    }
}
