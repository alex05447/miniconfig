#![allow(non_snake_case)]

use crate::*;

#[test]
fn InvalidCharacterAtLineStart() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("'")).err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::InvalidCharacterAtLineStart('\'')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new(":")).err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::InvalidCharacterAtLineStart(':')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new(" #")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterAtLineStart('#')
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("\"a\"=")).unwrap(); // Quoted key, empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini =
        DynConfig::from_ini(IniParser::new("' a'=").string_quotes(IniStringQuote::Single)).unwrap(); // Quoted key, empty value.
    assert_eq!(ini.root().get_string(" a").unwrap(), "");

    let ini = DynConfig::from_ini(IniParser::new("\\==")).unwrap(); // Escaped special character in key, empty value.
    assert_eq!(ini.root().get_string("=").unwrap(), "");

    let ini = DynConfig::from_ini(IniParser::new("a=")).unwrap(); // Valid key, empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = DynConfig::from_ini(IniParser::new("`~@$%^&*()_-+,<.>/? =")).unwrap(); // Weird key.
    assert_eq!(ini.root().get_string("`~@$%^&*()_-+,<.>/?").unwrap(), "");

    let ini =
        DynConfig::from_ini(IniParser::new("a:").key_value_separator(IniKeyValueSeparator::Colon))
            .unwrap(); // Valid key, empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = DynConfig::from_ini(IniParser::new("[a]")).unwrap(); // Section
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new(";")).unwrap(); // Comment
    assert_eq!(ini.root().len(), 0);

    let ini =
        DynConfig::from_ini(IniParser::new("#").comments(IniCommentSeparator::NumberSign)).unwrap(); // Comment
    assert_eq!(ini.root().len(), 0);
}

