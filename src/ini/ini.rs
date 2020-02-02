use std::str::Chars;

use crate::{DynConfig, IniError, IniErrorKind, Value, DynTable, DynTableMut, IniOptions, IniKeyValueSeparator, IniCommentSeparator, IniStringQuote};

/// INI parser FSM states.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum IniParserState {
    /// Accept whitespace (including new lines),
    /// section start delimiters ('[') (-> Section),
    /// valid key chars (-> Key),
    /// comment delimiters (';' / '#') (if supported) (-> SkipLine).
    StartLine,
    /// Accept valid key chars,
    /// section end delimiters (']') (-> SkipLineWhitespaceOrComments).
    Section,
    /// Accept new lines (-> StartLine),
    /// everything else.
    SkipLine,
    /// Accept new lines (-> StartLine),
    /// whitespace,
    /// comment start delimiters (';' / '#') (if supported) (-> SkipLine).
    SkipLineWhitespaceOrComments,
    /// Accept valid key chars,
    /// key-value separators ('=' / ':') (-> BeforeValue),
    /// whitespace (except new lines) (-> KeyValueSeparator).
    Key,
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
    /// escape sequneces (if supported),
    /// whitespace (-> SkipLineWhitespaceOrComments) (including new lines (-> StartLine)),
    /// inline comment delimiters (';' / '#') (if supported) (-> SkipLine).
    Value,
    /// Accept whitespace (except new lines),
    /// valid value chars,
    /// matching string quotes ('"' / '\'') (-> SkipLineWhitespaceOrComments),
    /// whitespace (except new lines).
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

        // Scratch buffer for unicode escape sequneces, if supported.
        let mut unicode_buffer = if self.options.escape {
            String::with_capacity(4)
        } else {
            String::new()
        };

        // Read the chars until EOF, process according to current state.
        while let Some(current) = self.next() {
            match self.state {
                IniParserState::StartLine => {
                    // Skip whitespace at the start of the line (including new lines).
                    if current.is_whitespace() {

                    // Section start - parse the section name.
                    } else if current == '[' {
                        self.state = IniParserState::Section;

                    // Valid key start - parse the key.
                    } else if is_key_char(current, true) {
                        debug_assert!(buffer.is_empty());
                        buffer.push(current);

                        self.state = IniParserState::Key;

                    // Line comment - skip the rest of the line.
                    } else if self.is_comment_char(current) {
                        self.state = IniParserState::SkipLine;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterAtLineStart));
                    }
                },
                IniParserState::Section => {
                    // Valid section name char (same rules as key chars) - keep parsing the section name.
                    if is_key_char(current, buffer.is_empty()) {
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

                        // Add the section to the config, if it does not exist already.
                        if root.get(section.as_str()).is_err() {
                            root.set(section.as_str(), Value::Table(DynTable::new())).unwrap();
                        }

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInSectionName));
                    }
                },
                IniParserState::SkipLine => {
                    debug_assert!(buffer.is_empty());

                    // If it's a new line, start parsing the next line.
                    // Skip everything else.
                    if current == '\n' {
                        self.state = IniParserState::StartLine;
                    }
                },
                IniParserState::SkipLineWhitespaceOrComments => {
                    debug_assert!(buffer.is_empty());

                    // If it's a new line, start parsing the next line.
                    if current == '\n' {
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
                },
                IniParserState::Key => {
                    // We have at least one key character already parsed.
                    debug_assert!(!buffer.is_empty());

                    // Key-value separator - finish the key, parse the value.
                    if self.is_key_value_separator_char(current) {
                        key.clear();
                        key.push_str(&buffer);
                        buffer.clear();

                        self.is_key_duplicate(&mut root, &section, &key)?;

                        self.state = IniParserState::BeforeValue;

                    // Whitespace between the key and the separator - skip it, finish the key, parse the separator.
                    } else if current.is_whitespace() {
                        // Unless it's a new line.
                        if current == '\n' {
                            return Err(self.error_offset(UnexpectedNewlineInKey));
                        }

                        key.clear();
                        key.push_str(&buffer);
                        buffer.clear();

                        self.is_key_duplicate(&mut root, &section, &key)?;

                        self.state = IniParserState::KeyValueSeparator;

                    // Valid key char - keep parsing the key.
                    } else if is_key_char(current, buffer.is_empty()) {
                        buffer.push(current);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInKey));
                    }
                },
                IniParserState::KeyValueSeparator => {
                    debug_assert!(buffer.is_empty());

                    // Key-value separator - parse the value (key already finished).
                    if self.is_key_value_separator_char(current) {
                        self.state = IniParserState::BeforeValue;

                    // Skip the whitespace between the key and the separator.
                    } else if current.is_whitespace() {
                        // Unless it's a new line.
                        if current == '\n' {
                            return Err(self.error_offset(UnexpectedNewlineInKey));
                        }

                    // Else an error.
                    } else {
                        return Err(self.error(UnexpectedCharacterInsteadOfKeyValueSeparator));
                    }
                },
                IniParserState::BeforeValue => {
                    debug_assert!(buffer.is_empty());

                    // Skip the whitespace before the value.
                    if current.is_whitespace() {
                        // Unless it's a new line - the value is empty.
                        if current == '\n' {
                            self.add_value(&mut root, &section, &key, "", false)?;
                            key.clear();

                            self.state = IniParserState::StartLine;
                        }

                    // Inline comment (if supported) - the value is empty, skip the rest of the line.
                    } else if self.options.inline_comments && self.is_comment_char(current) {
                        self.add_value(&mut root, &section, &key, "", false)?;
                        key.clear();

                        self.state = IniParserState::SkipLine;

                    // String quote - parse the string value in quotes, expecting the matching quotes.
                    } else if self.is_string_quote_char(current) {
                        debug_assert!(quote.is_none());
                        quote.replace(current);

                        self.state = IniParserState::QuotedString;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - start parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                debug_assert!(buffer.is_empty());
                                buffer.push(current);

                                self.state = IniParserState::Value;
                            },
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {},
                        }

                    // Valid value char - start parsing the unquoted value.
                    } else if is_value_char(current) {
                        debug_assert!(buffer.is_empty());
                        buffer.push(current);

                        self.state = IniParserState::Value;

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue));
                    }
                },
                IniParserState::Value => {
                    debug_assert!(!buffer.is_empty());

                    // New line - finish the value, start the new line.
                    if current == '\n' {
                        self.add_value(&mut root, &section, &key, &buffer, false)?;
                        buffer.clear();
                        key.clear();

                        self.state = IniParserState::StartLine;

                    // Whitespace - finish the value, skip the rest of the line.
                    } else if current.is_whitespace() {
                        self.add_value(&mut root, &section, &key, &buffer, false)?;
                        buffer.clear();
                        key.clear();

                        self.state = IniParserState::SkipLineWhitespaceOrComments;

                    // Inline comment (if supported) - finish the value, skip the rest of the line.
                    } else if self.options.inline_comments && self.is_comment_char(current) {
                        self.add_value(&mut root, &section, &key, &buffer, false)?;
                        buffer.clear();
                        key.clear();

                        self.state = IniParserState::SkipLine;

                    // Escaped char (if supported) - parse the escape sequence.
                    } else if self.is_escape_char(current) {
                        match self.parse_escape_sequence(&mut unicode_buffer)? {
                            // Parsed an escaped char - keep parsing the value.
                            ParseEscapeSequenceResult::EscapedChar(current) => {
                                buffer.push(current);
                            },
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {},
                        }

                    // Valid value char - keep parsing the value.
                    } else if is_value_char(current) {
                        buffer.push(current);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue));
                    }
                },
                IniParserState::QuotedString => {
                    debug_assert!(quote.is_some());

                    // New line before the closing quotes - error.
                    if current == '\n' {
                        return Err(self.error_offset(UnexpectedNewlineInQuotedString));

                    // Closing quotes - finish the value, skip the rest of the line.
                    } else if current == *quote.as_ref().unwrap() {
                        self.add_value(&mut root, &section, &key, &buffer, true)?;
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
                            },
                            // Line continuation - keep parsing.
                            ParseEscapeSequenceResult::LineContinuation => {},
                        }

                    // Whitespace or valid value char - keep parsing the value.
                    } else if current.is_whitespace() || is_value_char(current) {
                        buffer.push(current);

                    // Else an error.
                    } else {
                        return Err(self.error(InvalidCharacterInValue));
                    }
                },
            }
        }

        // Add the last value if we were parsing it right before EOF.
        if self.state == IniParserState::Value {
            self.add_value(&mut root, &section, &key, &buffer, quote.is_some())?;
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
            Some('\n') => {
                self.column += 1;
                self.new_line = true;
            },
            Some(_) => {
                self.column += 1;
            },
            None => {},
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
        ((val == ';') && self.options.comments.contains(IniCommentSeparator::Semicolon)) ||
        ((val == '#') && self.options.comments.contains(IniCommentSeparator::NumberSign))
    }

    /// Is the character a supported key-value separator?
    fn is_key_value_separator_char(&self, val: char) -> bool {
        ((val == '=') && self.options.key_value_separator.contains(IniKeyValueSeparator::Equals)) ||
        ((val == ':') && self.options.key_value_separator.contains(IniKeyValueSeparator::Colon))
    }

    /// Is the character a supported string quote?
    fn is_string_quote_char(&self, val: char) -> bool {
        ((val == '"') && self.options.string_quotes.contains(IniStringQuote::Double)) ||
        ((val == '\'') && self.options.string_quotes.contains(IniStringQuote::Single))
    }

    /// Is the character a supported escape character?
    fn is_escape_char(&self, val: char) -> bool {
        self.options.escape && (val == '\\')
    }

    /// Reads up to 4 following characters and tries to parses them as an escape sequence.
    fn parse_escape_sequence(&mut self, unicode_buffer: &mut String) -> Result<ParseEscapeSequenceResult, IniError> {
        use ParseEscapeSequenceResult::*;
        use IniErrorKind::*;

        debug_assert!(self.options.escape);

        match self.next() {
            None => Err(self.error(UnexpectedEndOfFileInEscapeSequence)),

            // Backslash followed by a new line is a line continuation, if supported.
            Some('\n') => if self.options.line_continuation {
                Ok(LineContinuation)
            } else {
                Err(self.error_offset(UnexpectedNewlineInEscapeSequence))
            },

            // Standard escaped characters.
            Some('0') => Ok(EscapedChar('\0')),
            Some('a') => Ok(EscapedChar('\x07')),
            Some('b') => Ok(EscapedChar('\x08')),
            Some('t') => Ok(EscapedChar('\t')),
            Some('r') => Ok(EscapedChar('\r')),
            Some('n') => Ok(EscapedChar('\n')),

            // Escaped INI special characters, disallowed otherwise.
            Some('\\') => Ok(EscapedChar('\\')),
            Some('[') => Ok(EscapedChar('[')),
            Some(']') => Ok(EscapedChar(']')),
            Some(';') => Ok(EscapedChar(';')),
            Some('#') => Ok(EscapedChar('#')),
            Some('=') => Ok(EscapedChar('=')),
            Some(':') => Ok(EscapedChar(':')),

            // 4 hexadecimal Unicode values.
            Some('x') => {
                unicode_buffer.clear();

                for _ in 0 .. 4 {
                    match self.next() {
                        None => return Err(self.error(UnexpectedEndOfFileInUnicodeEscapeSequence)),
                        Some('\n') => return Err(self.error_offset(UnexpectedNewlineInUnicodeEscapeSequence)),
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
            },

            Some(_) => Err(self.error(InvalidEscapeCharacter)),
        }
    }

    /// Parses a string `value` and adds it to the config `section` at `key`.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn add_value(&self, root: &mut DynTableMut<'_>, section: &str, key: &str, value: &str, quoted: bool) -> Result<(), IniError> {
        debug_assert!(!key.is_empty());

        let value = self.parse_value_string(value, quoted)?;

        if section.is_empty() {
            debug_assert!(self.options.duplicate_keys || root.get(key).is_err());
            Self::add_value_to_table(key, value, root);

        } else {
            // Must succeed.
            let mut table = root.get_mut(section).unwrap().table().unwrap();

            debug_assert!(self.options.duplicate_keys || table.get(key).is_err());
            Self::add_value_to_table(key, value, &mut table);
        }

        Ok(())
    }

    /// Parses a string `value`.
    /// If `quoted` is `true`, `value` is always treated as a string,
    /// else it is first interpreted as a bool / integer / float.
    /// Empty `value`'s are treated as strings.
    fn parse_value_string<'v>(&self, value: &'v str, quoted: bool) -> Result<IniValue<'v>, IniError> {
        use IniValue::*;
        use IniErrorKind::*;

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
            String(val) => table.set(key, Value::String(val)).unwrap(),
        }
    }

    fn is_key_duplicate(&self, root: &mut DynTableMut<'_>, section: &str, key: &str) -> Result<(), IniError> {
        use IniErrorKind::*;

        debug_assert!(!key.is_empty());

        let is_unique = if section.is_empty() {
            root.get(key).is_err()
        } else  {
            root.get(section).unwrap().table().unwrap().get(key).is_err()
        };

        if is_unique || self.options.duplicate_keys {
            Ok(())
        } else {
            Err(self.error_offset(DuplicateKey))
        }
    }
}

pub(crate) fn dyn_config_from_ini(string: &str, options: IniOptions) -> Result<DynConfig, IniError> {
    let reader = string.chars();
    let parser = IniParser::new(reader, options);

    Ok(parser.parse()?)
}

fn is_key_char(val: char, first: bool) -> bool {
    (val == '_') || val.is_alphabetic() || (!first && val.is_numeric())
}

fn is_value_char(val: char) -> bool {
    (val.is_alphanumeric() || val.is_ascii_punctuation()) &&
    (val != '\'') &&
    (val != '"') &&
    (val != '\\') &&
    (val != '[') &&
    (val != ']') &&
    (val != ';') &&
    (val != '#') &&
    (val != '=') &&
    (val != ':')
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