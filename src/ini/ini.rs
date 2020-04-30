use std::str::Chars;

use crate::{
    DynConfig, DynTable, DynTableMut, IniCommentSeparator, IniError, IniErrorKind,
    IniKeyValueSeparator, IniOptions, IniStringQuote, Value, IniDuplicateSections, IniDuplicateKeys
};

/// INI parser FSM states.
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
    /// valid value chars (-> Value),
    /// string quotes ('"' / '\'') (if supported) (-> QuotedString),
    /// escape sequences (if supported) (-> Value).
    BeforeValue,
    /// Accept valid value chars,
    /// escape sequences (if supported),
    /// whitespace (-> SkipLineWhitespaceOrComments) (including new lines (-> StartLine)),
    /// inline comment delimiters (';' / '#') (if supported) (-> SkipLine).
    Value,
    /// Accept valid value chars,
    /// escape sequences (if supported),
    /// non-matching string quotes (if supported),
    /// spaces (' '),
    /// matching string quotes ('"' / '\'') (-> SkipLineWhitespaceOrComments).
    QuotedString,
}

struct IniParser<'s> {
    /// Source string reader.
    reader: Chars<'s>,

    /// Current position in the source string.
    line: u32,
    column: u32,
    new_line: bool,

    /// Current parser FSM state.
    state: IniParserState,

    // Parsing options as provided by the user.
    options: IniOptions,
}

impl<'s> IniParser<'s> {
    fn new(reader: Chars<'s>, mut options: IniOptions) -> Self {
        // Must have some key-value separator if none provided by the user - use `Equals`.
        if options.key_value_separator.is_empty() {
            options.key_value_separator = IniKeyValueSeparator::Equals;
        }

        // If not using quoted strings, unquoted strings must be supported.
        if options.string_quotes.is_empty() {
            options.unquoted_strings = true;
        }

        Self {
            reader,
            state: IniParserState::StartLine,
            line: 1,
            column: 0,
            new_line: false,
            options,
        }
    }

