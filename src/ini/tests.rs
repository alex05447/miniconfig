#![allow(non_snake_case)]

use crate::*;

#[test]
fn InvalidCharacterAtLineStart() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini("'").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::InvalidCharacterAtLineStart
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(":").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::InvalidCharacterAtLineStart
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(" #").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterAtLineStart
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("\"a\"=").unwrap(); // Quoted key, empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = DynConfig::from_ini_opts(
        "' a'=",
        IniOptions {
            string_quotes: IniStringQuote::Single,
            ..Default::default()
        },
    )
    .unwrap(); // Quoted key, empty value.
    assert_eq!(ini.root().get_string(" a").unwrap(), "");

    let ini = DynConfig::from_ini("\\==").unwrap(); // Escaped special character in key, empty value.
    assert_eq!(ini.root().get_string("=").unwrap(), "");

    let ini = DynConfig::from_ini("a=").unwrap(); // Valid key, empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = DynConfig::from_ini("`~@$%^&*()_-+,<.>/? =").unwrap(); // Weird key.
    assert_eq!(ini.root().get_string("`~@$%^&*()_-+,<.>/?").unwrap(), "");

    let ini = DynConfig::from_ini_opts(
        "a:",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..IniOptions::default()
        },
    )
    .unwrap(); // Valid key, empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = DynConfig::from_ini("[a]").unwrap(); // Section
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = DynConfig::from_ini(";").unwrap(); // Comment
    assert_eq!(ini.root().len(), 0);

    let ini = DynConfig::from_ini_opts(
        "#",
        IniOptions {
            comments: IniCommentSeparator::NumberSign,
            ..Default::default()
        },
    )
    .unwrap(); // Comment
    assert_eq!(ini.root().len(), 0);
}

