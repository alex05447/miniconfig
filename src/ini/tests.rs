#![allow(non_snake_case)]

use {crate::*, ministr_macro::nestr};

fn dyn_config(string: &str) -> DynConfig {
    DynConfig::from_ini(IniParser::new(string)).expect("expected no error")
}

fn dyn_config_error(string: &str) -> IniError {
    DynConfig::from_ini(IniParser::new(string))
        .err()
        .expect("expected an error")
}

#[test]
fn InvalidCharacterAtLineStart() {
    // Unescaped special character.
    assert_eq!(
        dyn_config_error("'"),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::InvalidCharacterAtLineStart('\''),
            path: ConfigPath::new(),
        }
    );
    // Unescaped special character.
    assert_eq!(
        dyn_config_error(":"),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::InvalidCharacterAtLineStart(':'),
            path: ConfigPath::new(),
        }
    );
    // Unescaped special character.
    assert_eq!(
        dyn_config_error(" #"),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterAtLineStart('#'),
            path: ConfigPath::new(),
        }
    );
    // CRLF handling - CRLF ("\r\n") is treated as one newline.
    assert_eq!(
        dyn_config_error("foo = 7\r\n # "),
        IniError {
            line: 2,
            column: 2,
            error: IniErrorKind::InvalidCharacterAtLineStart('#'),
            path: ConfigPath::new(),
        }
    );
    assert_eq!(
        dyn_config_error("foo = 7\n # "),
        IniError {
            line: 2,
            column: 2,
            error: IniErrorKind::InvalidCharacterAtLineStart('#'),
            path: ConfigPath::new(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("\"a\"="); // Quoted key, empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini =
        DynConfig::from_ini(IniParser::new("' a'=").string_quotes(IniStringQuote::Single)).unwrap(); // Quoted key, empty value.
    assert_eq!(ini.root().get_string(" a").unwrap(), "");

    let ini = dyn_config("\\=="); // Escaped special character in key, empty value.
    assert_eq!(ini.root().get_string("=").unwrap(), "");

    let ini = dyn_config("a="); // Valid key, empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = dyn_config("`~@$%^&*()_-+,<.>/? ="); // Weird key.
    assert_eq!(ini.root().get_string("`~@$%^&*()_-+,<.>/?").unwrap(), "");

    let ini =
        DynConfig::from_ini(IniParser::new("a:").key_value_separator(IniKeyValueSeparator::Colon))
            .unwrap(); // Valid key, empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = dyn_config("[a]"); // Section
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = dyn_config(";"); // Comment
    assert_eq!(ini.root().len(), 0);

    let ini =
        DynConfig::from_ini(IniParser::new("#").comments(IniCommentDelimiter::NumberSign)).unwrap(); // Comment
    assert_eq!(ini.root().len(), 0);
}

#[test]
fn InvalidCharacterInSectionName() {
    // Unescaped special character.
    assert_eq!(
        dyn_config_error("[="),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterInSectionName('='),
            path: ConfigPath::new(),
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
            error: IniErrorKind::InvalidCharacterInSectionName(':'),
            path: ConfigPath::new(),
        }
    );
    // Unescaped special character.
    assert_eq!(
        dyn_config_error("[a#"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInSectionName('#'),
            path: ConfigPath::new(),
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a;").comments(IniCommentDelimiter::NumberSign),)
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInSectionName(';'),
            path: ConfigPath::new(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("[ \ta]"); // Skipped whitespace before section name.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = dyn_config("[a \t]"); // Skipped whitespace after section name.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = dyn_config("[\\=]"); // Special character in section.
    assert_eq!(ini.root().get_table("=").unwrap().len(), 0);

    let ini = dyn_config("[\\:]"); // Special character in section.
    assert_eq!(ini.root().get_table(":").unwrap().len(), 0);

    let ini = dyn_config("[a]"); // Normal section.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    dyn_config("[`~@$%^&*()_-+,<.>/?]"); // Weird section.

    let ini = dyn_config("[\\t\\ \\n]"); // Escaped whitespace in section.
    assert_eq!(ini.root().get_table("\t \n").unwrap().len(), 0);

    let ini = dyn_config("[\\x66\\x6f\\x6f]"); // Hexadecimal ASCII escape sequence in section ("foo").
    assert_eq!(ini.root().get_table("foo").unwrap().len(), 0);

    let ini = dyn_config("[\\u0066\\u006f\\u006f]"); // Hexadecimal Unicode escape sequence in section ("foo").
    assert_eq!(ini.root().get_table("foo").unwrap().len(), 0);

    let ini = dyn_config("[\t\"a \" ]"); // Whitespace in quoted section name.
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
        dyn_config_error("[a b]"),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidCharacterAfterSectionName('b'),
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::InvalidCharacterAfterSectionName('b'),
            path: vec![nestr!("a").into()].into(),
        }
    );

    // Any character after whitespace after nested section name.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a]\n[a/b c]").nested_section_depth(u32::MAX))
            .err()
            .unwrap(),
        IniError {
            line: 2,
            column: 6,
            error: IniErrorKind::InvalidCharacterAfterSectionName('c'),
            path: vec![nestr!("a").into(), nestr!("b").into()].into(),
        }
    );

    // Any character after whitespace after quoted nested section name.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a]\n[\"a\"/\"b\" c]").nested_section_depth(u32::MAX))
            .err()
            .unwrap(),
        IniError {
            line: 2,
            column: 10,
            error: IniErrorKind::InvalidCharacterAfterSectionName('c'),
            path: vec![nestr!("a").into(), nestr!("b").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("[\ta\\ b ]"); // Escaped whitespace in unquoted section name.
    assert_eq!(ini.root().get_table("a b").unwrap().len(), 0);

    let ini = dyn_config("[\t\"a b\" ]"); // Unescaped whitespace in quoted section name.
    assert_eq!(ini.root().get_table("a b").unwrap().len(), 0);

    let ini =
        DynConfig::from_ini(IniParser::new("[\"a \"]\n[\"a \"/b]").nested_section_depth(u32::MAX))
            .unwrap(); // Unescaped whitespace in quoted nested section name.
    let a = ini.root().get_table("a ").unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a.get_table("b").unwrap().len(), 0);

    let ini =
        DynConfig::from_ini(IniParser::new("[\"a\"]\n[\"a\" /b]").nested_section_depth(u32::MAX))
            .unwrap(); // Whitespace between quoted nested section names.
    let a = ini.root().get_table("a").unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a.get_table("b").unwrap().len(), 0);

    let ini = DynConfig::from_ini(
        IniParser::new("[\"a\"]\n[\"a\" / \"b\"]").nested_section_depth(u32::MAX),
    )
    .unwrap(); // Whitespace between quoted nested section names.
    let a = ini.root().get_table("a").unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a.get_table("b").unwrap().len(), 0);

    // Whitespace after nested section name.
    let ini =
        DynConfig::from_ini(IniParser::new("[a]\n[a /b]").nested_section_depth(u32::MAX)).unwrap();
    assert_eq!(
        ini.root()
            .get_table_path(&["a".into(), "b".into()])
            .unwrap()
            .len(),
        0
    );

    // Whitespace before nested section name.
    let ini =
        DynConfig::from_ini(IniParser::new("[a]\n[a/ b]").nested_section_depth(u32::MAX)).unwrap();
    assert_eq!(
        ini.root()
            .get_table_path(&["a".into(), "b".into()])
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn UnexpectedNewLineInSectionName() {
    // Unescaped new line at section name start.
    assert_eq!(
        dyn_config_error("[\n"),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedNewLineInSectionName,
            path: ConfigPath::new(),
        }
    );
    // Unescaped new line in section name.
    assert_eq!(
        dyn_config_error("[a\n"),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInSectionName,
            path: ConfigPath::new(),
        }
    );
    // Unescaped new line in quoted section name.
    assert_eq!(
        dyn_config_error("[\"a\n"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInSectionName,
            path: ConfigPath::new(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("[\\n]"); // Escaped new line at section name start.
    assert_eq!(ini.root().get_table("\n").unwrap().len(), 0);

    let ini = dyn_config("[a\\n]"); // Escaped new line in section name.
    assert_eq!(ini.root().get_table("a\n").unwrap().len(), 0);

    let ini = dyn_config("[ \"a\\n\" ]"); // Escaped new line in quoted section name.
    assert_eq!(ini.root().get_table("a\n").unwrap().len(), 0);
}

#[test]
fn UnexpectedEndOfFileInSectionName() {
    assert_eq!(
        dyn_config_error("[a"),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileInSectionName,
            path: ConfigPath::new(),
        }
    );
    assert_eq!(
        dyn_config_error("[\"a"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInSectionName,
            path: ConfigPath::new(),
        }
    );
    assert_eq!(
        dyn_config_error("[\"a\""),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedEndOfFileInSectionName,
            path: ConfigPath::new(),
        }
    );
}

#[test]
fn EmptySectionName() {
    assert_eq!(
        dyn_config_error("[]"),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::EmptySectionName,
            path: ConfigPath::new(),
        }
    );
    assert_eq!(
        dyn_config_error("[\"\"]"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::EmptySectionName,
            path: ConfigPath::new(),
        }
    );
    // Empty parent section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[/a]").nested_section_depth(u32::MAX))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::EmptySectionName,
            path: ConfigPath::new(),
        }
    );
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new("[b//a]")
                .nested_section_depth(u32::MAX)
                .implicit_parent_sections(true)
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::EmptySectionName,
            path: vec![nestr!("b").into()].into(),
        }
    );
    // Empty child section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a]\n[a/]").nested_section_depth(u32::MAX))
            .err()
            .unwrap(),
        IniError {
            line: 2,
            column: 4,
            error: IniErrorKind::EmptySectionName,
            path: vec![nestr!("a").into()].into(),
        }
    );
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new("[a/]")
                .nested_section_depth(u32::MAX)
                .implicit_parent_sections(true)
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::EmptySectionName,
            path: vec![nestr!("a").into()].into(),
        }
    );
    // Empty child section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a]\n[a/b]\n[a/b/]").nested_section_depth(u32::MAX))
            .err()
            .unwrap(),
        IniError {
            line: 3,
            column: 6,
            error: IniErrorKind::EmptySectionName,
            path: vec![nestr!("a").into(), nestr!("b").into()].into(),
        }
    );
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new("[a/b/]")
                .nested_section_depth(u32::MAX)
                .implicit_parent_sections(true)
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 6,
            error: IniErrorKind::EmptySectionName,
            path: vec![nestr!("a").into(), nestr!("b").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("[\\ ]"); // Whitespace in section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);

    let ini = dyn_config("[\\t]"); // Whitespace in section.
    assert_eq!(ini.root().get_table("\t").unwrap().len(), 0);

    let ini = dyn_config("[\\n]"); // Whitespace in section.
    assert_eq!(ini.root().get_table("\n").unwrap().len(), 0);

    let ini = dyn_config("[\t\" \" ]"); // Whitespace in quoted section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);
}

