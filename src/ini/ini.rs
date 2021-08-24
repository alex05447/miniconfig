use {
    super::util::IniPath,
    crate::*,
    std::{iter::Iterator, str::Chars},
};

/// `.ini` parser FSM states.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum IniParserFSMState {
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
    /// Contains the opening quote.
    QuotedSection(char),
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
    /// Contains the opening quote.
    QuotedKey(char),
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
    /// Contains the opening quote.
    QuotedValue(char),
    /// We started parsing an array, or finished parsing a previous array value and separator,
    /// and expect the new value or the end of the array.
    /// Accept whitespace (except new lines),
    /// array end delimiters (-> SkipLineWhitespaceOrComments),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedArrayValue),
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
    /// We started parsing a quoted array value.
    /// Accept matching string quotes ('"' / '\'') (-> AfterArrayValue),
    /// spaces (' '),
    /// non-matching string quotes (if supported),
    /// escape sequences (if supported),
    /// valid value chars.
    /// Contains the opening quote and the current array type, if any.
    QuotedArrayValue(char, Option<IniValueType>),
    /// We finished parsing a previous array value
    /// and expect the array value separator or the end of the array.
    /// Accept whitespace (except new lines),
    /// array value separators (-> BeforeArrayValue),
    /// array end delimiters (-> SkipLineWhitespaceOrComments).
    /// Contains the current array type, if any.
    AfterArrayValue(Option<IniValueType>),
}