#[test]
fn InvalidCharacterInSectionName() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini("[=").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterInSectionName
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini_opts(
            "[:",
            IniOptions {
                key_value_separator: IniKeyValueSeparator::Colon,
                ..IniOptions::default()
            }
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterInSectionName
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini("[a#").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInSectionName
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini_opts(
            "[a;",
            IniOptions {
                comments: IniCommentSeparator::NumberSign,
                ..IniOptions::default()
            }
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInSectionName
        }
    );
    // Any character after closing quotes.
    assert_eq!(
        DynConfig::from_ini("[\"a\" b]").err().unwrap(),
        IniError {
            line: 1,
            column: 6,
            error: IniErrorKind::InvalidCharacterInSectionName
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("[ \ta]").unwrap(); // Skipped whitespace before section name.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[a \t]").unwrap(); // Skipped whitespace after section name.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[\\=]").unwrap(); // Special character in section.
    assert_eq!(ini.root().get_table("=").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[\\:]").unwrap(); // Special character in section.
    assert_eq!(ini.root().get_table(":").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[a]").unwrap(); // Normal section.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    DynConfig::from_ini("[`~@$%^&*()_-+,<.>/?]").unwrap(); // Weird section.

    let ini = DynConfig::from_ini("[\\t\\ \\n]").unwrap(); // Escaped whitespace in section.
    assert_eq!(ini.root().get_table("\t \n").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[\\x0066\\x006f\\x006f]").unwrap(); // Unicode in section ("foo").
    assert_eq!(ini.root().get_table("foo").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[\t\"a \" ]").unwrap(); // Whitespace in quoted section name.
    assert_eq!(ini.root().get_table("a ").unwrap().len(), 0);

    let ini = DynConfig::from_ini_opts(
        "[\t\"' a'\" ]",
        IniOptions {
            string_quotes: IniStringQuote::Single | IniStringQuote::Double,
            ..Default::default()
        },
    )
    .unwrap(); // Non-matching quotes in quoted section name.
    assert_eq!(ini.root().get_table("' a'").unwrap().len(), 0);

    let ini = DynConfig::from_ini_opts(
        "[\t'a ' ]",
        IniOptions {
            string_quotes: IniStringQuote::Single,
            ..Default::default()
        },
    )
    .unwrap(); // Whitespace in quoted section name.
    assert_eq!(ini.root().get_table("a ").unwrap().len(), 0);

    let ini = DynConfig::from_ini_opts(
        "[\t'\" a\"' ]",
        IniOptions {
            string_quotes: IniStringQuote::Single | IniStringQuote::Double,
            ..Default::default()
        },
    )
    .unwrap(); // Non-matching quotes in quoted section name.
    assert_eq!(ini.root().get_table("\" a\"").unwrap().len(), 0);
}

#[test]
fn UnexpectedNewLineInSectionName() {
    // Unescaped new line at section name start.
    assert_eq!(
        DynConfig::from_ini("[\n").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedNewLineInSectionName
        }
    );
    // Unescaped new line in section name.
    assert_eq!(
        DynConfig::from_ini("[a\n").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInSectionName
        }
    );
    // Unescaped new line in quoted section name.
    assert_eq!(
        DynConfig::from_ini("[\"a\n").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInSectionName
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("[\\n]").unwrap(); // Escaped new line at section name start.
    assert_eq!(ini.root().get_table("\n").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[a\\n]").unwrap(); // Escaped new line in section name.
    assert_eq!(ini.root().get_table("a\n").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[ \"a\\n\" ]").unwrap(); // Escaped new line in quoted section name.
    assert_eq!(ini.root().get_table("a\n").unwrap().len(), 0);
}

#[test]
fn UnexpectedEndOfFileInSectionName() {
    assert_eq!(
        DynConfig::from_ini("[a").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileInSectionName
        }
    );
    assert_eq!(
        DynConfig::from_ini("[\"a").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInSectionName
        }
    );
    assert_eq!(
        DynConfig::from_ini("[\"a\"").err().unwrap(),
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
        DynConfig::from_ini("[]").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::EmptySectionName
        }
    );
    assert_eq!(
        DynConfig::from_ini("[\"\"]").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::EmptySectionName
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("[\\ ]").unwrap(); // Whitespace in section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[\\t]").unwrap(); // Whitespace in section.
    assert_eq!(ini.root().get_table("\t").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[\\n]").unwrap(); // Whitespace in section.
    assert_eq!(ini.root().get_table("\n").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[\t\" \" ]").unwrap(); // Whitespace in quoted section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);
}

#[test]
fn DuplicateSectionName() {
    // Duplicate section not supported.
    assert_eq!(
        DynConfig::from_ini_opts(
            "[a]\n[a]",
            IniOptions {
                duplicate_sections: false,
                ..Default::default()
            }
        )
        .err()
        .unwrap(),
        IniError {
            line: 2,
            column: 3,
            error: IniErrorKind::DuplicateSectionName
        }
    );

    // But this succeeds.
    let ini = DynConfig::from_ini("[a]\n[a]").unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);
}

#[test]
fn InvalidCharacterAtLineEnd() {
    // After section.
    assert_eq!(
        DynConfig::from_ini("[a] b").err().unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAtLineEnd
        }
    );
    // After value.
    assert_eq!(
        DynConfig::from_ini("a=7 b").err().unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAtLineEnd
        }
    );
    // Inline comments not supported.
    assert_eq!(
        DynConfig::from_ini("[a] ;").err().unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::InvalidCharacterAtLineEnd
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini_opts(
        "[a] ;",
        IniOptions {
            inline_comments: true,
            ..Default::default()
        },
    )
    .unwrap(); // Supported inline comment.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    let ini = DynConfig::from_ini_opts(
        "[a] #",
        IniOptions {
            comments: IniCommentSeparator::NumberSign,
            inline_comments: true,
            ..Default::default()
        },
    )
    .unwrap(); // Supported inline comment.
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);
}

#[test]
fn InvalidCharacterInKey() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini("a[").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterInKey
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(" a'").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInKey
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("a=true").unwrap(); // Normal key.
    assert_eq!(ini.root().get_bool("a").unwrap(), true);

    let ini = DynConfig::from_ini("\"a\"=true").unwrap(); // Quoted key.
    assert_eq!(ini.root().get_bool("a").unwrap(), true);

    let ini = DynConfig::from_ini_opts(
        "' a ' = false",
        IniOptions {
            string_quotes: IniStringQuote::Single,
            ..Default::default()
        },
    )
    .unwrap(); // Quoted key.
    assert_eq!(ini.root().get_bool(" a ").unwrap(), false);

    let ini = DynConfig::from_ini("a\\[=7").unwrap(); // Special character in key.
    assert_eq!(ini.root().get_i64("a[").unwrap(), 7);

    let ini = DynConfig::from_ini("a\\t=3.14").unwrap(); // Escaped whitespace in key.
    assert!(cmp_f64(ini.root().get_f64("a\t").unwrap(), 3.14));

    let ini = DynConfig::from_ini("\\x0066\\x006f\\x006f=\"bar\"").unwrap(); // Unicode in key ("foo").
    assert_eq!(ini.root().get_string("foo").unwrap(), "bar");
}

#[test]
fn UnexpectedNewLineInKey() {
    // New line in key.
    assert_eq!(
        DynConfig::from_ini("a\n").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedNewLineInKey
        }
    );
    // New line in quoted key.
    assert_eq!(
        DynConfig::from_ini("\"a\n\"").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInKey
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("a\\n=7").unwrap(); // Escaped new line in key.
    assert_eq!(ini.root().get_i64("a\n").unwrap(), 7);

    let ini = DynConfig::from_ini("a=\n").unwrap(); // Empty value.
    assert_eq!(ini.root().get_string("a").unwrap(), "");
}

#[test]
fn EmptyKey() {
    // Empty unquoted key.
    assert_eq!(
        DynConfig::from_ini(" = 7").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::EmptyKey
        }
    );
    // Empty unquoted key.
    assert_eq!(
        DynConfig::from_ini_opts(
            " : 7",
            IniOptions {
                key_value_separator: IniKeyValueSeparator::Colon,
                ..Default::default()
            }
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
        DynConfig::from_ini(" \"\" = 7").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::EmptyKey
        }
    );
    // Empty quoted key.
    assert_eq!(
        DynConfig::from_ini_opts(
            " '' : 7",
            IniOptions {
                key_value_separator: IniKeyValueSeparator::Colon,
                string_quotes: IniStringQuote::Single,
                ..Default::default()
            }
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

    let ini = DynConfig::from_ini("a = 7").unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 7);

    let ini = DynConfig::from_ini("[a]\n\" \" = 7").unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().get_i64(" ").unwrap(), 7);
}

#[test]
fn DuplicateKey() {
    // In the root.
    assert_eq!(
        DynConfig::from_ini("a=7\na=9").err().unwrap(),
        IniError {
            line: 2,
            column: 1,
            error: IniErrorKind::DuplicateKey
        }
    );
    // In the section.
    assert_eq!(
        DynConfig::from_ini("[a]\na=7\na=9").err().unwrap(),
        IniError {
            line: 3,
            column: 1,
            error: IniErrorKind::DuplicateKey
        }
    );
    // In the merged section.
    assert_eq!(
        DynConfig::from_ini("[a]\na=7\n[a]\na=9").err().unwrap(),
        IniError {
            line: 4,
            column: 1,
            error: IniErrorKind::DuplicateKey
        }
    );

    // But this succeeds.

    // In the root.
    let ini = DynConfig::from_ini_opts(
        "a=7\na=9",
        IniOptions {
            duplicate_keys: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 9);

    // In the section.
    let ini = DynConfig::from_ini_opts(
        "[a]\na=7\na=9",
        IniOptions {
            duplicate_keys: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("a").unwrap(), 9);

    // In the merged section.
    let ini = DynConfig::from_ini_opts(
        "[a]\na=7\n[a]\na=9",
        IniOptions {
            duplicate_keys: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().get_i64("a").unwrap(), 9);
}

#[test]
fn UnexpectedEndOfFileBeforeKeyValueSeparator() {
    // Unquoted key.
    assert_eq!(
        DynConfig::from_ini("a").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedEndOfFileBeforeKeyValueSeparator
        }
    );
    // Quoted key.
    assert_eq!(
        DynConfig::from_ini("\"a \"").err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedEndOfFileBeforeKeyValueSeparator
        }
    );

    // But this succeeds (empty value).

    let ini = DynConfig::from_ini("a=").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = DynConfig::from_ini_opts(
        "[a]\n\"a\":",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..IniOptions::default()
        },
    )
    .unwrap();
    assert_eq!(
        ini.root().get_table("a").unwrap().get_string("a").unwrap(),
        ""
    );
}

#[test]
fn UnexpectedCharacterInsteadOfKeyValueSeparator() {
    assert_eq!(
        DynConfig::from_ini("a !").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedCharacterInsteadOfKeyValueSeparator
        }
    );
    assert_eq!(
        DynConfig::from_ini("a :").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedCharacterInsteadOfKeyValueSeparator
        }
    );
    // Unescaped whitespace in key.
    assert_eq!(
        DynConfig::from_ini("a b = 7").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedCharacterInsteadOfKeyValueSeparator
        }
    );
    // Unexpected character after quoted key.
    assert_eq!(
        DynConfig::from_ini("\"a\" b = 7").err().unwrap(),
        IniError {
            line: 1,
            column: 5,
            error: IniErrorKind::UnexpectedCharacterInsteadOfKeyValueSeparator
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("a = 7").unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 7);

    let ini = DynConfig::from_ini_opts(
        "a : 7",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 7);
}

#[test]
fn InvalidCharacterInValue() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini("a==").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini("a=:").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini_opts(
            "a:=",
            IniOptions {
                key_value_separator: IniKeyValueSeparator::Colon,
                ..Default::default()
            }
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini_opts(
            "a::",
            IniOptions {
                key_value_separator: IniKeyValueSeparator::Colon,
                ..Default::default()
            }
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::InvalidCharacterInValue
        }
    );
    // Inline comments not supported.
    assert_eq!(
        DynConfig::from_ini("a=a;").err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidCharacterInValue
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("a=\\=").unwrap(); // Escaped special character in value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini_opts(
        "a:\\=",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..Default::default()
        },
    )
    .unwrap(); // Escaped special character in value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini("a=\"\\=\"").unwrap(); // Escaped special character in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini("a=\"=\"").unwrap(); // Unescaped special character in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini("a=\"'\"").unwrap(); // Unmatched quote in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "'");

    let ini = DynConfig::from_ini_opts(
        "a='\"'",
        IniOptions {
            string_quotes: IniStringQuote::Single,
            ..Default::default()
        },
    )
    .unwrap(); // Unmatched quote in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), "\"");

    let ini = DynConfig::from_ini_opts(
        "a=a;",
        IniOptions {
            inline_comments: true,
            ..Default::default()
        },
    )
    .unwrap(); // Supported inline comments.
    assert_eq!(ini.root().get_string("a").unwrap(), "a");

    let ini = DynConfig::from_ini_opts(
        "a=a#",
        IniOptions {
            comments: IniCommentSeparator::NumberSign,
            inline_comments: true,
            ..Default::default()
        },
    )
    .unwrap(); // Supported inline comments.
    assert_eq!(ini.root().get_string("a").unwrap(), "a");

    let ini = DynConfig::from_ini("foo=\\x0066\\x006f\\x006f").unwrap(); // Unicode in value ("foo").
    assert_eq!(ini.root().get_string("foo").unwrap(), "foo");

    let ini = DynConfig::from_ini("a=\" \"").unwrap(); // Unescaped whitespace in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), " ");
}

#[test]
fn UnexpectedEndOfFileInEscapeSequence() {
    // In section.
    assert_eq!(
        DynConfig::from_ini("[\\").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In quoted section.
    assert_eq!(
        DynConfig::from_ini("[\"\\").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In key.
    assert_eq!(
        DynConfig::from_ini("\\").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In quoted key.
    assert_eq!(
        DynConfig::from_ini("\"\\").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In unquoted value.
    assert_eq!(
        DynConfig::from_ini("a=\\").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );
    // In quoted value.
    assert_eq!(
        DynConfig::from_ini("a=\"\\").err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedEndOfFileInEscapeSequence
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("[\\ ]").unwrap(); // Escaped space in section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);

    let ini = DynConfig::from_ini("[ \"\\ \" ]").unwrap(); // Escaped space in quoted section.
    assert_eq!(ini.root().get_table(" ").unwrap().len(), 0);

    let ini = DynConfig::from_ini("\\ =").unwrap(); // Escaped space in key.
    assert_eq!(ini.root().get_string(" ").unwrap(), "");

    let ini = DynConfig::from_ini("\"\\ \" =").unwrap(); // Escaped space in quoted key.
    assert_eq!(ini.root().get_string(" ").unwrap(), "");

    let ini = DynConfig::from_ini("a = \\ ").unwrap(); // Escaped space in unquoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), " ");

    let ini = DynConfig::from_ini("a = \"\\ \"").unwrap(); // Escaped space in quoted value.
    assert_eq!(ini.root().get_string("a").unwrap(), " ");
}

#[test]
fn UnexpectedNewLineInEscapeSequence() {
    // Unsupported line continuation in section name.
    assert_eq!(
        DynConfig::from_ini("[\\\n").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInEscapeSequence
        }
    );
    // Unsupported line continuation in key.
    assert_eq!(
        DynConfig::from_ini("a\\\n").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedNewLineInEscapeSequence
        }
    );
    // Unsupported line continuation in value.
    assert_eq!(
        DynConfig::from_ini("a=\\\n").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInEscapeSequence
        }
    );

    // But this succeeds (supported line continuation).

    // Line continuation in section name.
    let ini = DynConfig::from_ini_opts(
        "[\\\na]",
        IniOptions {
            line_continuation: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_table("a").unwrap().len(), 0);

    // Line continuation in key.
    let ini = DynConfig::from_ini_opts(
        "a\\\nb = 7",
        IniOptions {
            line_continuation: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_i64("ab").unwrap(), 7);

    // Line continuation in value.
    let ini = DynConfig::from_ini_opts(
        "a = 7\\\n9",
        IniOptions {
            line_continuation: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_i64("a").unwrap(), 79);
}

#[test]
fn InvalidEscapeCharacter() {
    assert_eq!(
        DynConfig::from_ini("a=\\z").err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::InvalidEscapeCharacter
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("a=\\ ").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), " ");

    let ini = DynConfig::from_ini("a=\" \"").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), " ");

    let ini = DynConfig::from_ini("a=\\0").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\0");

    let ini = DynConfig::from_ini("a=\\a").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\x07"); // '\a'

    let ini = DynConfig::from_ini("a=\\b").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\x08"); // '\a'

    let ini = DynConfig::from_ini("a=\\t").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\t");

    let ini = DynConfig::from_ini("a=\\n").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\n");

    let ini = DynConfig::from_ini("a=\\r").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\r");

    let ini = DynConfig::from_ini("a=\\v").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\x0b"); // '\v'

    let ini = DynConfig::from_ini("a=\\f").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\x0c"); // '\f'

    let ini = DynConfig::from_ini("a=\\\\").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\\");

    let ini = DynConfig::from_ini("a=\\[").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "[");

    let ini = DynConfig::from_ini("a=\"[\"").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "[");

    let ini = DynConfig::from_ini("a=\\]").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "]");

    let ini = DynConfig::from_ini("a=\"]\"").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "]");

    let ini = DynConfig::from_ini("a=\\;").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), ";");

    let ini = DynConfig::from_ini("a=\";\"").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), ";");

    let ini = DynConfig::from_ini("a=\\#").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "#");

    let ini = DynConfig::from_ini("a=\"#\"").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "#");

    let ini = DynConfig::from_ini("a=\\=").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini("a=\"=\"").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "=");

    let ini = DynConfig::from_ini("a=\\:").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), ":");

    let ini = DynConfig::from_ini("a=\":\"").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), ":");

    let ini = DynConfig::from_ini("a=\\x00e4").unwrap(); // ä
    assert_eq!(ini.root().get_string("a").unwrap(), "ä");

    let ini = DynConfig::from_ini("a=\"\\x00e4\"").unwrap(); // ä
    assert_eq!(ini.root().get_string("a").unwrap(), "ä");
}

#[test]
fn UnexpectedEndOfFileInUnicodeEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini("a=\\x000").err().unwrap(),
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
        DynConfig::from_ini("a=\\x\n").err().unwrap(),
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
        DynConfig::from_ini("a=\\xdfff").err().unwrap(),
        IniError {
            line: 1,
            column: 8,
            error: IniErrorKind::InvalidUnicodeEscapeSequence
        }
    );
}

#[test]
fn UnexpectedNewLineInQuotedString() {
    // Unescaped newline.
    assert_eq!(
        DynConfig::from_ini("a=\"\n").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewLineInQuotedString
        }
    );

    // But this succeeds.

    // Escaped newline.
    let ini = DynConfig::from_ini("a=\\n").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\n");

    // Escaped newline in quoted string.
    let ini = DynConfig::from_ini("a=\"\\n\"").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "\n");

    // Line continuation.
    let ini = DynConfig::from_ini_opts(
        "a=\\\n",
        IniOptions {
            line_continuation: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    // Line continuation in quoted string.
    let ini = DynConfig::from_ini_opts(
        "a=\"\\\n\"",
        IniOptions {
            line_continuation: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");
}

#[test]
fn UnexpectedEndOfFileInQuotedString() {
    assert_eq!(
        DynConfig::from_ini("a=\"").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedEndOfFileInQuotedString
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("a=\"\"").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");

    let ini = DynConfig::from_ini_opts(
        "a=''",
        IniOptions {
            string_quotes: IniStringQuote::Single,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "");
}

#[test]
fn UnquotedString() {
    assert_eq!(
        DynConfig::from_ini_opts(
            "a=a",
            IniOptions {
                unquoted_strings: false,
                ..Default::default()
            }
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnquotedString
        }
    );

    // But this succeeds.

    let ini = DynConfig::from_ini("a=a").unwrap();
    assert_eq!(ini.root().get_string("a").unwrap(), "a");
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

    let config = DynConfig::from_ini(ini).unwrap();
    assert_eq!(config.root().len(), 4 + 2);

    assert_eq!(config.root().get_bool("bool").unwrap(), true);
    assert_eq!(config.root().get_i64("int").unwrap(), 7);
    assert!(cmp_f64(config.root().get_f64("float").unwrap(), 3.14));
    assert_eq!(config.root().get_string("string").unwrap(), "foo");

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
fn ArraysNotSupported() {
    let mut config = DynConfig::new();
    config
        .root_mut()
        .set("array", Value::Array(DynArray::new()))
        .unwrap();

    assert_eq!(
        config.to_ini_string(),
        Err(ToINIStringError::ArraysNotSupported)
    );
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
        Err(ToINIStringError::NestedTablesNotSupported)
    );
}

#[test]
fn from_string_and_back() {
    let ini = r#"bool = true
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

    let config = DynConfig::from_ini(ini).unwrap();

    let string = config.to_ini_string().unwrap();

    assert_eq!(ini, string);
}

#[test]
fn section_merge() {
    let ini = DynConfig::from_ini("[a]\nb = 7\n[a]\nc = 9").unwrap();

    let table = ini.root().get_table("a").unwrap();
    assert_eq!(table.len(), 2);
    assert_eq!(table.get_i64("b").unwrap(), 7);
    assert_eq!(table.get_i64("c").unwrap(), 9);
}