#[test]
fn InvalidParentSection() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a/]").nested_section_depth(u32::MAX))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidParentSection,
            path: vec![nestr!("a").into()].into(),
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a/b]").nested_section_depth(u32::MAX))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidParentSection,
            path: vec![nestr!("a").into()].into(),
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a]\n[a/b/]").nested_section_depth(u32::MAX))
            .err()
            .unwrap(),
        IniError {
            line: 2,
            column: 4,
            error: IniErrorKind::InvalidParentSection,
            path: vec![nestr!("a").into(), nestr!("b").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("[a/]"); // `/` is just a normal character if nested sections are not supported.
    assert_eq!(ini.root().get_table("a/").unwrap().len(), 0);

    let ini = dyn_config("[a/b]"); // `/` is just a normal character if nested sections are not supported.
    assert_eq!(ini.root().get_table("a/b").unwrap().len(), 0);

    let ini = dyn_config("[a]\n[a/b/]"); // `/` is just a normal character if nested sections are not supported.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);
    assert_eq!(ini.root().get_table("a/b/").unwrap().len(), 0);

    let ini =
        DynConfig::from_ini(IniParser::new("[a]\n[a/b]").nested_section_depth(u32::MAX)).unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().len(), 1);
    assert_eq!(
        ini.root()
            .get_table_path(&["a".into(), "b".into()])
            .unwrap()
            .len(),
        0
    );

    // Implicit parent sections are allowed.
    let ini = DynConfig::from_ini(
        IniParser::new("[a/b]")
            .nested_section_depth(u32::MAX)
            .implicit_parent_sections(true),
    )
    .unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().len(), 1);
    assert!(ini
        .root()
        .get_table_path(&["a".into(), "b".into()])
        .unwrap()
        .is_empty());
}