impl IniParserFSMState {
    /// Processes the next char `c`.
    /// May call `next` to request up to 4 new chars if ASCII / Unicode hex escape sequences are supported.
    /// Returns the new parser state or an error.
    /// The error tuple contains a boolean, which, if `true`, indicates the error location column must be offset
    /// one character back from the current location in the source.
    fn process<'a, C: IniConfig, F: FnMut() -> Option<char>>(
        self,
        c: char,
        next: F,
        config: &mut C,
        state: &mut IniParserPersistentState,
        options: &IniOptions,
    ) -> Result<IniParserFSMState, (IniErrorKind<'a>, bool)> {
        use IniErrorKind::*;

        Ok(match self {
            IniParserFSMState::StartLine => {
                debug_assert!(state.key.is_empty());
                debug_assert!(state.buffer.is_empty());

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
                    while let Some(section) = state.path.last() {
                        // We didn't call `start_section()` if we skipped it, so don't call `end_section`.
                        if !state.skip_section {
                            config.end_section(section);
                        } else {
                            state.skip_section = false;
                        }
                        state.path.pop();
                    }

                    state.skip_section = false;

                    IniParserFSMState::BeforeSection

                // Line comment (if supported) - skip the rest of the line.
                } else if options.is_comment_char(c) {
                    IniParserFSMState::SkipLine

                // String quote (if supported) - parse the key in quotes, expecting the matching quotes.
                } else if options.is_string_quote_char(c) {
                    IniParserFSMState::QuotedKey(c)

                // Escaped char (if supported) - parse the escape sequence as the key.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - start parsing the key.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);

                            IniParserFSMState::Key
                        }
                        // Line continuation - error.
                        ParseEscapeSequenceResult::LineContinuation => {
                            return Err((UnexpectedNewLineInKey, false))
                        }
                    }

                // Valid key start - parse the key.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.buffer.push(c);

                    IniParserFSMState::Key

                // Key-value separator - it's an empty key.
                } else if options.is_key_value_separator_char(c) {
                    return Err((EmptyKey, true));

                // Else an error.
                } else {
                    return Err((InvalidCharacterAtLineStart(c), false));
                }
            }
            IniParserFSMState::BeforeSection => {
                debug_assert!(state.key.is_empty());
                debug_assert!(state.buffer.is_empty());
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
                } else if options.is_string_quote_char(c) {
                    IniParserFSMState::QuotedSection(c)

                // Nested section separator (if supported) - empty parent section names are not allowed.
                } else if options.is_nested_section_separator(c) {
                    return Err((EmptySectionName(state.path.to_config_path()), false));

                // Escaped char (if supported) - start parsing the section name.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the section name.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);

                            IniParserFSMState::Section
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => self,
                    }

                // Valid section name char (same rules as key chars) - start parsing the section name.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.buffer.push(c);

                    IniParserFSMState::Section

                // Section end delimiter - empty section names not allowed.
                } else if options.is_section_end(c) {
                    return Err((EmptySectionName(state.path.to_config_path()), false));

                // Else an error.
                } else {
                    return Err((InvalidCharacterInSectionName(c), false));
                }
            }
            IniParserFSMState::Section => {
                debug_assert!(state.key.is_empty());
                debug_assert!(!state.buffer.is_empty());

                // New line before the section delimiter - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInSectionName, true));

                // Nested section separator (if supported) - finish the current section, keep parsing the nested section.
                } else if options.is_nested_section_separator(c) {
                    // Empty section names are not allowed.
                    let section = NonEmptyStr::new(&state.buffer)
                        .ok_or((EmptySectionName(state.path.to_config_path()), false))?;

                    if state.path.len() + 1 >= options.nested_section_depth {
                        return Err((NestedSectionDepthExceeded, false));
                    }

                    state.path.push(section);

                    // The path must already exist in the config.
                    if config.contains_key(section) != Ok(true) {
                        return Err((InvalidParentSection(state.path.to_config_path()), true));
                    }

                    // Start the parent section in the config.
                    config.start_section(section, false);

                    state.buffer.clear();

                    IniParserFSMState::BeforeSection

                // Escaped char (if supported) - keep parsing the section name.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        true,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the section name.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => {}
                    }

                    self

                // Valid section name char - keep parsing the section name.
                } else if options.is_key_or_value_char(c, true, None) {
                    state.buffer.push(c);

                    self

                // Section end delimiter - finish the section name, skip the rest of the line.
                } else if options.is_section_end(c) {
                    debug_assert!(state.path.len() <= options.nested_section_depth);

                    // Empty section names not allowed.
                    let section = NonEmptyStr::new(&state.buffer)
                        .ok_or((EmptySectionName(state.path.to_config_path()), false))?;

                    // Try to add the section to the config at the current path.
                    state.skip_section = start_section(config, section, &state.path, options)?;

                    state.path.push(section);
                    state.buffer.clear();

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
                debug_assert!(options.is_string_quote_char(quote));
                debug_assert!(state.key.is_empty());

                // New line before the closing quotes - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInSectionName, true));

                // Closing quotes - keep parsing until the nested section separator or section end delimiter.
                } else if c == quote {
                    IniParserFSMState::AfterSection

                // Escaped char (if supported) - keep parsing the section name.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the section name.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => {}
                    }

                    self

                // Non-matching quotes - keep parsing the section.
                } else if options.is_non_matching_string_quote_char(quote, c) {
                    state.buffer.push(c);

                    self

                // Space or valid value char - keep parsing the section.
                } else if c == ' ' || options.is_key_or_value_char(c, true, Some(quote)) {
                    state.buffer.push(c);

                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInSectionName(c), false));
                }
            }
            IniParserFSMState::AfterSection => {
                debug_assert!(state.key.is_empty());

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
                    let section = NonEmptyStr::new(&state.buffer)
                        .ok_or((EmptySectionName(state.path.to_config_path()), true))?;

                    // Try to add the section to the config at the current path.
                    state.skip_section = start_section(config, section, &state.path, options)?;

                    state.path.push(section);
                    state.buffer.clear();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // Nested section separator (if supported) - start parsing the nested section name.
                } else if options.is_nested_section_separator(c) {
                    if (state.path.len() + 1) >= options.nested_section_depth {
                        return Err((NestedSectionDepthExceeded, false));
                    }

                    // Empty section names are not allowed.
                    let section = NonEmptyStr::new(&state.buffer)
                        .ok_or((EmptySectionName(state.path.to_config_path()), true))?;

                    state.path.push(section);

                    // The path must already exist in the config.
                    if config.contains_key(section) != Ok(true) {
                        return Err((InvalidParentSection(state.path.to_config_path()), true));
                    }

                    // Start the parent section in the config.
                    config.start_section(section, false);

                    state.buffer.clear();

                    IniParserFSMState::BeforeSection

                // Else an error.
                } else {
                    return Err((InvalidCharacterAfterSectionName(c), false));
                }
            }
            IniParserFSMState::SkipLine => {
                debug_assert!(state.key.is_empty());
                debug_assert!(state.buffer.is_empty());

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
                debug_assert!(state.buffer.is_empty());

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
                debug_assert!(!state.buffer.is_empty());
                debug_assert!(state.key.is_empty());

                // Key-value separator - finish the key, parse the value.
                if options.is_key_value_separator_char(c) {
                    std::mem::swap(&mut state.key, &mut state.buffer);

                    check_is_key_duplicate(
                        config,
                        // Must succeed.
                        unwrap_unchecked_msg(NonEmptyStr::new(&state.key), "empty key"),
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

                    std::mem::swap(&mut state.key, &mut state.buffer);

                    check_is_key_duplicate(
                        config,
                        // Must succeed.
                        unwrap_unchecked_msg(NonEmptyStr::new(&state.key), "empty key"),
                        state.skip_section,
                        &mut state.skip_value,
                        &mut state.is_key_unique,
                        options.duplicate_keys,
                    )?;

                    IniParserFSMState::KeyValueSeparator

                // Escaped char (if supported) - keep parsing the key.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the key.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => {}
                    }

                    self

                // Valid key char - keep parsing the key.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.buffer.push(c);

                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInKey(c), false));
                }
            }
            IniParserFSMState::QuotedKey(quote) => {
                debug_assert!(options.is_string_quote_char(quote));
                debug_assert!(state.key.is_empty());

                // New line before the closing quotes - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInKey, true));

                // Closing quotes - finish the key, parse the separator.
                } else if c == quote {
                    std::mem::swap(&mut state.key, &mut state.buffer);

                    // Empty keys are not allowed.
                    let key = NonEmptyStr::new(&state.key).ok_or((EmptyKey, false))?;

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
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the key.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => {}
                    }

                    self

                // Non-matching quotes - keep parsing the key.
                } else if options.is_non_matching_string_quote_char(quote, c) {
                    state.buffer.push(c);

                    self

                // Space or valid key char - keep parsing the key.
                } else if c == ' ' || options.is_key_or_value_char(c, false, Some(quote)) {
                    state.buffer.push(c);

                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInKey(c), false));
                }
            }
            IniParserFSMState::KeyValueSeparator => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(state.buffer.is_empty());

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
                debug_assert!(state.buffer.is_empty());

                // Skip the whitespace before the value.
                if c.is_whitespace() {
                    // Unless it's a new line - the value is empty.
                    if options.is_new_line(c) {
                        add_value_to_config(
                            config,
                            // Must succeed.
                            unwrap_unchecked_msg(NonEmptyStr::new(&state.key), "empty key"),
                            "",
                            false,
                            state.skip_section | state.skip_value,
                            state.is_key_unique,
                            options.unquoted_strings,
                        )
                        .map_err(|error_kind| (error_kind, false))?;
                        state.key.clear();

                        IniParserFSMState::StartLine
                    } else {
                        self
                    }

                // Inline comment (if supported) - the value is empty, skip the rest of the line.
                } else if options.is_inline_comment_char(c) {
                    add_value_to_config(
                        config,
                        // Must succeed.
                        unwrap_unchecked_msg(NonEmptyStr::new(&state.key), "empty key"),
                        "",
                        false,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                        options.unquoted_strings,
                    )
                    .map_err(|error_kind| (error_kind, false))?;
                    state.key.clear();

                    IniParserFSMState::SkipLine

                // String quote - parse the string value in quotes, expecting the matching quotes.
                } else if options.is_string_quote_char(c) {
                    IniParserFSMState::QuotedValue(c)

                // Escaped char (if supported) - parse the escape sequence, start parsing the value.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - start parsing the value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);

                            IniParserFSMState::Value
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => self,
                    }

                // Array start delimiter (if supported) - start parsing the array.
                } else if options.is_array_start(c) {
                    // Must succeed.
                    let array_key = unwrap_unchecked_msg(NonEmptyStr::new(&state.key), "empty key");

                    add_array_to_config(
                        config,
                        array_key,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                    );

                    state.path.push(array_key);

                    IniParserFSMState::BeforeArrayValue(None)

                // Valid value char - start parsing the unquoted value.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.buffer.push(c);

                    IniParserFSMState::Value

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::Value => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(!state.buffer.is_empty());

                // Whitespace - finish the value.
                if c.is_whitespace() {
                    add_value_to_config(
                        config,
                        // Must succeed.
                        unwrap_unchecked_msg(NonEmptyStr::new(&state.key), "empty key"),
                        &state.buffer,
                        false,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                        options.unquoted_strings,
                    )
                    .map_err(|error_kind| (error_kind, false))?;
                    state.buffer.clear();
                    state.key.clear();

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
                        unwrap_unchecked_msg(NonEmptyStr::new(&state.key), "empty key"),
                        &state.buffer,
                        false,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                        options.unquoted_strings,
                    )
                    .map_err(|error_kind| (error_kind, false))?;
                    state.buffer.clear();
                    state.key.clear();

                    IniParserFSMState::SkipLine

                // Escaped char (if supported) - parse the escape sequence.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => {}
                    }

                    self

                // Valid value char - keep parsing the value.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.buffer.push(c);

                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::QuotedValue(quote) => {
                debug_assert!(options.is_string_quote_char(quote));
                debug_assert!(!state.key.is_empty());

                // New line before the closing quotes - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInQuotedValue, true));

                // Closing quotes - finish the value (may be empty), skip the rest of the line.
                } else if c == quote {
                    add_value_to_config(
                        config,
                        // Must succeed.
                        unwrap_unchecked_msg(NonEmptyStr::new(&state.key), "empty key"),
                        &state.buffer,
                        true,
                        state.skip_section | state.skip_value,
                        state.is_key_unique,
                        options.unquoted_strings,
                    )
                    .map_err(|error_kind| (error_kind, false))?;
                    state.buffer.clear();
                    state.key.clear();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // Escaped char (if supported) - parse the escape sequence.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => {}
                    }

                    self

                // Non-matching quotes - keep parsing the value.
                } else if options.is_non_matching_string_quote_char(quote, c) {
                    state.buffer.push(c);

                    self

                // Space or valid value char - keep parsing the value.
                } else if c == ' ' || options.is_key_or_value_char(c, false, Some(quote)) {
                    state.buffer.push(c);

                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::BeforeArrayValue(array_type) => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(state.buffer.is_empty());
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
                    config.end_array(unwrap_unchecked_msg(
                        NonEmptyStr::new(&state.key),
                        "missing array key",
                    ));

                    // Pop the array key off the path.
                    state.path.pop();
                    state.key.clear();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // String quote - parse the string array value in quotes, expecting the matching quotes.
                } else if options.is_string_quote_char(c) {
                    // Make sure the array is empty or contains strings (is not mixed).
                    if let Some(array_type) = array_type {
                        if !array_type.is_compatible(IniValueType::String) {
                            return Err((IniErrorKind::MixedArray, false));
                        }
                    }

                    IniParserFSMState::QuotedArrayValue(c, array_type)

                // Escaped char (if supported) - parse the escape sequence, start parsing the array value.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);

                            IniParserFSMState::ArrayValue(array_type)
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => self,
                    }

                // Valid value char - start parsing the unquoted array value.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.buffer.push(c);

                    IniParserFSMState::ArrayValue(array_type)

                // Else an error.
                } else {
                    return Err((InvalidCharacterInArray(c), false));
                }
            }
            IniParserFSMState::ArrayValue(mut array_type) => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(!state.buffer.is_empty());
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
                        &state.buffer,
                        false,
                        state.skip_value | state.skip_section,
                        &mut array_type,
                        options.unquoted_strings,
                    )?;

                    state.buffer.clear();

                    IniParserFSMState::AfterArrayValue(array_type)

                // Array value separator - finish the current array value,
                // parse the next array value / array end delimiter.
                } else if options.is_array_value_separator(c) {
                    add_value_to_array(
                        config,
                        &state.buffer,
                        false,
                        state.skip_value | state.skip_section,
                        &mut array_type,
                        options.unquoted_strings,
                    )?;

                    state.buffer.clear();

                    IniParserFSMState::BeforeArrayValue(array_type)

                // Array end delimiter - add the value to the array, finish the array, skip the rest of the line.
                } else if options.is_array_end(c) {
                    add_value_to_array(
                        config,
                        &state.buffer,
                        false,
                        state.skip_value | state.skip_section,
                        &mut array_type,
                        options.unquoted_strings,
                    )?;

                    state.buffer.clear();

                    // Must succeed.
                    config.end_array(unwrap_unchecked_msg(
                        NonEmptyStr::new(&state.key),
                        "missing array key",
                    ));

                    // Pop the array key off the path.
                    state.path.pop();
                    state.key.clear();

                    IniParserFSMState::SkipLineWhitespaceOrComments

                // Escaped char (if supported) - parse the escape sequence.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the array value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => {}
                    }

                    self

                // Valid value char - keep parsing the array value.
                } else if options.is_key_or_value_char(c, false, None) {
                    state.buffer.push(c);

                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::QuotedArrayValue(quote, mut array_type) => {
                debug_assert!(options.is_string_quote_char(quote));
                debug_assert!(!state.key.is_empty());

                // New line before the closing quotes - error.
                if options.is_new_line(c) {
                    return Err((UnexpectedNewLineInQuotedValue, true));

                // Closing quotes - finish the array value (may be empty),
                // parse the array value separator / array end delimiter.
                } else if c == quote {
                    add_value_to_array(
                        config,
                        &state.buffer,
                        true,
                        state.skip_value | state.skip_section,
                        &mut array_type,
                        options.unquoted_strings,
                    )?;
                    state.buffer.clear();

                    IniParserFSMState::AfterArrayValue(array_type)

                // Escaped char (if supported) - parse the escape sequence.
                } else if options.is_escape_char(c) {
                    match parse_escape_sequence(
                        next,
                        &mut state.escape_sequence_buffer,
                        false,
                        options,
                    )? {
                        // Parsed an escaped char - keep parsing the array value.
                        ParseEscapeSequenceResult::EscapedChar(c) => {
                            state.buffer.push(c);
                        }
                        // Line continuation - keep parsing.
                        ParseEscapeSequenceResult::LineContinuation => {}
                    }

                    self

                // Non-matching quotes - keep parsing the array value.
                } else if options.is_non_matching_string_quote_char(quote, c) {
                    state.buffer.push(c);

                    self

                // Space or valid value char - keep parsing the array value.
                } else if c == ' ' || options.is_key_or_value_char(c, false, Some(quote)) {
                    state.buffer.push(c);

                    self

                // Else an error.
                } else {
                    return Err((InvalidCharacterInValue(c), false));
                }
            }
            IniParserFSMState::AfterArrayValue(array_type) => {
                debug_assert!(!state.key.is_empty());
                debug_assert!(state.buffer.is_empty());
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
                    IniParserFSMState::BeforeArrayValue(array_type)

                // Array end delimiter - finish the array, skip the rest of the line.
                } else if options.is_array_end(c) {
                    // Must succeed.
                    config.end_array(unwrap_unchecked_msg(
                        NonEmptyStr::new(&state.key),
                        "empty key",
                    ));

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
    fn finish<'a, C: IniConfig>(
        self,
        config: &mut C,
        state: &IniParserPersistentState,
        options: &IniOptions,
    ) -> Result<(), IniErrorKind<'a>> {
        use IniErrorKind::*;
        use IniParserFSMState::*;

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

                add_value_to_config(
                    config,
                    // Must succeed.
                    unwrap_unchecked_msg(NonEmptyStr::new(&state.key), "empty key"),
                    &state.buffer,
                    false,
                    state.skip_section | state.skip_value,
                    state.is_key_unique,
                    options.unquoted_strings,
                )
            }
            BeforeArrayValue(_) | ArrayValue(_) | AfterArrayValue(_) => {
                return Err(UnexpectedEndOfFileInArray)
            }
            QuotedArrayValue(_, _) => return Err(UnexpectedEndOfFileInQuotedArrayValue),
            StartLine | SkipLine | SkipLineWhitespaceOrComments => Ok(()),
        }
    }
}