    /// Consumes the parser, returns the parsed config or an error.
    fn parse(mut self) -> Result<DynConfig, IniError> {
        use IniErrorKind::*;

        let mut config = DynConfig::new();
        let mut root = config.root_mut();

        // Scratch buffer for sections / keys / values.
        let mut buffer = String::new();

        // Current section, if any.
        let mut section = String::new();

        // Current key, if any.
        let mut key = String::new();

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

        // Read the chars until EOF, process according to current state.
        while let Some(current) = self.next() {
            match self.state {
                IniParserState::StartLine => {
                    skip_value = false;

                    // Skip whitespace at the start of the line (including new lines).
                    if current.is_whitespace() {

                        // Section start delimiter - parse the section name.
                    } else if current == '[' {
                        skip_section = false;

                        self.state = IniParserState::BeforeSection;

                    // Line comment (if supported) - skip the rest of the line.
                    } else if self.is_comment_char(current) {
                        self.state = IniParserState::SkipLine;

                    // String quote - parse the key in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(current) {
                        debug_assert!(quote.is_none());
                        quote.replace(current);

                        self.state = IniParserState::QuotedKey;

                    // Escaped char (if supported) - parse the escape sequence as the key.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - start parsing the key.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                debug_assert!(buffer.is_empty());
                                buffer.push(current);

                                self.state = IniParserState::Key;
                            }
                            // Line continuation - error.
                            ParseEscapeSequenceResult::LineContinuation => {
                                return Err(self.error(UnexpectedNewLineInKey));
                            }
                        }

                    // Valid key start - parse the key.
                    } else if self.is_key_or_value_char(current, quote) {
                        debug_assert!(buffer.is_empty());
                        buffer.push(current);

                        self.state = IniParserState::Key;

                    // Key-value separator - it's an empty key.
                    } else if self.is_key_value_separator_char(current) {
                        return Err(self.error_offset(EmptyKey));

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterAtLineStart));
                    }
                }
                IniParserState::BeforeSection => {
                    debug_assert!(buffer.is_empty());

                    // Skip whitespace.
                    if current.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(current) {
                            return Err(self.error_offset(UnexpectedNewLineInSectionName));
                        }

                    // String quote - parse the section name in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(current) {
                        debug_assert!(quote.is_none());
                        quote.replace(current);

                        self.state = IniParserState::QuotedSection;

                    // Escaped char (if supported) - start parsing the section name.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the section name.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                buffer.push(current);

                                self.state = IniParserState::Section;
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid section name char (same rules as key chars) - start parsing the section name.
                    } else if self.is_key_or_value_char(current, quote) {
                        buffer.push(current);

                        self.state = IniParserState::Section;

                    // Section end delimiter - empty section names not allowed.
                    } else if current == ']' {
                        return Err(self.error(EmptySectionName));

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName));
                    }
                }
                IniParserState::Section => {
                    debug_assert!(!buffer.is_empty());

                    // New line before the section delimiter - error.
                    if self.is_new_line(current) {
                        return Err(self.error_offset(UnexpectedNewLineInSectionName));

                    // Escaped char (if supported) - keep parsing the section name.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the section name.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                buffer.push(current);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid section name char (same rules as key chars) - keep parsing the section name.
                    } else if self.is_key_or_value_char(current, quote) {
                        buffer.push(current);

                    // Section end delimiter - finish the section name, skip the rest of the line.
                    } else if current == ']' {
                        // Empty section names not allowed.
                        if buffer.is_empty() {
                            return Err(self.error(EmptySectionName));
                        }

                        section.clear();
                        section.push_str(&buffer);
                        buffer.clear();

                        // Try to add the section to the config.
                        skip_section = self.add_section(&mut root, &section)?;

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Whitespace after section name (new lines handle above) - skip it, finish the section name, parse the section end delimiter.
                    // NOTE - section name is not empty if we got here.
                    } else if current.is_whitespace() {
                        section.clear();
                        section.push_str(&buffer);
                        buffer.clear();

                        // Try to add the section to the config.
                        skip_section = self.add_section(&mut root, &section)?;

                        self.state = IniParserState::AfterSection;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName));
                    }
                }
                IniParserState::QuotedSection => {
                    let cur_quote = quote.unwrap();

                    // New line before the closing quotes - error.
                    if self.is_new_line(current) {
                        return Err(self.error_offset(UnexpectedNewLineInSectionName));

                    // Closing quotes - finish the quoted section, keep parsing until the section delimiter.
                    } else if current == cur_quote {
                        // Empty section names not allowed.
                        if buffer.is_empty() {
                            return Err(self.error(EmptySectionName));
                        }

                        quote.take();

                        section.clear();
                        section.push_str(&buffer);
                        buffer.clear();

                        // Try to add the section to the config.
                        skip_section = self.add_section(&mut root, &section)?;

                        self.state = IniParserState::AfterSection;

                    // Escaped char (if supported) - keep parsing the section name.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the section name.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                buffer.push(current);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Non-matching quotes - keep parsing the value.
                    } else if self.is_non_matching_string_quote_char(cur_quote, current) {
                        buffer.push(current);

                    // Space or valid value char - keep parsing the value.
                    } else if current == ' ' || self.is_key_or_value_char(current, quote) {
                        buffer.push(current);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName));
                    }
                }
                IniParserState::AfterSection => {
                    debug_assert!(buffer.is_empty());
                    debug_assert!(!section.is_empty());

                    // Skip whitespace.
                    if current.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(current) {
                            return Err(self.error_offset(UnexpectedNewLineInSectionName));
                        }

                    // Section end delimiter - skip the rest of the line.
                    } else if current == ']' {
                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterAfterSectionName));
                    }
                }
                IniParserState::SkipLine => {
                    debug_assert!(buffer.is_empty());

                    // If it's a new line, start parsing the next line.
                    // Skip everything else.
                    if self.is_new_line(current) {
                        self.state = IniParserState::StartLine;
                    }
                }
                IniParserState::SkipLineWhitespaceOrComments => {
                    debug_assert!(buffer.is_empty());

                    // If it's a new line, start parsing the next line.
                    if self.is_new_line(current) {
                        self.state = IniParserState::StartLine;

                    // Skip other whitespace.
                    } else if current.is_whitespace() {
                        // continue

                        // Inline comment (if supported) - skip the rest of the line.
                    } else if self.options.inline_comments && self.is_comment_char(current) {
                        self.state = IniParserState::SkipLine;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterAtLineEnd));
                    }
                }
                IniParserState::Key => {
                    // We have at least one key character already parsed.
                    debug_assert!(!buffer.is_empty());

                    // Key-value separator - finish the key, parse the value.
                    if self.is_key_value_separator_char(current) {
                        debug_assert!(key.is_empty());
                        key.push_str(&buffer);
                        buffer.clear();

                        skip_value = self.check_is_key_duplicate(&mut root, &section, &key, skip_section)?;

                        self.state = IniParserState::BeforeValue;

                    // Whitespace between the key and the separator - skip it, finish the key, parse the separator.
                    } else if current.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(current) {
                            return Err(self.error_offset(UnexpectedNewLineInKey));
                        }

                        debug_assert!(key.is_empty());
                        key.push_str(&buffer);
                        buffer.clear();

                        skip_value = self.check_is_key_duplicate(&mut root, &section, &key, skip_section)?;

                        self.state = IniParserState::KeyValueSeparator;

                    // Escaped char (if supported) - keep parsing the key.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the key.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                buffer.push(current);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid key char - keep parsing the key.
                    } else if self.is_key_or_value_char(current, quote) {
                        buffer.push(current);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInKey));
                    }
                }
                IniParserState::QuotedKey => {
                    let cur_quote = quote.unwrap();

                    // New line before the closing quotes - error.
                    if self.is_new_line(current) {
                        return Err(self.error_offset(UnexpectedNewLineInKey));

                    // Closing quotes - finish the key, parse the separator.
                    } else if current == cur_quote {
                        // Empty keys are not allowed.
                        if buffer.is_empty() {
                            return Err(self.error(EmptyKey));
                        }

                        quote.take();

                        debug_assert!(key.is_empty());
                        key.push_str(&buffer);
                        buffer.clear();

                        skip_value = self.check_is_key_duplicate(&mut root, &section, &key, skip_section)?;

                        self.state = IniParserState::KeyValueSeparator;

                    // Escaped char (if supported) - keep parsing the key.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the key.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                buffer.push(current);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Non-matching quotes - keep parsing the key.
                    } else if self.is_non_matching_string_quote_char(cur_quote, current) {
                        buffer.push(current);

                    // Space or valid key char - keep parsing the key.
                    } else if current == ' ' || self.is_key_or_value_char(current, quote) {
                        buffer.push(current);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInKey));
                    }
                }
                IniParserState::KeyValueSeparator => {
                    debug_assert!(buffer.is_empty());

                    // Key-value separator - parse the value (key already finished).
                    if self.is_key_value_separator_char(current) {
                        self.state = IniParserState::BeforeValue;

                    // Skip the whitespace between the key and the separator.
                    } else if current.is_whitespace() {
                        // Unless it's a new line.
                        if self.is_new_line(current) {
                            return Err(self.error_offset(UnexpectedNewLineInKey));
                        }

                    // Else an error.
                    } else {
                        return Err(self.error(UnexpectedCharacterInsteadOfKeyValueSeparator));
                    }
                }
                IniParserState::BeforeValue => {
                    debug_assert!(buffer.is_empty());
                    debug_assert!(!key.is_empty());

                    // Skip the whitespace before the value.
                    if current.is_whitespace() {
                        // Unless it's a new line - the value is empty.
                        if self.is_new_line(current) {
                            self.add_value(&mut root, &section, &key, "", false, skip_section | skip_value)?;
                            key.clear();

                            self.state = IniParserState::StartLine;
                        }

                    // Inline comment (if supported) - the value is empty, skip the rest of the line.
                    } else if self.options.inline_comments && self.is_comment_char(current) {
                        self.add_value(&mut root, &section, &key, "", false, skip_section | skip_value)?;
                        key.clear();

                        self.state = IniParserState::SkipLine;

                    // String quote - parse the string value in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(current) {
                        debug_assert!(quote.is_none());
                        quote.replace(current);

                        self.state = IniParserState::QuotedString;

                    // Escaped char (if supported) - parse the escape sequence, start parsing the value.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - start parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                debug_assert!(buffer.is_empty());
                                buffer.push(current);

                                self.state = IniParserState::Value;
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid value char - start parsing the unquoted value.
                    } else if self.is_key_or_value_char(current, quote) {
                        buffer.push(current);

                        self.state = IniParserState::Value;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue));
                    }
                }
                IniParserState::Value => {
                    debug_assert!(!buffer.is_empty());
                    debug_assert!(!key.is_empty());

                    // New line - finish the value, start the new line.
                    if self.is_new_line(current) {
                        self.add_value(&mut root, &section, &key, &buffer, false, skip_section | skip_value)?;
                        buffer.clear();
                        key.clear();

                        self.state = IniParserState::StartLine;

                    // Whitespace (except a new line - handled above) - finish the value, skip the rest of the line.
                    } else if current.is_whitespace() {
                        debug_assert!(!self.is_new_line(current));
                        self.add_value(&mut root, &section, &key, &buffer, false, skip_section | skip_value)?;
                        buffer.clear();
                        key.clear();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Inline comment (if supported) - finish the value, skip the rest of the line.
                    } else if self.options.inline_comments && self.is_comment_char(current) {
                        self.add_value(&mut root, &section, &key, &buffer, false, skip_section | skip_value)?;
                        buffer.clear();
                        key.clear();

                        self.state = IniParserState::SkipLine;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                buffer.push(current);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Valid value char - keep parsing the value.
                    } else if self.is_key_or_value_char(current, quote) {
                        buffer.push(current);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue));
                    }
                }
                IniParserState::QuotedString => {
                    debug_assert!(!key.is_empty());

                    let cur_quote = quote.unwrap();

                    // New line before the closing quotes - error.
                    if self.is_new_line(current) {
                        return Err(self.error_offset(UnexpectedNewLineInQuotedString));

                    // Closing quotes - finish the value (may be empty), skip the rest of the line.
                    } else if current == cur_quote {
                        self.add_value(&mut root, &section, &key, &buffer, true, skip_section | skip_value)?;
                        buffer.clear();
                        key.clear();

                        quote.take();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                buffer.push(current);
                            }
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {}
                        }

                    // Non-matching quotes - keep parsing the value.
                    } else if self.is_non_matching_string_quote_char(cur_quote, current) {
                        buffer.push(current);

                    // Space or valid value char - keep parsing the value.
                    } else if current == ' ' || self.is_key_or_value_char(current, quote) {
                        buffer.push(current);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue));
                    }
                }
            }
        }

        match self.state {
            IniParserState::Section
            | IniParserState::QuotedSection
            | IniParserState::AfterSection => {
                return Err(self.error(UnexpectedEndOfFileInSectionName))
            }
            IniParserState::Key | IniParserState::QuotedKey | IniParserState::KeyValueSeparator => {
                return Err(self.error(UnexpectedEndOfFileBeforeKeyValueSeparator))
            }
            IniParserState::QuotedString => {
                return Err(self.error(UnexpectedEndOfFileInQuotedString))
            }

            // Add the last value if we were parsing it right before EOF.
            IniParserState::Value | IniParserState::BeforeValue => {
                self.add_value(&mut root, &section, &key, &buffer, quote.is_some(), skip_section | skip_value)?;
            }

            _ => {}
        }

        Ok(config)
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
                .contains(IniCommentSeparator::Semicolon))
            || ((val == '#')
                && self
                    .options
                    .comments
                    .contains(IniCommentSeparator::NumberSign))
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

            // Escaped INI special characters, disallowed otherwise.
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
                        Some(current) => unicode_buffer.push(current),
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

            Some(_) => Err(self.error(InvalidEscapeCharacter)),
        }
    }

    /// Returns `Ok(true)` if we need to skip the current section;
    /// else returns `Ok(false)`.
    fn add_section(&self, root: &mut DynTableMut<'_>, section: &str) -> Result<bool, IniError> {
        // Section does not exist in the config - add it.
        if root.get(section).is_err() {
            root.set(section, Value::Table(DynTable::new())).unwrap();
            Ok(false)

        // Section already exists.
        } else {
            match self.options.duplicate_sections {
                // We don't support duplicate sections - error.
                IniDuplicateSections::Forbid => return Err(self.error(IniErrorKind::DuplicateSection)),
                // Skip this section.
                IniDuplicateSections::First => Ok(true),
                // Overwrite the previous instance section with the new one.
                IniDuplicateSections::Last => {
                    root.set(section, Value::Table(DynTable::new())).unwrap();
                    Ok(false)
                },
                // Just add the new key/value pairs to the existing section.
                IniDuplicateSections::Merge => Ok(false),
            }
        }
    }

    /// Parses a string `value` and adds it to the config `section` at `key`.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn add_value(
        &self,
        root: &mut DynTableMut<'_>,
        section: &str,
        key: &str,
        value: &str,
        quoted: bool,
        skip: bool,
    ) -> Result<(), IniError> {
        debug_assert!(!key.is_empty());

        if skip {
            return Ok(());
        }

        let value = self.parse_value_string(value, quoted)?;

        if section.is_empty() {
            debug_assert!(self.options.duplicate_keys.allow_non_unique() || root.get(key).is_err());
            Self::add_value_to_table(key, value, root);
        } else {
            // Must succeed.
            let mut table = root.get_mut(section).unwrap().table().unwrap();

            debug_assert!(self.options.duplicate_keys.allow_non_unique() || table.get(key).is_err());
            Self::add_value_to_table(key, value, &mut table);
        }

        Ok(())
    }

    /// Parses a string `value`.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn parse_value_string<'v>(
        &self,
        value: &'v str,
        quoted: bool,
    ) -> Result<IniValue<'v>, IniError> {
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

    fn add_value_to_table(key: &str, val: IniValue<'_>, table: &mut DynTableMut<'_>) {
        use IniValue::*;

        match val {
            Bool(val) => table.set(key, Value::Bool(val)).unwrap(),
            I64(val) => table.set(key, Value::I64(val)).unwrap(),
            F64(val) => table.set(key, Value::F64(val)).unwrap(),
            String(val) => table.set(key, Value::String(val.into())).unwrap(),
        }
    }

    /// Returns `Ok(true)` if we need to skip the current value;
    /// else returns `Ok(false)`.
    fn check_is_key_duplicate(
        &self,
        root: &mut DynTableMut<'_>,
        section: &str,
        key: &str,
        skip: bool,
    ) -> Result<bool, IniError> {
        use IniErrorKind::*;

        debug_assert!(!key.is_empty());

        if skip {
            return Ok(false);
        }

        let is_unique = if section.is_empty() {
            root.get(key).is_err()
        } else {
            root.get(section)
                .unwrap()
                .table()
                .unwrap()
                .get(key)
                .is_err()
        };

        match self.options.duplicate_keys {
            IniDuplicateKeys::Forbid => {
                if is_unique {
                    Ok(false)
                } else {
                    Err(self.error_offset(DuplicateKey))
                }
            },
            // If `is_unique == true`, it's the first key and we must process it -> return `false` (don't skip).
            IniDuplicateKeys::First => Ok(!is_unique),
            // Never skip keys when we're interested in the last one.
            IniDuplicateKeys::Last => Ok(false),
        }
    }

    fn is_key_or_value_char(&self, val: char, quote: Option<char>) -> bool {
        if let Some(quote) = quote {
            debug_assert!(quote == '"' || quote == '\'');
        }

        match val {
            // Escape char must be escaped if escape sequences are supported.
            '\\' if self.options.escape => false,

            // Non-matching quotes don't need to be escaped in quoted strings.
            '"' => quote == Some('\''),
            '\'' => quote == Some('"'),

            // Space and special INI characters in key/value/section strings
            // (except string quotes, handled above) don't need to be escaped in quoted strings.
            ' ' | '[' | ']' | '=' | ':' | ';' | '#' => quote.is_some(),

            val => (val.is_alphanumeric() || val.is_ascii_punctuation()),
        }
    }
}