#[test]
fn NestedSectionDepthExceeded() {
    // `nested_section_depth == 0` - sections not supported at all
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new(
                r#"[a]
[a/b]
[a/b/c]"#
            )
            .nested_section_depth(0)
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::NestedSectionDepthExceeded,
            path: ConfigPath::new(),
        }
    );

    // `nested_section_depth == 1` - sections supported, nested section separators ('/') treated as normal section name chars.
    let ini = DynConfig::from_ini(
        IniParser::new(
            r#"[a]
[a/b]
[a/b/c]"#,
        )
        .nested_section_depth(1),
    )
    .unwrap();

    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);
    assert_eq!(ini.root().get_table("a/b").unwrap().len(), 0);
    assert_eq!(ini.root().get_table("a/b/c").unwrap().len(), 0);

    // `nested_section_depth == 2` - 2 levels of nested sections supported,
    // nested section separators ('/') are not valid section name chars (need to be escaped).
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new(
                r#"[a]
[a/b]
[a/b/c]"#
            )
            .nested_section_depth(2)
        )
        .err()
        .unwrap(),
        IniError {
            line: 3,
            column: 5,
            error: IniErrorKind::NestedSectionDepthExceeded,
            path: vec![nestr!("a").into(), nestr!("b").into()].into(),
        }
    );

    let ini = DynConfig::from_ini(
        IniParser::new(
            r#"[a]
[a/b]
[a/b/c]"#,
        )
        .nested_section_depth(3),
    )
    .unwrap();

    assert_eq!(ini.root().get_table("a").unwrap().len(), 1);
    assert_eq!(
        ini.root()
            .get_table_path(&["a".into(), "b".into()])
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        ini.root()
            .get_table_path(&["a".into(), "b".into(), "c".into()])
            .unwrap()
            .len(),
        0
    );

    // Same with implicit parent sections.
    let ini = DynConfig::from_ini(
        IniParser::new(r#"[a/b/c]"#)
            .nested_section_depth(3)
            .implicit_parent_sections(true),
    )
    .unwrap();

    assert_eq!(ini.root().get_table("a").unwrap().len(), 1);
    assert_eq!(
        ini.root()
            .get_table_path(&["a".into(), "b".into()])
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        ini.root()
            .get_table_path(&["a".into(), "b".into(), "c".into()])
            .unwrap()
            .len(),
        0
    );
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
            error: IniErrorKind::DuplicateSection,
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::DuplicateKey,
            path: vec![nestr!("a").into(), nestr!("a").into()].into(),
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
        dyn_config_error("[a] b"),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAtLineEnd('b'),
            path: vec![nestr!("a").into()].into(),
        }
    );
    // After value.
    assert_eq!(
        dyn_config_error("a=7 b"),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAtLineEnd('b'),
            path: ConfigPath::new(),
        }
    );
    // Inline comments not supported.
    assert_eq!(
        dyn_config_error("[a] ;"),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAtLineEnd(';'),
            path: vec![nestr!("a").into()].into(),
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini(IniParser::new("[a] ;").inline_comments(true)).unwrap(); // Supported inline comment.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = DynConfig::from_ini(
        IniParser::new("[a] #")
            .comments(IniCommentDelimiter::NumberSign)
            .inline_comments(true),
    )
    .unwrap(); // Supported inline comment.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);
}

