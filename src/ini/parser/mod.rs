mod fsm_state;

use {
    super::*,
    crate::*,
    fsm_state::*,
    std::{iter::Iterator, ops::RangeInclusive, str::CharIndices},
};

//////////////////////////////////////////////////////////
/// Trait alias for a closure which, given an inclusive byte range in the `.ini` source string,
/// returns the non-empty substring corresponding to the byte range.
/// The caller guarantees the byte range is valid and corresponds to a valid UTF-8 substring.
pub(crate) trait Substr<'s>: Fn(RangeInclusive<usize>) -> &'s NonEmptyStr {}

impl<'s, F> Substr<'s> for F where F: Fn(RangeInclusive<usize>) -> &'s NonEmptyStr {}
//////////////////////////////////////////////////////////

//////////////////////////////////////////////////////////
/// Trait alias for a closure which returns the next character, if any, in the `.ini` source string.
trait NextChar: FnMut() -> Option<char> {}

impl<F> NextChar for F where F: FnMut() -> Option<char> {}
//////////////////////////////////////////////////////////

/// Persistent state used to communicate information between parser FSM states.
pub(super) struct IniParserPersistentState<'s> {
    // Scratch buffer for parsed section names / keys, and the current key, if any.
    pub key: ParsedIniKey,
    // Scratch buffer for parsed values.
    pub value: ParsedIniValue,
    // Current nested section path, if any.
    // Contains at most one section name if nested sections are not supported.
    pub path: IniPath<'s>,
    // Whether the key is unique in its table (root or section).
    pub is_key_unique: bool,
    // Whether we need to skip all key/value pairs in the current section
    // (i.e., when we encountered a duplicate section instance and we use the `First` duplicate section policy).
    pub skip_section: bool,
    // Whether we need to skip the current value
    // (i.e., when we encountered a duplicate key and we use the `First` duplicate key policy).
    pub skip_value: bool,
}

impl<'s> IniParserPersistentState<'s> {
    fn new() -> Self {
        Self {
            key: ParsedIniKey::new(),
            value: ParsedIniValue::new(),
            path: IniPath::new(),
            is_key_unique: true,
            skip_section: false,
            skip_value: false,
        }
    }

    fn clear_path<C: IniConfig<'s>>(&mut self, config: &mut C) {
        while let Some(section) = self.path.last() {
            // We didn't call `start_section()` if we skipped it, so don't call `end_section`.
            if !self.skip_section {
                config.end_section(section);
            } else {
                self.skip_section = false;
            }
            self.path.pop();
        }
    }
}

/// Current position in the source string.
/// Used for error reporting.
struct IniParserSrcPositionState {
    line: u32,
    column: u32,
    new_line: bool,
    // Set to `true` in order to consume a `\n` following a `\r` as a single newline.
    cr: bool,
}

impl IniParserSrcPositionState {
    fn new() -> Self {
        Self {
            line: 1,
            column: 0,
            new_line: false,
            cr: false,
        }
    }
}

/// Parses the `.ini` config string, using the user-provided [`parsing options`](struct.IniOptions.html)
/// and the [`event handler`](trait.IniConfig.html) object.
pub struct IniParser<'s> {
    /// Source `.ini` string.
    source: &'s str,
    /// Source string reader.
    reader: CharIndices<'s>,
    /// Parsing options as provided by the user.
    options: IniOptions,
}