/// Reads up to 4 following characters and tries to parse them as an escape sequence.
/// `in_unquoted_section` is `true` if we are parsing an unquoted `.ini` section name.
fn parse_escape_sequence<'a, F: FnMut() -> Option<char>>(
    mut next: F,
    escape_sequence_buffer: &mut String,
    in_unquoted_section: bool,
    options: &IniOptions,
) -> Result<ParseEscapeSequenceResult, (IniErrorKind<'a>, bool)> {
    use IniErrorKind::*;
    use ParseEscapeSequenceResult::*;

    debug_assert!(options.escape);

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

        // 2 hexadecimal ASCII values.
        Some('x') => {
            escape_sequence_buffer.clear();

            for _ in 0..2 {
                match next() {
                    None => return Err((UnexpectedEndOfFileInASCIIEscapeSequence, false)),
                    Some('\n') | Some('\r') => {
                        return Err((UnexpectedNewLineInASCIIEscapeSequence, true))
                    }
                    Some(c) => {
                        if !c.is_digit(16) {
                            return Err((InvalidASCIIEscapeSequence, false));
                        }

                        escape_sequence_buffer.push(c);
                    }
                }
            }

            Ok(EscapedChar(
                std::char::from_u32(
                    u32::from_str_radix(&escape_sequence_buffer, 16)
                        .map_err(|_| (InvalidASCIIEscapeSequence, false))?,
                )
                .ok_or((InvalidASCIIEscapeSequence, false))?,
            ))
        }

        // 4 hexadecimal Unicode values.
        Some('u') => {
            escape_sequence_buffer.clear();

            for _ in 0..4 {
                match next() {
                    None => return Err((UnexpectedEndOfFileInUnicodeEscapeSequence, false)),
                    Some('\n') | Some('\r') => {
                        return Err((UnexpectedNewLineInUnicodeEscapeSequence, true))
                    }
                    Some(c) => {
                        if !c.is_digit(16) {
                            return Err((InvalidUnicodeEscapeSequence, false));
                        }

                        escape_sequence_buffer.push(c);
                    }
                }
            }

            Ok(EscapedChar(
                std::char::from_u32(
                    u32::from_str_radix(&escape_sequence_buffer, 16)
                        .map_err(|_| (InvalidUnicodeEscapeSequence, false))?,
                )
                .ok_or((InvalidUnicodeEscapeSequence, false))?,
            ))
        }

        Some(c) => Err((InvalidEscapeCharacter(c), false)),
    }
}