#[test]
fn InvalidCharacterInKey() {
    // Unescaped special character.
    assert_eq!(
        dyn_config_error("a["),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterInKey('['),
            path: ConfigPath::new(),
        }
    );
    // Unescaped special character.
    assert_eq!(
        dyn_config_error(" a'"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInKey('\''),
            path: ConfigPath::new(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("a=true"); // Normal key.
    assert_eq!(ini.root().get_bool("a").unwrap(), true);

    let ini = dyn_config("\"a\"=true"); // Quoted key.
    assert_eq!(ini.root().get_bool("a").unwrap(), true);

    let ini =
        DynConfig::from_ini(IniParser::new("' a ' = false").string_quotes(IniStringQuote::Single))
            .unwrap(); // Quoted key.
    assert_eq!(ini.root().get_bool(" a ").unwrap(), false);

    let ini = dyn_config("a\\[=7"); // Special character in key.
    assert_eq!(ini.root().get_i64("a[").unwrap(), 7);

    let ini = dyn_config("a\\t=3.14"); // Escaped whitespace in key.
    assert!(cmp_f64(ini.root().get_f64("a\t").unwrap(), 3.14));

    let ini = dyn_config("\\x66\\x6f\\x6f=\"bar\""); // Hexadecimal ASCII escape sequence in key ("foo").
    assert_eq!(ini.root().get_string("foo").unwrap(), "bar");

    let ini = dyn_config("\\u0066\\u006f\\u006f=\"bar\""); // Hexadecimal Unicode escape sequence in key ("foo").
    assert_eq!(ini.root().get_string("foo").unwrap(), "bar");
}

#[test]
fn UnexpectedNewLineInKey() {
    // New line in key.
    assert_eq!(
        dyn_config_error("a\n"),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedNewLineInKey,
            path: ConfigPath::new(),
        }
    );
    // New line in quoted key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[b]\n\"a\n\""))
            .err()
            .unwrap(),
        IniError {
            line: 2,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInKey,
            path: vec![nestr!("b").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("a\\n=7"); // Escaped new line in key.
    assert_eq!(ini.root().get_i64("a\n").unwrap(), 7);

    let ini = dyn_config("a=\n"); // Empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");
}

#[test]
fn EmptyKey() {
    // Empty unquoted key.
    assert_eq!(
        dyn_config_error(" = 7"),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::EmptyKey,
            path: ConfigPath::new(),
        }
    );
    assert_eq!(
        dyn_config_error("[a]\n=false"),
        IniError {
            line: 2,
            column: 0,
            error: IniErrorKind::EmptyKey,
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::EmptyKey,
            path: ConfigPath::new(),
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
            error: IniErrorKind::EmptyKey,
            path: ConfigPath::new(),
        }
    );
    // Empty quoted key.
    assert_eq!(
        DynConfig::from_ini(
            IniParser::new(" '' : 7")
                .key_value_separator(IniKeyValueSeparator::Colon)
                .string_quotes(IniStringQuote::Single),
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::EmptyKey,
            path: ConfigPath::new(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("a = 7");
    assert_eq!(ini.root().get_i64("a").unwrap(), 7);

    let ini = dyn_config("[a]\n\" \" = 7");
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
            error: IniErrorKind::DuplicateKey,
            path: vec![nestr!("a").into()].into(),
        }
    );
    // In the section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[x]\na=7\nb=8\na=9\nc=10"))
            .err()
            .unwrap(),
        IniError {
            line: 4,
            column: 1,
            error: IniErrorKind::DuplicateKey,
            path: vec![nestr!("x").into(), nestr!("a").into()].into(),
        }
    );
    // In the merged section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[x]\na=7\nb=8\n[x]\na=9\nc=10"))
            .err()
            .unwrap(),
        IniError {
            line: 5,
            column: 1,
            error: IniErrorKind::DuplicateKey,
            path: vec![nestr!("x").into(), nestr!("a").into()].into()
        }
    );
    // Key and section.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=7\n[a]\nb=8"))
            .err()
            .unwrap(),
        IniError {
            line: 2,
            column: 2,
            error: IniErrorKind::DuplicateKey,
            path: vec![nestr!("a").into()].into(),
        }
    );
    // Section and key.
    assert_eq!(
        DynConfig::from_ini(IniParser::new("[a]\nb=8\n[a / b]").nested_section_depth(2))
            .err()
            .unwrap(),
        IniError {
            line: 3,
            column: 6,
            error: IniErrorKind::DuplicateKey,
            path: vec![nestr!("a").into(), nestr!("b").into()].into(),
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

    // Key and section, `First`.
    let ini = DynConfig::from_ini(
        IniParser::new("a=7\n[a]\nb=8").duplicate_keys(IniDuplicateKeys::First),
    )
    .unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 7);
    assert!(ini.root().get_table("a").is_err());

    // Key and section, `Last`.
    let ini =
        DynConfig::from_ini(IniParser::new("a=7\n[a]\nb=8").duplicate_keys(IniDuplicateKeys::Last))
            .unwrap();
    assert!(ini.root().get_i64("a").is_err());
    assert_eq!(ini.root().get_table("a").unwrap().len(), 1);

    // Section and key, `First`.
    let ini = DynConfig::from_ini(
        IniParser::new("[a]\nb=8\n[a/ b]")
            .duplicate_keys(IniDuplicateKeys::First)
            .nested_section_depth(2),
    )
    .unwrap();
    assert_eq!(
        ini.root().get_i64_path(&["a".into(), "b".into()]).unwrap(),
        8
    );
    assert!(ini
        .root()
        .get_table_path(&["a".into(), "b".into()])
        .is_err());

    // Section and key, `Last`.
    let ini = DynConfig::from_ini(
        IniParser::new("[a]\nb=8\n[a/b]")
            .duplicate_keys(IniDuplicateKeys::Last)
            .nested_section_depth(2),
    )
    .unwrap();
    assert!(ini.root().get_i64_path(&["a".into(), "b".into()]).is_err());
    assert_eq!(
        ini.root()
            .get_table_path(&["a".into(), "b".into()])
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn UnexpectedEndOfFileBeforeKeyValueSeparator() {
    // Unquoted key.
    assert_eq!(
        dyn_config_error("a"),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedEndOfFileBeforeKeyValueSeparator,
            path: ConfigPath::new(),
        }
    );
    // Quoted key.
    assert_eq!(
        dyn_config_error("[\"foo\"]\n\"a \""),
        IniError {
            line: 2,
            column: 4,
            error: IniErrorKind::UnexpectedEndOfFileBeforeKeyValueSeparator,
            path: vec![nestr!("foo").into(), nestr!("a ").into()].into(),
        }
    );

    // But this succeeds (empty value).

    let ini = dyn_config("a=");
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
        dyn_config_error("a !"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidKeyValueSeparator('!'),
            path: vec![nestr!("a").into()].into(),
        }
    );
    assert_eq!(
        dyn_config_error("a :"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidKeyValueSeparator(':'),
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::InvalidKeyValueSeparator('b'),
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::InvalidKeyValueSeparator('b'),
            path: vec![nestr!("a").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("a = 7");
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
        dyn_config_error("a=="),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue('='),
            path: vec![nestr!("a").into()].into(),
        }
    );
    // Unescaped special character.
    assert_eq!(
        dyn_config_error("a=:"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue(':'),
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::InvalidCharacterInValue('='),
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::InvalidCharacterInValue(':'),
            path: vec![nestr!("a").into()].into(),
        }
    );
    // Inline comments not supported.
    assert_eq!(
        dyn_config_error("a=a;"),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidCharacterInValue(';'),
            path: vec![nestr!("a").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("a=\\="); // Escaped special character in value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini(
        IniParser::new("a:\\=").key_value_separator(IniKeyValueSeparator::Colon),
    )
    .unwrap(); // Escaped special character in value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = dyn_config("a=\"\\=\""); // Escaped special character in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = dyn_config("a=\"=\""); // Unescaped special character in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = dyn_config("a=\"'\""); // Unmatched quote in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "'");

    let ini = DynConfig::from_ini(IniParser::new("a='\"'").string_quotes(IniStringQuote::Single))
        .unwrap(); // Unmatched quote in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "\"");

    let ini = DynConfig::from_ini(IniParser::new("a=a;").inline_comments(true)).unwrap(); // Supported inline comments.
    assert_eq!(ini.root().get_string("a").unwrap(), "a");

    let ini = DynConfig::from_ini(
        IniParser::new("a=a#")
            .comments(IniCommentDelimiter::NumberSign)
            .inline_comments(true),
    )
    .unwrap(); // Supported inline comments.
    assert_eq!(ini.root().get_string("a").unwrap(), "a");

    let ini = dyn_config("foo=\\x66\\x6f\\x6f"); // Hexadecimal ASCII escape sequence in value ("foo").
    assert_eq!(ini.root().get_string("foo").unwrap(), "foo");

    let ini = dyn_config("foo=\\u0066\\u006f\\u006f"); // Hexadecimal Unicode escape sequence in value ("foo").
    assert_eq!(ini.root().get_string("foo").unwrap(), "foo");

    let ini = dyn_config("a=\" \""); // Unescaped whitespace in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), " ");
}

#[test]
fn UnexpectedEndOfFileInEscapeSequence() {
    // In section.
    assert_eq!(
        dyn_config_error("[\\"),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence,
            path: ConfigPath::new(),
        }
    );
    // In quoted section.
    assert_eq!(
        dyn_config_error("[\"\\"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence,
            path: ConfigPath::new(),
        }
    );
    // In key.
    assert_eq!(
        dyn_config_error("\\"),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence,
            path: ConfigPath::new(),
        }
    );
    // In quoted key.
    assert_eq!(
        dyn_config_error("\"\\"),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence,
            path: ConfigPath::new(),
        }
    );
    // In unquoted value.
    assert_eq!(
        dyn_config_error("a=\\"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );
    // In quoted value.
    assert_eq!(
        dyn_config_error("a=\"\\"),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("[\\ ]"); // Escaped space in section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);

    let ini = dyn_config("[ \"\\ \" ]"); // Escaped space in quoted section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);

    let ini = dyn_config("\\ ="); // Escaped space in key.
    assert_eq!(ini.root().get_string(" ").unwrap(), "");

    let ini = dyn_config("\"\\ \" ="); // Escaped space in quoted key.
    assert_eq!(ini.root().get_string(" ").unwrap(), "");

    let ini = dyn_config("a = \\ "); // Escaped space in unquoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), " ");

    let ini = dyn_config("a = \"\\ \""); // Escaped space in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), " ");
}

#[test]
fn UnexpectedNewLineInEscapeSequence() {
    // Unsupported line continuation in section name.
    assert_eq!(
        dyn_config_error("[\\\n"),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInEscapeSequence,
            path: ConfigPath::new(),
        }
    );
    // Unsupported line continuation in key.
    assert_eq!(
        dyn_config_error("a\\\n"),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInEscapeSequence,
            path: ConfigPath::new(),
        }
    );
    // Unsupported line continuation in value.
    assert_eq!(
        dyn_config_error("a=\\\n"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInEscapeSequence,
            path: vec![nestr!("a").into()].into(),
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

    // Line continuation in quoted string.
    let ini =
        DynConfig::from_ini(IniParser::new("a = \"foo\\\nbar\"").line_continuation(true)).unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "foobar");
}

#[test]
fn InvalidEscapeCharacter() {
    assert_eq!(
        dyn_config_error("a=\\z"),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidEscapeCharacter('z'),
            path: vec![nestr!("a").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("a=\\ ");
    assert_eq!(ini.root().get_string("a").unwrap(), " ");

    let ini = dyn_config("a=\" \"");
    assert_eq!(ini.root().get_string("a").unwrap(), " ");

    let ini = dyn_config("a=\\0");
    assert_eq!(ini.root().get_string("a").unwrap(), "\0");

    let ini = dyn_config("a=\\a");
    assert_eq!(ini.root().get_string("a").unwrap(), "\x07"); // '\a'

    let ini = dyn_config("a=\\b");
    assert_eq!(ini.root().get_string("a").unwrap(), "\x08"); // '\a'

    let ini = dyn_config("a=\\t");
    assert_eq!(ini.root().get_string("a").unwrap(), "\t");

    let ini = dyn_config("a=\\n");
    assert_eq!(ini.root().get_string("a").unwrap(), "\n");

    let ini = dyn_config("a=\\r");
    assert_eq!(ini.root().get_string("a").unwrap(), "\r");

    let ini = dyn_config("a=\\v");
    assert_eq!(ini.root().get_string("a").unwrap(), "\x0b"); // '\v'

    let ini = dyn_config("a=\\f");
    assert_eq!(ini.root().get_string("a").unwrap(), "\x0c"); // '\f'

    let ini = dyn_config("a=\\\\");
    assert_eq!(ini.root().get_string("a").unwrap(), "\\");

    let ini = dyn_config("a=\\[");
    assert_eq!(ini.root().get_string("a").unwrap(), "[");

    let ini = dyn_config("a=\"[\"");
    assert_eq!(ini.root().get_string("a").unwrap(), "[");

    let ini = dyn_config("a=\\]");
    assert_eq!(ini.root().get_string("a").unwrap(), "]");

    let ini = dyn_config("a=\"]\"");
    assert_eq!(ini.root().get_string("a").unwrap(), "]");

    let ini = dyn_config("a=\\;");
    assert_eq!(ini.root().get_string("a").unwrap(), ";");

    let ini = dyn_config("a=\";\"");
    assert_eq!(ini.root().get_string("a").unwrap(), ";");

    let ini = dyn_config("a=\\#");
    assert_eq!(ini.root().get_string("a").unwrap(), "#");

    let ini = dyn_config("a=\"#\"");
    assert_eq!(ini.root().get_string("a").unwrap(), "#");

    let ini = dyn_config("a=\\=");
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = dyn_config("a=\"=\"");
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = dyn_config("a=\\:");
    assert_eq!(ini.root().get_string("a").unwrap(), ":");

    let ini = dyn_config("a=\":\"");
    assert_eq!(ini.root().get_string("a").unwrap(), ":");

    let ini = dyn_config("a=\\x40"); // @
    assert_eq!(ini.root().get_string("a").unwrap(), "@");

    let ini = dyn_config("a=\"\\x40\""); // @
    assert_eq!(ini.root().get_string("a").unwrap(), "@");

    let ini = dyn_config("a=\\u00e4"); // ä
    assert_eq!(ini.root().get_string("a").unwrap(), "ä");

    let ini = dyn_config("a=\"\\u00e4\""); // ä
    assert_eq!(ini.root().get_string("a").unwrap(), "ä");
}

#[test]
fn UnexpectedEndOfFileInASCIIEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\x0")).err().unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::UnexpectedEndOfFileInASCIIEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );
}

#[test]
fn UnexpectedEndOfFileInUnicodeEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\u000"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 7,
            error: IniErrorKind::UnexpectedEndOfFileInUnicodeEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );
}