impl<'s> IniParser<'s> {
    /// Creates a new [`parser`](struct.IniParser.html) from the `.ini` config `string`
    /// using default [`parsing options`](struct.IniOptions.html).
    pub fn new(string: &'s str) -> Self {
        Self {
            source: string,
            reader: string.char_indices(),
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

    /// Sets whether escape sequences (a character sequence following a backslash (`'\'`))
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
    /// If `false`, backslash (`'\'`) is treated as a normal section name / key / value character.
    ///
    /// Default: `true`.
    pub fn escape(mut self, escape: bool) -> Self {
        self.options.escape = escape;
        self
    }

    /// Sets whether line ontinuation esacpe sequences (a backslash `'\'` followed by a newline `'\n'` / `'\r'`)
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

    /// Whether implicit parent sections are allowed.
    /// If `nested_section_depth` is `>1` (we allow nested sections), and this is `true`,
    /// using section names in nested section paths which have not been declared prior
    /// is allowed and results in an implicit empty section with that name being declared.
    /// Otherwise using an unknown section name in a nested section path is treated as an error.
    ///
    /// Default: `false`.
    pub fn implicit_parent_sections(mut self, implicit_parent_sections: bool) -> Self {
        self.options.implicit_parent_sections = implicit_parent_sections;
        self
    }

    /// Consumes the parser and tries to parse the `.ini` config string, calling the methods on the passed `config` event handler.
    pub fn parse<C: IniConfig<'s>>(mut self, config: &mut C) -> Result<(), IniError> {
        self.validate_options();

        let options = self.options;

        let reader = &mut self.reader;
        let source = &self.source;

        let substr = |range| Self::substr(source, range);

        let mut persistent_state = IniParserPersistentState::new();
        let mut src_pos_state = IniParserSrcPositionState::new();
        let mut fsm_state = IniParserFSMState::StartLine;

        // Read the chars until EOF, process according to current state.
        while let Some((c, idx)) = Self::next(reader, &mut src_pos_state) {
            fsm_state = fsm_state
                .process(
                    c,
                    idx,
                    || Self::next(reader, &mut src_pos_state).map(|(c, _)| c),
                    substr,
                    config,
                    &mut persistent_state,
                    &options,
                )
                .map_err(|(err, offset)| {
                    Self::error(
                        err,
                        offset,
                        &src_pos_state,
                        persistent_state.path.to_config_path(),
                    )
                })?;
        }

        fsm_state
            .finish(substr, config, &mut persistent_state, &options)
            .map_err(|err| {
                Self::error(
                    err,
                    false,
                    &src_pos_state,
                    persistent_state.path.to_config_path(),
                )
            })?;

        persistent_state.clear_path(config);

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
    fn next(
        reader: &mut CharIndices<'s>,
        state: &mut IniParserSrcPositionState,
    ) -> Option<(char, usize)> {
        let next = reader.next();

        if state.new_line {
            state.line += 1;
            state.column = 0;

            state.new_line = false;
        }

        match next {
            Some((idx, c)) => {
                match c {
                    // Eat a line feed if the previous char was a carriage return.
                    '\n' if state.cr => {
                        state.cr = false;
                    }
                    '\r' => {
                        state.column += 1;
                        state.new_line = true;

                        state.cr = true;
                    }
                    '\n' => {
                        state.column += 1;
                        state.new_line = true;
                    }
                    _ => {
                        state.column += 1;
                    }
                }

                Some((c, idx))
            }
            None => None,
        }
    }

    fn substr(src: &'s str, idx: RangeInclusive<usize>) -> &'s NonEmptyStr {
        debug_assert!(idx.end() >= idx.start());
        debug_assert!(*idx.start() < src.len());
        debug_assert!(*idx.end() <= src.len());

        unsafe { unwrap_unchecked(NonEmptyStr::new(src.get_unchecked(idx)), "empty substring") }
    }

    /// Error helper method.
    fn error(
        error: IniErrorKind,
        offset: bool,
        state: &IniParserSrcPositionState,
        path: ConfigPath,
    ) -> IniError {
        if offset {
            debug_assert!(state.column > 0);
        }

        IniError {
            line: state.line,
            column: if offset {
                state.column - 1
            } else {
                state.column
            },
            error,
            path,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ParseEscapeSequenceResult {
    /// Parsed an escape sequence as a valid char.
    EscapedChar(char),
    /// Parsed an escape sequence as a line continuation.
    LineContinuation,
}