/// Returns `Ok(true)` if we need to skip the current section;
/// else returns `Ok(false)`.
fn start_section<'a, C: IniConfig>(
    config: &mut C,
    section: NonEmptyStr<'_>,
    path: &IniPath,
    options: &IniOptions,
) -> Result<bool, (IniErrorKind<'a>, bool)> {
    let key_already_exists = config.contains_key(section);

    // Section already exists.
    if let Ok(true) = key_already_exists {
        match options.duplicate_sections {
            // We don't support duplicate sections - error.
            IniDuplicateSections::Forbid => {
                let mut path = path.to_config_path();
                path.0.push(section.as_ref().to_owned().into());

                return Err((IniErrorKind::DuplicateSection(path), false));
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
                    let mut path = path.to_config_path();
                    path.0.push(section.as_ref().to_owned().into());

                    return Err((
                        IniErrorKind::DuplicateKey(section.as_ref().to_owned().into()),
                        true,
                    ));
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
fn check_is_key_duplicate<'a, 'k, C: IniConfig>(
    config: &C,
    key: NonEmptyStr<'k>,
    skip_section: bool,
    skip_value: &mut bool,
    is_key_unique: &mut bool,
    duplicate_keys: IniDuplicateKeys,
) -> Result<(), (IniErrorKind<'a>, bool)> {
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
                Err((DuplicateKey(key.as_ref().to_string().into()), true))
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
fn add_value_to_config<'a, C: IniConfig>(
    config: &mut C,
    key: NonEmptyStr<'_>,
    value: &str,
    quoted: bool,
    skip: bool,
    is_key_unique: bool,
    unquoted_strings: bool,
) -> Result<(), IniErrorKind<'a>> {
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
/// Updates the `array_type`.
fn add_value_to_array<'a, C: IniConfig>(
    config: &mut C,
    value: &str,
    quoted: bool,
    skip: bool,
    array_type: &mut Option<IniValueType>,
    unquoted_strings: bool,
) -> Result<(), (IniErrorKind<'a>, bool)> {
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
fn parse_value_string<'a>(
    value: &str,
    quoted: bool,
    unquoted_strings: bool,
) -> Result<IniValue<&str>, IniErrorKind<'a>> {
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
    } else if let Some(value) = try_parse_integer(value) {
        I64(value)

    // Else check if it's a float.
    } else if let Ok(value) = value.parse::<f64>() {
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

/// Persistent state used to communicate information between parser FSM states.
struct IniParserPersistentState {
    // Scratch buffer for sections / keys / values.
    pub buffer: String,
    // Current nested section path, if any.
    // Contains at most one section name if nested sections are not supported.
    pub path: IniPath,
    // Current key, if any.
    pub key: String,
    // Whether the key is unique in its table (root or section).
    pub is_key_unique: bool,
    // Whether we need to skip all key/value pairs in the current section
    // (i.e., when we encountered a duplicate section instance and we use the `First` duplicate section policy).
    pub skip_section: bool,
    // Whether we need to skip the current value
    // (i.e., when we encountered a duplicate key and we use the `First` duplicate key policy).
    pub skip_value: bool,
    // Scratch buffer for ASCII / Unicode escape sequences, if supported.
    pub escape_sequence_buffer: String,
}

impl IniParserPersistentState {
    fn new(escape: bool) -> Self {
        Self {
            buffer: String::new(),
            path: IniPath::new(),
            key: String::new(),
            is_key_unique: true,
            skip_section: false,
            skip_value: false,
            escape_sequence_buffer: if escape {
                String::with_capacity(4)
            } else {
                String::new()
            },
        }
    }
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
    cr: bool,

    /// Parsing options as provided by the user.
    options: IniOptions,
}

impl<'s> IniParser<'s> {
    /// Creates a new [`parser`](struct.IniParser.html) from the `.ini` config `string`
    /// using default parsing options.
    pub fn new(string: &'s str) -> Self {
        Self {
            reader: string.chars(),
            line: 1,
            column: 0,
            new_line: false,
            cr: false,
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
        self.validate_options();

        let mut persistent_state = IniParserPersistentState::new(self.options.escape);

        let options = self.options;

        let mut fsm_state = IniParserFSMState::StartLine;

        // Read the chars until EOF, process according to current state.
        while let Some(c) = self.next() {
            fsm_state = fsm_state
                .process(c, || self.next(), config, &mut persistent_state, &options)
                .map_err(|(error_kind, offset)| self.error(error_kind, offset))?;
        }

        fsm_state
            .finish(config, &persistent_state, &options)
            .map_err(|error_kind| self.error(error_kind, false))?;

        while let Some(section) = persistent_state.path.last() {
            // We didn't call `start_section()` if we skipped it, so don't call `end_section`.
            if !persistent_state.skip_section {
                config.end_section(section);
            } else {
                persistent_state.skip_section = false;
            }
            persistent_state.path.pop();
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
            // Eat a line feed if the previous char was a carriage return.
            Some('\n') if self.cr => {
                self.cr = false;
            }
            Some('\r') => {
                self.column += 1;
                self.new_line = true;

                self.cr = true;
            }
            Some('\n') => {
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
    fn error<'a>(&self, error: IniErrorKind<'a>, offset: bool) -> IniError<'a> {
        if offset {
            debug_assert!(self.column > 0);
        }

        IniError {
            line: self.line,
            column: if offset { self.column - 1 } else { self.column },
            error,
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
}