#[test]
fn UnexpectedNewLineInASCIIEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\x\n"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedNewLineInASCIIEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );
}

#[test]
fn UnexpectedNewLineInUnicodeEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\u\n"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedNewLineInUnicodeEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );
}

#[test]
fn InvalidASCIIEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\x$?"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidASCIIEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\"\\xf\""))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 7,
            error: IniErrorKind::InvalidASCIIEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );
}

#[test]
fn InvalidUnicodeEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\\udfff"))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 8,
            error: IniErrorKind::InvalidUnicodeEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=\"\\udff\""))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 9,
            error: IniErrorKind::InvalidUnicodeEscapeSequence,
            path: vec![nestr!("a").into()].into(),
        }
    );
}

#[test]
fn UnexpectedNewLineInQuotedValue() {
    // Unescaped newline.
    assert_eq!(
        dyn_config_error("a=\"\n"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInQuotedValue,
            path: vec![nestr!("a").into()].into(),
        }
    );

    // But this succeeds.

    // Escaped newline.
    let ini = dyn_config("a=\\n");
    assert_eq!(ini.root().get_string("a").unwrap(), "\n");

    // Escaped newline in quoted string.
    let ini = dyn_config("a=\"\\n\"");
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
        dyn_config_error("a=\""),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInQuotedString,
            path: vec![nestr!("a").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("a=\"\"");
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
            error: IniErrorKind::UnquotedString,
            path: vec![nestr!("a").into()].into(),
        }
    );

    // But this succeeds.

    let ini = dyn_config("a=a");
    assert_eq!(ini.root().get_string("a").unwrap(), "a");
}