pub(crate) fn dyn_config_from_ini(
    string: &str,
    options: IniOptions,
) -> Result<DynConfig, IniError> {
    let reader = string.chars();
    let parser = IniParser::new(reader, options);

    Ok(parser.parse()?)
}

enum ParseEscapeSequenceResult {
    // Parsed an escape sequence as a valid char.
    EscapedChar(char),
    // Parsed an escape sequence as a line continuation.
    LineContinuation,
}

enum IniValue<'s> {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(&'s str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_key_or_value_char() {
        let parser = IniParser::new("".chars(), Default::default());

        for c in (b'0'..b'9').map(|c| char::from(c)) {
            assert!(parser.is_key_or_value_char(c, None));
        }

        for c in (b'a'..b'z').map(|c| char::from(c)) {
            assert!(parser.is_key_or_value_char(c, None));
        }

        for c in (b'A'..b'Z').map(|c| char::from(c)) {
            assert!(parser.is_key_or_value_char(c, None));
        }

        assert!(!parser.is_key_or_value_char('"', None));
        assert!(parser.is_key_or_value_char('"', Some('\'')));

        assert!(!parser.is_key_or_value_char('\'', None));
        assert!(parser.is_key_or_value_char('\'', Some('"')));

        let assert_ini_char = |c| {
            assert!(!parser.is_key_or_value_char(c, None));
            assert!(parser.is_key_or_value_char(c, Some('"')));
            assert!(parser.is_key_or_value_char(c, Some('\'')));
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