#[test]
fn InvalidCharacterInSectionName() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[=")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterInSectionName('=')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[:").key_value_separator(IniKeyValueSeparator::Colon),)
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterInSectionName(':')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a#")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInSectionName('#')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a;").comments(IniCommentSeparator::NumberSign),)
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInSectionName(';')
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("[ \ta]")).unwrap(); // Skipped whitespace before section name.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[a \t]")).unwrap(); // Skipped whitespace after section name.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[\\=]")).unwrap(); // Special character in section.
    assert_eq!(ini.root().get_table("=").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[\\:]")).unwrap(); // Special character in section.
    assert_eq!(ini.root().get_table(":").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[a]")).unwrap(); // Normal section.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    DynConfig::from_ini(IniParser::new("[`~@$%^&*()_-+,<.>/?]")).unwrap(); // Weird section.

    let ini = DynConfig::from_ini(IniParser::new("[\\t\\ \\n]")).unwrap(); // Escaped whitespace in section.
    assert_eq!(ini.root().get_table("\t \n").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[\\x0066\\x006f\\x006f]")).unwrap(); // Unicode in section ("foo").
    assert_eq!(ini.root().get_table("foo").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[\t\"a \" ]")).unwrap(); // Whitespace in quoted section name.
    assert_eq!(ini.root().get_table("a ").unwrap().len(), 0);

    let ini = DynConfig::from_ini(
        IniParser::new("[\t\"' a'\" ]")
            .string_quotes(IniStringQuote::Single | IniStringQuote::Double),
    )
    .unwrap(); // Non-matching quotes in quoted section name.
    assert_eq!(ini.root().get_table("' a'").unwrap().len(), 0);

    let ini =
        DynConfig::from_ini(IniParser::new("[\t'a ' ]").string_quotes(IniStringQuote::Single))
            .unwrap(); // Whitespace in quoted section name.
    assert_eq!(ini.root().get_table("a ").unwrap().len(), 0);

    let ini = DynConfig::from_ini(
        IniParser::new("[\t'\" a\"' ]")
            .string_quotes(IniStringQuote::Single | IniStringQuote::Double),
    )
    .unwrap(); // Non-matching quotes in quoted section name.
    assert_eq!(ini.root().get_table("\" a\"").unwrap().len(), 0);
}

#[test]
fn InvalidCharacterAfterSectionName() {
    // Any character after whitespace after section name.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a b]")).err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidCharacterAfterSectionName('b')
        }
    );

    // Any character after closing quotes.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[\"a\" b]"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 6,
            error: IniErrorKind::InvalidCharacterAfterSectionName('b')
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("[\ta\\ b ]")).unwrap(); // Escaped whitespace in unquoted section name.
    assert_eq!(ini.root().get_table("a b").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[\t\"a b\" ]")).unwrap(); // Unescaped whitespace in quoted section name.
    assert_eq!(ini.root().get_table("a b").unwrap().len(), 0);
}

#[test]
fn UnexpectedNewLineInSectionName() {
    // Unescaped new line at section name start.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[\n")).err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedNewLineInSectionName
        }
    );
    // Unescaped new line in section name.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a\n")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInSectionName
        }
    );
    // Unescaped new line in quoted section name.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[\"a\n")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInSectionName
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("[\\n]")).unwrap(); // Escaped new line at section name start.
    assert_eq!(ini.root().get_table("\n").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[a\\n]")).unwrap(); // Escaped new line in section name.
    assert_eq!(ini.root().get_table("a\n").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[ \"a\\n\" ]")).unwrap(); // Escaped new line in quoted section name.
    assert_eq!(ini.root().get_table("a\n").unwrap().len(), 0);
}

#[test]
fn UnexpectedEndOfFileInSectionName() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileInSectionName
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[\"a")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInSectionName
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[\"a\"")).err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedEndOfFileInSectionName
        }
    );
}

#[test]
fn EmptySectionName() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[]")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::EmptySectionName
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[\"\"]")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::EmptySectionName
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("[\\ ]")).unwrap(); // Whitespace in section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[\\t]")).unwrap(); // Whitespace in section.
    assert_eq!(ini.root().get_table("\t").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[\\n]")).unwrap(); // Whitespace in section.
    assert_eq!(ini.root().get_table("\n").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[\t\" \" ]")).unwrap(); // Whitespace in quoted section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);
}

#[test]
fn DuplicateSection() {
    // Duplicate sections not supported.
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new("[a]\n[b]\n[a]").duplicate_sections(IniDuplicateSections::Forbid),
        )
        .err()
        .unwrap(),
        IniError {
            line: 3,
            column: 3,
            error: IniErrorKind::DuplicateSection("a".into())
        }
    );

    // But this succeeds.

    // Use the `First` section. Second section skipped, including duplicate keys.
    let ini = DynConfig::from_ini(
        IniParser::new("[a]\na=7\n[a]\na=9\n[b]\na=42")
            .duplicate_sections(IniDuplicateSections::First),
    )
    .unwrap();
    assert_eq!(ini.root().len(), 2);
    assert_eq!(ini.root().get_table("a").unwrap().len(), 1);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("a").unwrap(), 7);
    assert_eq!(ini.root().get_table("b").unwrap().len(), 1);
    assert_eq!(ini.root().get_table("b").unwrap().get_i64("a").unwrap(), 42);

    // Use the `Last` section. First section overwritten.
    let ini = DynConfig::from_ini(
        IniParser::new("[a]\na=7\n[a]\na=9\n[b]\na=42")
            .duplicate_sections(IniDuplicateSections::Last),
    )
    .unwrap();
    assert_eq!(ini.root().len(), 2);
    assert_eq!(ini.root().get_table("a").unwrap().len(), 1);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("a").unwrap(), 9);

    // `Merge` sections, duplicate keys.
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new("[a]\na=7\n[a]\na=9\n[b]\na=42")
                .duplicate_sections(IniDuplicateSections::Merge),
        )
        .err()
        .unwrap(),
        IniError {
            line: 4,
            column: 1,
            error: IniErrorKind::DuplicateKey("a".into())
        }
    );

    // `Merge` sections, unique keys.
    let ini = DynConfig::from_ini(
        IniParser::new("[a]\na=7\n[b]\na=42\n[a]\nb=9\n[b]\nb=43")
            .duplicate_sections(IniDuplicateSections::Merge),
    )
    .unwrap();
    assert_eq!(ini.root().len(), 2);
    assert_eq!(ini.root().get_table("a").unwrap().len(), 2);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("a").unwrap(), 7);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("b").unwrap(), 9);
    assert_eq!(ini.root().get_table("b").unwrap().len(), 2);
    assert_eq!(ini.root().get_table("b").unwrap().get_i64("a").unwrap(), 42);
    assert_eq!(ini.root().get_table("b").unwrap().get_i64("b").unwrap(), 43);
}

#[test]
fn InvalidCharacterAtLineEnd() {
    // After section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a] b")).err().unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAtLineEnd('b')
        }
    );
    // After value.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=7 b")).err().unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAtLineEnd('b')
        }
    );
    // Inline comments not supported.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a] ;")).err().unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAtLineEnd(';')
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("[a] ;").inline_comments(true)).unwrap(); // Supported inline comment.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = DynConfig::from_ini(
        IniParser::new("[a] #")
            .comments(IniCommentSeparator::NumberSign)
            .inline_comments(true),
    )
    .unwrap(); // Supported inline comment.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);
}

#[test]
fn InvalidCharacterInKey() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a[")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterInKey('[')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new(" a'")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInKey('\'')
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a=true")).unwrap(); // Normal key.
    assert_eq!(ini.root().get_bool("a").unwrap(), true);

    let ini = DynConfig::from_ini(IniParser::new("\"a\"=true")).unwrap(); // Quoted key.
    assert_eq!(ini.root().get_bool("a").unwrap(), true);

    let ini =
        DynConfig::from_ini(IniParser::new("' a ' = false").string_quotes(IniStringQuote::Single))
            .unwrap(); // Quoted key.
    assert_eq!(ini.root().get_bool(" a ").unwrap(), false);

    let ini = DynConfig::from_ini(IniParser::new("a\\[=7")).unwrap(); // Special character in key.
    assert_eq!(ini.root().get_i64("a[").unwrap(), 7);

    let ini = DynConfig::from_ini(IniParser::new("a\\t=3.14")).unwrap(); // Escaped whitespace in key.
    assert!(cmp_f64(ini.root().get_f64("a\t").unwrap(), 3.14));

    let ini = DynConfig::from_ini(IniParser::new("\\x0066\\x006f\\x006f=\"bar\"")).unwrap(); // Unicode in key ("foo").
    assert_eq!(ini.root().get_string("foo").unwrap(), "bar");
}

#[test]
fn UnexpectedNewLineInKey() {
    // New line in key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a\n")).err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedNewLineInKey
        }
    );
    // New line in quoted key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("\"a\n\""))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInKey
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a\\n=7")).unwrap(); // Escaped new line in key.
    assert_eq!(ini.root().get_i64("a\n").unwrap(), 7);

    let ini = DynConfig::from_ini(IniParser::new("a=\n")).unwrap(); // Empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");
}

#[test]
fn EmptyKey() {
    // Empty unquoted key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new(" = 7")).err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::EmptyKey
        }
    );
    // Empty unquoted key.
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new(" : 7").key_value_separator(IniKeyValueSeparator::Colon),
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::EmptyKey
        }
    );
    // Empty quoted key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new(" \"\" = 7"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::EmptyKey
        }
    );
    // Empty quoted key.
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new(" '' : 7").key_value_separator(IniKeyValueSeparator::Colon).string_quotes(IniStringQuote::Single),
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::EmptyKey
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a = 7")).unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 7);

    let ini = DynConfig::from_ini(IniParser::new("[a]\n\" \" = 7")).unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().get_i64(" ").unwrap(), 7);
}

#[test]
fn DuplicateKey() {
    // In the root.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=7\nb=8\na=9\nc=10"))
            .err()
            .unwrap(),
        IniError {
            line: 3,
            column: 1,
            error: IniErrorKind::DuplicateKey("a".into())
        }
    );
    // In the section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a]\na=7\nb=8\na=9\nc=10"))
            .err()
            .unwrap(),
        IniError {
            line: 4,
            column: 1,
            error: IniErrorKind::DuplicateKey("a".into())
        }
    );
    // In the merged section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a]\na=7\nb=8\n[a]\na=9\nc=10"))
            .err()
            .unwrap(),
        IniError {
            line: 5,
            column: 1,
            error: IniErrorKind::DuplicateKey("a".into())
        }
    );

    // But this succeeds.

    // In the root, `First`.
    let ini = DynConfig::from_ini(
        IniParser::new("a=7\nb=8\na=9\nc=10").duplicate_keys(IniDuplicateKeys::First),
    )
    .unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 7);
    assert_eq!(ini.root().get_i64("b").unwrap(), 8);
    assert_eq!(ini.root().get_i64("c").unwrap(), 10);

    // In the root, `Last`.
    let ini = DynConfig::from_ini(
        IniParser::new("a=7\nb=8\na=9\nc=10").duplicate_keys(IniDuplicateKeys::Last),
    )
    .unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 9);
    assert_eq!(ini.root().get_i64("b").unwrap(), 8);
    assert_eq!(ini.root().get_i64("c").unwrap(), 10);

    // In the section, `First`.
    let ini = DynConfig::from_ini(
        IniParser::new("[a]\na=7\nb=8\na=9\nc=10").duplicate_keys(IniDuplicateKeys::First),
    )
    .unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("a").unwrap(), 7);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("b").unwrap(), 8);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("c").unwrap(), 10);

    // In the section, `Last`.
    let ini = DynConfig::from_ini(
        IniParser::new("[a]\na=7\nb=8\na=9\nc=10").duplicate_keys(IniDuplicateKeys::Last),
    )
    .unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("a").unwrap(), 9);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("b").unwrap(), 8);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("c").unwrap(), 10);

    // In the merged section, `First`.
    let ini = DynConfig::from_ini(
        IniParser::new("[a]\na=7\nb=8\n[a]\na=9\nc=10").duplicate_keys(IniDuplicateKeys::First),
    )
    .unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("a").unwrap(), 7);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("b").unwrap(), 8);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("c").unwrap(), 10);

    // In the merged section, `Last`.
    let ini = DynConfig::from_ini(
        IniParser::new("[a]\na=7\nb=8\n[a]\na=9\nc=10").duplicate_keys(IniDuplicateKeys::Last),
    )
    .unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("a").unwrap(), 9);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("b").unwrap(), 8);
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("c").unwrap(), 10);
}

#[test]
fn UnexpectedEndOfFileBeforeKeyValueSeparator() {
    // Unquoted key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a")).err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedEndOfFileBeforeKeyValueSeparator
        }
    );
    // Quoted key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("\"a \"")).err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedEndOfFileBeforeKeyValueSeparator
        }
    );

    // But this succeeds (empty value).

    let ini = DynConfig::from_ini(IniParser::new("a=")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = DynConfig::from_ini(
        IniParser::new("[a]\n\"a\":").key_value_separator(IniKeyValueSeparator::Colon),
    )
    .unwrap();
    assert_eq!(
        ini.root().get_table("a").unwrap().get_string("a").unwrap(),
        ""
    );
}

#[test]
fn InvalidKeyValueSeparator() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a !")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidKeyValueSeparator('!')
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a :")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidKeyValueSeparator(':')
        }
    );
    // Unescaped whitespace in key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a b = 7"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidKeyValueSeparator('b')
        }
    );
    // Unexpected character after quoted key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("\"a\" b = 7"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidKeyValueSeparator('b')
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a = 7")).unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 7);

    let ini = DynConfig::from_ini(
        IniParser::new("a : 7").key_value_separator(IniKeyValueSeparator::Colon),
    )
    .unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 7);
}

#[test]
fn InvalidCharacterInValue() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a==")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue('=')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=:")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue(':')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a:=").key_value_separator(IniKeyValueSeparator::Colon))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue('=')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new("a::").key_value_separator(IniKeyValueSeparator::Colon),
        ).err()
        .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue(':')
        }
    );
    // Inline comments not supported.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=a;")).err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidCharacterInValue(';')
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a=\\=")).unwrap(); // Escaped special character in value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini(
        IniParser::new("a:\\=").key_value_separator(IniKeyValueSeparator::Colon),
    )
    .unwrap(); // Escaped special character in value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini(IniParser::new("a=\"\\=\"")).unwrap(); // Escaped special character in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini(IniParser::new("a=\"=\"")).unwrap(); // Unescaped special character in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini(IniParser::new("a=\"'\"")).unwrap(); // Unmatched quote in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "'");

    let ini = DynConfig::from_ini(IniParser::new("a='\"'").string_quotes(IniStringQuote::Single))
        .unwrap(); // Unmatched quote in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "\"");

    let ini = DynConfig::from_ini(IniParser::new("a=a;").inline_comments(true)).unwrap(); // Supported inline comments.
    assert_eq!(ini.root().get_string("a").unwrap(), "a");

    let ini = DynConfig::from_ini(
        IniParser::new("a=a#")
            .comments(IniCommentSeparator::NumberSign)
            .inline_comments(true),
    )
    .unwrap(); // Supported inline comments.
    assert_eq!(ini.root().get_string("a").unwrap(), "a");

    let ini = DynConfig::from_ini(IniParser::new("foo=\\x0066\\x006f\\x006f")).unwrap(); // Unicode in value ("foo").
    assert_eq!(ini.root().get_string("foo").unwrap(), "foo");

    let ini = DynConfig::from_ini(IniParser::new("a=\" \"")).unwrap(); // Unescaped whitespace in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), " ");
}

#[test]
fn UnexpectedEndOfFileInEscapeSequence() {
    // In section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[\\")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In quoted section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[\"\\")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("\\")).err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In quoted key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("\"\\")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In unquoted value.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In quoted value.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\"\\")).err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("[\\ ]")).unwrap(); // Escaped space in section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("[ \"\\ \" ]")).unwrap(); // Escaped space in quoted section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);

    let ini = DynConfig::from_ini(IniParser::new("\\ =")).unwrap(); // Escaped space in key.
    assert_eq!(ini.root().get_string(" ").unwrap(), "");

    let ini = DynConfig::from_ini(IniParser::new("\"\\ \" =")).unwrap(); // Escaped space in quoted key.
    assert_eq!(ini.root().get_string(" ").unwrap(), "");

    let ini = DynConfig::from_ini(IniParser::new("a = \\ ")).unwrap(); // Escaped space in unquoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), " ");

    let ini = DynConfig::from_ini(IniParser::new("a = \"\\ \"")).unwrap(); // Escaped space in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), " ");
}

#[test]
fn UnexpectedNewLineInEscapeSequence() {
    // Unsupported line continuation in section name.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[\\\n")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInEscapeSequence
        }
    );
    // Unsupported line continuation in key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a\\\n")).err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInEscapeSequence
        }
    );
    // Unsupported line continuation in value.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\\n")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInEscapeSequence
        }
    );

    // But this succeeds (supported line continuation).

    // Line continuation in section name.
    let ini = DynConfig::from_ini(IniParser::new("[\\\na]").line_continuation(true)).unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    // Line continuation in key.
    let ini = DynConfig::from_ini(IniParser::new("a\\\nb = 7").line_continuation(true)).unwrap();
    assert_eq!(ini.root().get_i64("ab").unwrap(), 7);

    // Line continuation in value.
    let ini = DynConfig::from_ini(IniParser::new("a = 7\\\n9").line_continuation(true)).unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 79);
}

#[test]
fn InvalidEscapeCharacter() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\z")).err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidEscapeCharacter('z')
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a=\\ ")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), " ");

    let ini = DynConfig::from_ini(IniParser::new("a=\" \"")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), " ");

    let ini = DynConfig::from_ini(IniParser::new("a=\\0")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\0");

    let ini = DynConfig::from_ini(IniParser::new("a=\\a")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\x07"); // '\a'

    let ini = DynConfig::from_ini(IniParser::new("a=\\b")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\x08"); // '\a'

    let ini = DynConfig::from_ini(IniParser::new("a=\\t")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\t");

    let ini = DynConfig::from_ini(IniParser::new("a=\\n")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\n");

    let ini = DynConfig::from_ini(IniParser::new("a=\\r")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\r");

    let ini = DynConfig::from_ini(IniParser::new("a=\\v")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\x0b"); // '\v'

    let ini = DynConfig::from_ini(IniParser::new("a=\\f")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\x0c"); // '\f'

    let ini = DynConfig::from_ini(IniParser::new("a=\\\\")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\\");

    let ini = DynConfig::from_ini(IniParser::new("a=\\[")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "[");

    let ini = DynConfig::from_ini(IniParser::new("a=\"[\"")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "[");

    let ini = DynConfig::from_ini(IniParser::new("a=\\]")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "]");

    let ini = DynConfig::from_ini(IniParser::new("a=\"]\"")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "]");

    let ini = DynConfig::from_ini(IniParser::new("a=\\;")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), ";");

    let ini = DynConfig::from_ini(IniParser::new("a=\";\"")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), ";");

    let ini = DynConfig::from_ini(IniParser::new("a=\\#")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "#");

    let ini = DynConfig::from_ini(IniParser::new("a=\"#\"")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "#");

    let ini = DynConfig::from_ini(IniParser::new("a=\\=")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini(IniParser::new("a=\"=\"")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini(IniParser::new("a=\\:")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), ":");

    let ini = DynConfig::from_ini(IniParser::new("a=\":\"")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), ":");

    let ini = DynConfig::from_ini(IniParser::new("a=\\x00e4")).unwrap(); // ä
    assert_eq!(ini.root().get_string("a").unwrap(), "ä");

    let ini = DynConfig::from_ini(IniParser::new("a=\"\\x00e4\"")).unwrap(); // ä
    assert_eq!(ini.root().get_string("a").unwrap(), "ä");
}

#[test]
fn UnexpectedEndOfFileInUnicodeEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\x000"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 7,
            error: IniErrorKind::UnexpectedEndOfFileInUnicodeEscapeSequence
        }
    );
}

#[test]
fn UnexpectedNewLineInUnicodeEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\x\n"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedNewLineInUnicodeEscapeSequence
        }
    );
}

#[test]
fn InvalidUnicodeEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\xdfff"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 8,
            error: IniErrorKind::InvalidUnicodeEscapeSequence
        }
    );
}

#[test]
fn UnexpectedNewLineInQuotedValue() {
    // Unescaped newline.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\"\n")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInQuotedValue
        }
    );

    // But this succeeds.

    // Escaped newline.
    let ini = DynConfig::from_ini(IniParser::new("a=\\n")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\n");

    // Escaped newline in quoted string.
    let ini = DynConfig::from_ini(IniParser::new("a=\"\\n\"")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\n");

    // Line continuation.
    let ini = DynConfig::from_ini(IniParser::new("a=\\\n").line_continuation(true)).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    // Line continuation in quoted string.
    let ini = DynConfig::from_ini(IniParser::new("a=\"\\\n\"").line_continuation(true)).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");
}

#[test]
fn UnexpectedEndOfFileInQuotedString() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\"")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInQuotedString
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a=\"\"")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini =
        DynConfig::from_ini(IniParser::new("a=''").string_quotes(IniStringQuote::Single)).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");
}

#[test]
fn UnquotedString() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=a").unquoted_strings(false))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnquotedString
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a=a")).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "a");
}

#[test]
fn UnexpectedNewLineInArray() {
    // Arrays not supported.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[\n")).err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue('[')
        }
    );

    // Actual error.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[\n").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInArray
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a=[]").arrays(true)).unwrap();
    assert_eq!(ini.root().get_array("a").unwrap().len(), 0);
}

#[test]
fn MixedArray() {
    // Ints and bools.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[7, true]").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 10,
            error: IniErrorKind::MixedArray
        }
    );
    // Ints and strings.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[7, foo]").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 9,
            error: IniErrorKind::MixedArray
        }
    );
    // Ints and quoted strings.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[7, \"foo\"]").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 7,
            error: IniErrorKind::MixedArray
        }
    );

    // But this succeeds.

    // Ints and floats.
    let ini = DynConfig::from_ini(IniParser::new("a=[7, 3.14]").arrays(true)).unwrap();
    assert_eq!(ini.root().get_array("a").unwrap().len(), 2);
    assert_eq!(ini.root().get_array("a").unwrap().get_i64(0).unwrap(), 7);
    assert!(cmp_f64(
        ini.root().get_array("a").unwrap().get_f64(0).unwrap(),
        7.0
    ));
    assert_eq!(ini.root().get_array("a").unwrap().get_i64(1).unwrap(), 3);
    assert!(cmp_f64(
        ini.root().get_array("a").unwrap().get_f64(1).unwrap(),
        3.14
    ));

    // Strings and quoted strings.
    let ini = DynConfig::from_ini(IniParser::new("a=[foo, \"bar\"]").arrays(true)).unwrap();
    assert_eq!(ini.root().get_array("a").unwrap().len(), 2);
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "foo"
    );
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(1).unwrap(),
        "bar"
    );
}

#[test]
fn InvalidCharacterInArray() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[=]").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidCharacterInArray('=')
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[[]").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidCharacterInArray('[')
        }
    );
    // Unescaped space.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[a b]").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 6,
            error: IniErrorKind::InvalidCharacterInArray('b')
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("a=[\\=]").arrays(true)).unwrap(); // Escaped special character in array value.
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "="
    );

    let ini = DynConfig::from_ini(IniParser::new("a=[\\[]").arrays(true)).unwrap(); // Escaped special character in array value.
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "["
    );

    let ini = DynConfig::from_ini(IniParser::new("a=[\"\\=\"]").arrays(true)).unwrap(); // Escaped special character in quoted array value.
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "="
    );

    let ini = DynConfig::from_ini(IniParser::new("a=[\"=\"]").arrays(true)).unwrap(); // Unescaped special character in quoted array value.
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "="
    );

    let ini = DynConfig::from_ini(IniParser::new("a=[\"'\"]").arrays(true)).unwrap(); // Unmatched quote in quoted array value.
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "'"
    );

    let ini = DynConfig::from_ini(
        IniParser::new("a=['\"']")
            .arrays(true)
            .string_quotes(IniStringQuote::Single),
    )
    .unwrap(); // Unmatched quote in quoted array value.
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "\""
    );

    let ini =
        DynConfig::from_ini(IniParser::new("a=[\\x0066\\x006f\\x006f]").arrays(true)).unwrap(); // Unicode in array value ("foo").
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "foo"
    );

    let ini = DynConfig::from_ini(IniParser::new("a=[\" \"]").arrays(true)).unwrap(); // Unescaped whitespace in quoted array value.
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        " "
    );
}

#[test]
fn UnexpectedEndOfFileInArray() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInArray
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[7,").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::UnexpectedEndOfFileInArray
        }
    );

    // But this works (line continuations enabled).

    let ini = DynConfig::from_ini(
        IniParser::new("a=[7\\\n]")
            .arrays(true)
            .line_continuation(true),
    )
    .unwrap();
    assert_eq!(ini.root().get_array("a").unwrap().get_i64(0).unwrap(), 7);
}

#[test]
fn UnexpectedEndOfFileInQuotedArrayValue() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[\"").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedEndOfFileInQuotedArrayValue
        }
    );

    // But this works (line continuations enabled).

    let ini = DynConfig::from_ini(
        IniParser::new("a=[\"fo\\\no\"]")
            .arrays(true)
            .line_continuation(true),
    )
    .unwrap();
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "foo"
    );
}

fn cmp_f64(l: f64, r: f64) -> bool {
    (l - r).abs() < 0.000_001
}

#[test]
fn basic() {
    let ini = r#"bool = true
float = 3.14
int = 7
; "foo"
string = "\x0066\x006f\x006f"
array = [foo, bar, "baz",]

["other 'section'"]
other_bool = true
other_int = 7
other_float = 3.14
other_string = "foo"

[section]
bool = false
int = 9
float = 7.62
string = "bar""#;

    let config = DynConfig::from_ini(IniParser::new(ini).arrays(true)).unwrap();
    assert_eq!(config.root().len(), 5 + 2);

    assert_eq!(config.root().get_bool("bool").unwrap(), true);
    assert_eq!(config.root().get_i64("int").unwrap(), 7);
    assert!(cmp_f64(config.root().get_f64("float").unwrap(), 3.14));
    assert_eq!(config.root().get_string("string").unwrap(), "foo");

    let array = config.root().get_array("array").unwrap();

    assert_eq!(array.get_string(0).unwrap(), "foo");
    assert_eq!(array.get_string(1).unwrap(), "bar");
    assert_eq!(array.get_string(2).unwrap(), "baz");

    let section = config.root().get_table("section").unwrap();
    assert_eq!(section.len(), 4);

    assert_eq!(section.get_bool("bool").unwrap(), false);
    assert_eq!(section.get_i64("int").unwrap(), 9);
    assert!(cmp_f64(section.get_f64("float").unwrap(), 7.62));
    assert_eq!(section.get_string("string").unwrap(), "bar");

    let other_section = config.root().get_table("other 'section'").unwrap();
    assert_eq!(other_section.len(), 4);

    assert_eq!(other_section.get_bool("other_bool").unwrap(), true);
    assert_eq!(other_section.get_i64("other_int").unwrap(), 7);
    assert!(cmp_f64(other_section.get_f64("other_float").unwrap(), 3.14));
    assert_eq!(other_section.get_string("other_string").unwrap(), "foo");
}

#[test]
fn ArraysNotAllowed() {
    let mut config = DynConfig::new();
    config
        .root_mut()
        .set("array", Value::Array(DynArray::new()))
        .unwrap();

    assert_eq!(
        config.to_ini_string().err().unwrap(),
        ToIniStringError::ArraysNotAllowed
    );

    // But this succeeds.
    assert_eq!(
        config
            .to_ini_string_opts(ToIniStringOptions {
                arrays: true,
                ..Default::default()
            })
            .unwrap(),
        "array = []"
    );
}

#[test]
fn InvalidArrayType() {
    // Array of tables.
    {
        let mut config = DynConfig::new();
        let mut array = DynArray::new();
        array.push(Value::Table(DynTable::new())).unwrap();
        config.root_mut().set("array", Value::Array(array)).unwrap();

        assert_eq!(
            config
                .to_ini_string_opts(ToIniStringOptions {
                    arrays: true,
                    ..Default::default()
                })
                .err()
                .unwrap(),
            ToIniStringError::InvalidArrayType
        );
    }

    // Array of arrays.
    {
        let mut config = DynConfig::new();
        let mut array = DynArray::new();
        array.push(Value::Array(DynArray::new())).unwrap();
        config.root_mut().set("array", Value::Array(array)).unwrap();

        assert_eq!(
            config
                .to_ini_string_opts(ToIniStringOptions {
                    arrays: true,
                    ..Default::default()
                })
                .err()
                .unwrap(),
            ToIniStringError::InvalidArrayType
        );
    }
}

#[test]
fn NestedTablesNotSupported() {
    let mut config = DynConfig::new();
    config
        .root_mut()
        .set("table", Value::Table(DynTable::new()))
        .unwrap();
    let mut table = config.root_mut().get_table_mut("table").unwrap();
    table
        .set("nested_table", Value::Table(DynTable::new()))
        .unwrap();

    assert_eq!(
        config.to_ini_string(),
        Err(ToIniStringError::NestedTablesNotSupported)
    );
}

#[test]
fn from_string_and_back() {
    let ini = r#"array = ["foo", "bar", "baz"]
bool = true
float = 3.14
int = 7
string = "foo"

["other 'section'"]
other_bool = true
other_float = 3.14
other_int = 7
other_string = "foo 'bar'\t"

[section]
bool = false
float = 7.62
int = 9
string = "bar""#;

    let config = DynConfig::from_ini(IniParser::new(ini).arrays(true)).unwrap();

    let string = config
        .to_ini_string_opts(ToIniStringOptions {
            arrays: true,
            ..Default::default()
        })
        .unwrap();

    assert_eq!(ini, string);
}

#[test]
fn escape() {
    // With escape sequences supported.
    let ini = DynConfig::from_ini(
        IniParser::new("[a\\ b]\n\"c\\t\" = '\\x0066\\x006f\\x006f'")
            .string_quotes(IniStringQuote::Single | IniStringQuote::Double),
    )
    .unwrap();

    let section = ini.root().get_table("a b").unwrap();

    assert_eq!(section.len(), 1);
    assert_eq!(section.get_string("c\t").unwrap(), "foo");

    // Section name enclosed in double quotes when serializing back to string.
    assert_eq!(
        ini.to_ini_string().unwrap(),
        "[\"a b\"]\n\"c\\t\" = \"foo\""
    );

    // Attempt to serialize an escaped character with support for escaped characters disabled.
    let ini = DynConfig::from_ini(IniParser::new("a\\t = 7")).unwrap();

    assert_eq!(
        ini.to_ini_string_opts(ToIniStringOptions {
            escape: false,
            ..Default::default()
        })
        .err()
        .unwrap(),
        ToIniStringError::EscapedCharacterNotAllowed('\t')
    );

    // With escape sequences unsupported.
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new("[a\\ b]\n\"c\\t\" = '\\x0066\\x006f\\x006f'").escape(false)
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAfterSectionName('b')
        }
    );

    let ini = DynConfig::from_ini(
        IniParser::new("[\"a\\ b\"]\n\"c\\t\" = '\\x0066\\x006f\\x006f'").escape(false).string_quotes(IniStringQuote::Single | IniStringQuote::Double)
    )
    .unwrap();

    assert_eq!(
        ini.root()
            .get_table("a\\ b")
            .unwrap()
            .get_string("c\\t")
            .unwrap(),
        "\\x0066\\x006f\\x006f"
    );

    let string = "[\"a\\ b\"]\n\"c\\t\" = \"\\x0066\\x006f\\x006f\"";

    assert_ne!(ini.to_ini_string().unwrap(), string);
    assert_eq!(
        ini.to_ini_string_opts(ToIniStringOptions {
            escape: false,
            ..Default::default()
        })
        .unwrap(),
        string
    );
}