#[test]
fn UnexpectedNewLineInArray() {
    // Arrays not supported.
    assert_eq!(
        dyn_config_error("a=[\n"),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue('['),
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::UnexpectedNewLineInArray,
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::MixedArray,
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::MixedArray,
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::MixedArray,
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::InvalidCharacterInArray('='),
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::InvalidCharacterInArray('['),
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::InvalidCharacterInArray('b'),
            path: vec![nestr!("a").into()].into(),
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

    let ini = DynConfig::from_ini(IniParser::new("a=[\\x66\\x6f\\x6f]").arrays(true)).unwrap(); // Hexadecimal ASCII escape sequence in array value ("foo").
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "foo"
    );
    let ini =
        DynConfig::from_ini(IniParser::new("a=[\\u0066\\u00f6\\u00f6]").arrays(true)).unwrap(); // Hexadecimal Unicode escape sequence in array value ("föö").
    assert_eq!(
        ini.root().get_array("a").unwrap().get_string(0).unwrap(),
        "föö"
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
            error: IniErrorKind::UnexpectedEndOfFileInArray,
            path: vec![nestr!("a").into()].into(),
        }
    );
    assert_eq!(
        DynConfig::from_ini(IniParser::new("a=[7,").arrays(true))
            .err()
            .unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::UnexpectedEndOfFileInArray,
            path: vec![nestr!("a").into()].into(),
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
            error: IniErrorKind::UnexpectedEndOfFileInQuotedArrayValue,
            path: vec![nestr!("a").into()].into(),
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

#[test]
fn basic() {
    let ini = r#"bool = true
float = 3.14
; hexadecimal
int = +0x17
; "foo"
string = "\x66\x6f\x6f"
; "föö"
unicode_string = "\u0066\u00f6\u00f6"
array = [foo, bar, "baz",]

["other 'section'"]
other_bool = true
; octal
other_int = -0o17
other_float = 3.14
other_string = "foo"

[section]
bool = false
int = 9
float = 7.62
string = "bar""#;

    let config = DynConfig::from_ini(IniParser::new(ini).arrays(true)).unwrap();
    assert_eq!(config.root().len(), 6 + 2);

    assert_eq!(config.root().get_bool("bool").unwrap(), true);
    assert_eq!(config.root().get_i64("int").unwrap(), 23);
    assert!(cmp_f64(config.root().get_f64("float").unwrap(), 3.14));
    assert_eq!(config.root().get_string("string").unwrap(), "foo");
    assert_eq!(config.root().get_string("unicode_string").unwrap(), "föö");

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
    assert_eq!(other_section.get_i64("other_int").unwrap(), -15);
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
fn NestedSectionsNotAllowed() {
    let mut config = DynConfig::new();
    config
        .root_mut()
        .set("table", Value::Table(DynTable::new()))
        .unwrap();
    let table = config.root_mut().get_table_mut("table").unwrap();
    table
        .set("nested_table", Value::Table(DynTable::new()))
        .unwrap();

    assert_eq!(
        config.to_ini_string(),
        Err(ToIniStringError::NestedSectionDepthExceeded)
    );

    // But this works.

    assert_eq!(
        config
            .to_ini_string_opts(ToIniStringOptions {
                nested_section_depth: 2,
                ..Default::default()
            })
            .unwrap(),
        "[table]\n\n[table/nested_table]"
    );

    // With implicit parent sections.
    assert_eq!(
        config
            .to_ini_string_opts(ToIniStringOptions {
                nested_section_depth: 2,
                implicit_parent_sections: true,
                ..Default::default()
            })
            .unwrap(),
        "[table/nested_table]"
    );

    config.root_mut().set("baz", Value::Bool(true)).unwrap();
    config
        .root_mut()
        .set("bill", Value::String("bob".into()))
        .unwrap();

    let table = config.root_mut().get_table_mut("table").unwrap();
    table.set("foo", Value::I64(7)).unwrap();
    table.set("bar", Value::F64(3.14)).unwrap();

    assert_eq!(
        config
            .to_ini_string_opts(ToIniStringOptions {
                nested_section_depth: 2,
                ..Default::default()
            })
            .unwrap(),
        "baz = true\nbill = \"bob\"\n\n[table]\nbar = 3.14\nfoo = 7\n\n[table/nested_table]"
    );

    let table = config.root_mut().get_table_mut("table").unwrap();
    table
        .get_table_mut("nested_table")
        .unwrap()
        .set("another_nested_table", Value::Table(DynTable::new()))
        .unwrap();

    assert_eq!(config
        .to_ini_string_opts(ToIniStringOptions {
            nested_section_depth: 3,
            ..Default::default()
        })
        .unwrap(),
        "baz = true\nbill = \"bob\"\n\n[table]\nbar = 3.14\nfoo = 7\n\n[table/nested_table]\n\n[table/nested_table/another_nested_table]"
    );

    // With implicit parent sections.
    assert_eq!(config
        .to_ini_string_opts(ToIniStringOptions {
            nested_section_depth: 3,
            implicit_parent_sections: true,
            ..Default::default()
        })
        .unwrap(),
        "baz = true\nbill = \"bob\"\n\n[table]\nbar = 3.14\nfoo = 7\n\n[table/nested_table/another_nested_table]"
    );

    config
        .root_mut()
        .set("test_table", Value::Table(DynTable::new()))
        .unwrap();

    assert_eq!(config
        .to_ini_string_opts(ToIniStringOptions {
            nested_section_depth: 3,
            ..Default::default()
        })
        .unwrap(),
        "baz = true\nbill = \"bob\"\n\n[table]\nbar = 3.14\nfoo = 7\n\n[table/nested_table]\n\n[table/nested_table/another_nested_table]\n\n[test_table]"
    );

    // With implicit parent sections.
    assert_eq!(config
        .to_ini_string_opts(ToIniStringOptions {
            nested_section_depth: 3,
            implicit_parent_sections: true,
            ..Default::default()
        })
        .unwrap(),
        "baz = true\nbill = \"bob\"\n\n[table]\nbar = 3.14\nfoo = 7\n\n[table/nested_table/another_nested_table]\n\n[test_table]"
    );
}

#[test]
fn from_string_and_back() {
    let ini = r#"array = ["foo", "bar", "baz"]
bool = true
float = 3.14
int = 7
string = "foo"

["other 's/e/c/t/i/o/n'"]
other_bool = true
other_float = 5.45
other_int = 9
other_string = "foo 'bar'\t"

[section]
bool = false
float = 7.62
int = 11
string = "bar"

[section/nested_section]
bool = false
float = 5.56
int = 13
string = "baz""#;

    let config = DynConfig::from_ini(
        IniParser::new(ini)
            .arrays(true)
            .nested_section_depth(u32::MAX),
    )
    .unwrap();

    let string = config
        .to_ini_string_opts(ToIniStringOptions {
            arrays: true,
            nested_section_depth: 2,
            ..Default::default()
        })
        .unwrap();

    assert_eq!(ini, string);
}

#[test]
fn escape() {
    // With escape sequences supported.
    let ini = DynConfig::from_ini(
        IniParser::new(
            r#"[a\ b]
"c\t" = '\x66\x6f\x6f'"#,
        )
        .string_quotes(IniStringQuote::Single | IniStringQuote::Double),
    )
    .unwrap();

    let section = ini.root().get_table("a b").unwrap();

    assert_eq!(section.len(), 1);
    assert_eq!(section.get_string("c\t").unwrap(), "foo");

    // Section name enclosed in double quotes when serializing back to string.
    assert_eq!(
        ini.to_ini_string().unwrap(),
        r#"["a b"]
"c\t" = "foo""#
    );

    // Attempt to serialize an escaped character with support for escaped characters disabled.
    let ini = dyn_config("a\\t = 7");

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
            IniParser::new(
                r#"[a\ b]
"c\t" = '\u0066\u006f\u006f'"#
            )
            .escape(false)
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAfterSectionName('b'),
            path: vec![nestr!("a\\").into()].into(),
        }
    );

    let ini = DynConfig::from_ini(
        IniParser::new(
            r#"["a\ b"]
"c\t" = '\u0066\u00f6\u00f6'"#,
        )
        .escape(false)
        .string_quotes(IniStringQuote::Single | IniStringQuote::Double),
    )
    .unwrap();

    assert_eq!(
        ini.root()
            .get_table("a\\ b")
            .unwrap()
            .get_string("c\\t")
            .unwrap(),
        "\\u0066\\u00f6\\u00f6"
    );

    let string = r#"["a\ b"]
"c\t" = "\u0066\u00f6\u00f6""#;

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
