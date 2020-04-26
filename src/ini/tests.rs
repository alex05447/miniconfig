#![allow(non_snake_case)]

use crate::*;

#[test]
fn InvalidCharacterAtLineStart() {
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini("=").err().unwrap(),
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
        DynConfig::from_ini(" \"").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::InvalidCharacterAtLineStart
        }
    );
    // Unescaped special character.
    assert_eq!(
        DynConfig::from_ini(
            r#"
'
    "#
        )
        .err()
        .unwrap(),
        IniError {
            line: 2,
            column: 1,
            error: IniErrorKind::InvalidCharacterAtLineStart
        }
    );

    // But this succeeds.

    DynConfig::from_ini("\\==").unwrap(); // Special character in key, empty value.
    assert_eq!(
        DynConfig::from_ini("a=").unwrap().to_ini_string().unwrap(),
        "a = \"\""
    ); // Valid key, empty value.
    DynConfig::from_ini("`~@$%^&*()_-+,<.>/? =").unwrap(); // Weird key.
    DynConfig::from_ini_opts(
        "a:",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..IniOptions::default()
        },
    )
    .unwrap(); // Valid key, empty value.
    DynConfig::from_ini("[a]").unwrap(); // Section
    DynConfig::from_ini(";").unwrap(); // Comment
    DynConfig::from_ini_opts(
        "#",
        IniOptions {
            comments: IniCommentSeparator::NumberSign,
            ..Default::default()
        },
    )
    .unwrap(); // Comment
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

    // But this succeeds.

    DynConfig::from_ini("[\\=]").unwrap(); // Special character in section.
    DynConfig::from_ini("[\\:]").unwrap(); // Special character in section.
    DynConfig::from_ini("[a]").unwrap(); // Normal section.
    DynConfig::from_ini("[`~@$%^&*()_-+,<.>/?]").unwrap(); // Weird section.
    DynConfig::from_ini("[\\t\\ \\n]").unwrap(); // Whitespace in section.
    DynConfig::from_ini("[\\x0066\\x006f\\x006f]").unwrap(); // Unicode in section ("foo").
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

    // But this succeeds.

    DynConfig::from_ini("[\\ ]").unwrap(); // Whitespace in section.
    DynConfig::from_ini("[\\t]").unwrap(); // Whitespace in section.
    DynConfig::from_ini("[\\n]").unwrap(); // Whitespace in section.
}

#[test]
fn InvalidCharacterAtLineEnd() {
    assert_eq!(
        DynConfig::from_ini("[a] a").err().unwrap(),
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

    DynConfig::from_ini_opts(
        "[a] ;",
        IniOptions {
            inline_comments: true,
            ..Default::default()
        },
    )
    .unwrap(); // Supported inline comment.
    DynConfig::from_ini_opts(
        "[a] #",
        IniOptions {
            comments: IniCommentSeparator::NumberSign,
            inline_comments: true,
            ..Default::default()
        },
    )
    .unwrap(); // Supported inline comment.
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

    // But this succeeds.
    DynConfig::from_ini("a=7").unwrap(); // Normal key.
    DynConfig::from_ini("a\\[=7").unwrap(); // Special character in key.
    DynConfig::from_ini("a\\t=7").unwrap(); // Whitespace in key.
    DynConfig::from_ini("\\x0066\\x006f\\x006f=7").unwrap(); // Unicode in key ("foo").
}

#[test]
fn UnexpectedNewlineInKey() {
    assert_eq!(
        DynConfig::from_ini("a\n").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedNewlineInKey
        }
    );

    // But this succeeds.
    DynConfig::from_ini("a\\n=7").unwrap(); // Newline in key.
    DynConfig::from_ini("a=\n").unwrap(); // Empty value.
}

#[test]
fn UnexpectedEndOfFileBeforeKeyValueSeparator() {
    assert_eq!(
        DynConfig::from_ini("a").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
            error: IniErrorKind::UnexpectedEndOfFileBeforeKeyValueSeparator
        }
    );

    assert_eq!(
        DynConfig::from_ini("a ").err().unwrap(),
        IniError {
            line: 1,
            column: 2,
            error: IniErrorKind::UnexpectedEndOfFileBeforeKeyValueSeparator
        }
    );

    // But this succeeds (empty value).

    DynConfig::from_ini("a=").unwrap();
    DynConfig::from_ini_opts(
        "a:",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..IniOptions::default()
        },
    )
    .unwrap();
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

    // But this succeeds.

    DynConfig::from_ini("a = 7").unwrap();
    DynConfig::from_ini_opts(
        "a : 7",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..Default::default()
        },
    )
    .unwrap();
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
    // Unescaped special character in quoted string.
    assert_eq!(
        DynConfig::from_ini("a=\"=\"").err().unwrap(),
        IniError {
            line: 1,
            column: 4,
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
    // Unescaped special character in quoted string.
    assert_eq!(
        DynConfig::from_ini_opts(
            "a=\':\'",
            IniOptions {
                string_quotes: IniStringQuote::Single,
                ..Default::default()
            }
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 4,
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
    // Unescaped special character in quoted string.
    assert_eq!(
        DynConfig::from_ini_opts(
            "a:\"=\"",
            IniOptions {
                key_value_separator: IniKeyValueSeparator::Colon,
                ..Default::default()
            }
        )
        .err()
        .unwrap(),
        IniError {
            line: 1,
            column: 4,
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
    // Unescaped special character in quoted string.
    assert_eq!(
        DynConfig::from_ini_opts(
            "a:\':\'",
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
            column: 4,
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

    DynConfig::from_ini("a=\\=").unwrap(); // Escaped special character in value.
    DynConfig::from_ini("a=\"\\=\"").unwrap(); // Escaped special character in quoted value.
    DynConfig::from_ini("a=\\:").unwrap(); // Special character in value.
    DynConfig::from_ini_opts(
        "a=\'\\:\'",
        IniOptions {
            string_quotes: IniStringQuote::Single,
            ..Default::default()
        },
    )
    .unwrap(); // Escaped special character in quoted value.
    DynConfig::from_ini_opts(
        "a:\\=",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..Default::default()
        },
    )
    .unwrap(); // Escaped special character in value.
    DynConfig::from_ini_opts(
        "a:\"\\=\"",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..Default::default()
        },
    )
    .unwrap(); // Escaped special character in quoted value.
    DynConfig::from_ini_opts(
        "a:\\:",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            ..Default::default()
        },
    )
    .unwrap(); // Escaped special character in value.
    DynConfig::from_ini_opts(
        "a:\'\\:\'",
        IniOptions {
            key_value_separator: IniKeyValueSeparator::Colon,
            string_quotes: IniStringQuote::Single,
            ..Default::default()
        },
    )
    .unwrap(); // Escaped special character quoted in value.
    DynConfig::from_ini_opts(
        "a=a;",
        IniOptions {
            inline_comments: true,
            ..Default::default()
        },
    )
    .unwrap(); // Supported inline comments.
    DynConfig::from_ini_opts(
        "a=a#",
        IniOptions {
            comments: IniCommentSeparator::NumberSign,
            inline_comments: true,
            ..Default::default()
        },
    )
    .unwrap(); // Supported inline comments.
    DynConfig::from_ini("foo=\\x0066\\x006f\\x006f").unwrap(); // Unicode in value ("foo").
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
    // In key.
    assert_eq!(
        DynConfig::from_ini("\\").err().unwrap(),
        IniError {
            line: 1,
            column: 1,
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

    DynConfig::from_ini("[\\ ]").unwrap(); // In section.
    DynConfig::from_ini("\\ =").unwrap(); // In key.
    DynConfig::from_ini("\\ = \\ ").unwrap(); // In unquoted value.
    DynConfig::from_ini("\\ = \\\"\\ \\\"").unwrap(); // In quoted value.
}

#[test]
fn UnexpectedNewlineInEscapeSequence() {
    // Unsupported line continuation.
    assert_eq!(
        DynConfig::from_ini("a=\\\n").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewlineInEscapeSequence
        }
    );

    // But this succeeds (supported line continuation, empty value).

    DynConfig::from_ini_opts(
        "a=\\\n",
        IniOptions {
            line_continuation: true,
            ..Default::default()
        },
    )
    .unwrap();
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
    DynConfig::from_ini("a=\\ ").unwrap();
    DynConfig::from_ini("a=\\0").unwrap();
    DynConfig::from_ini("a=\\a").unwrap();
    DynConfig::from_ini("a=\\b").unwrap();
    DynConfig::from_ini("a=\\t").unwrap();
    DynConfig::from_ini("a=\\r").unwrap();
    DynConfig::from_ini("a=\\n").unwrap();
    DynConfig::from_ini("a=\\\\").unwrap();
    DynConfig::from_ini("a=\\[").unwrap();
    DynConfig::from_ini("a=\\]").unwrap();
    DynConfig::from_ini("a=\\;").unwrap();
    DynConfig::from_ini("a=\\#").unwrap();
    DynConfig::from_ini("a=\\=").unwrap();
    DynConfig::from_ini("a=\\:").unwrap();

    assert_eq!(
        DynConfig::from_ini("a=\\x00e4")
            .unwrap()
            .root()
            .get_string("a")
            .unwrap(),
        "ä"
    );
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
fn UnexpectedNewlineInUnicodeEscapeSequence() {
    assert_eq!(
        DynConfig::from_ini("a=\\x\n").err().unwrap(),
        IniError {
            line: 1,
            column: 4,
            error: IniErrorKind::UnexpectedNewlineInUnicodeEscapeSequence
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

    // But this succeeds.

    // In the root.
    DynConfig::from_ini_opts(
        "a=7\na=9",
        IniOptions {
            duplicate_keys: true,
            ..Default::default()
        },
    )
    .unwrap();

    // In the section.
    DynConfig::from_ini_opts(
        "[a]\na=7\na=9",
        IniOptions {
            duplicate_keys: true,
            ..Default::default()
        },
    )
    .unwrap();
}

#[test]
fn UnexpectedNewlineInQuotedString() {
    // Unescaped newline.
    assert_eq!(
        DynConfig::from_ini("a=\"\n").err().unwrap(),
        IniError {
            line: 1,
            column: 3,
            error: IniErrorKind::UnexpectedNewlineInQuotedString
        }
    );

    // But this succeeds (escaped newline).
    DynConfig::from_ini_opts(
        "a=\"\\\n\"",
        IniOptions {
            line_continuation: true,
            ..Default::default()
        },
    )
    .unwrap();
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
    DynConfig::from_ini("a=\"\"").unwrap();
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
    DynConfig::from_ini("a=a").unwrap();
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

[other_section]
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

    let other_section = config.root().get_table("other_section").unwrap();
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

[other_section]
other_bool = true
other_float = 3.14
other_int = 7
other_string = "foo bar"

[section]
bool = false
float = 7.62
int = 9
string = "bar""#;

    let config = DynConfig::from_ini(ini).unwrap();

    let string = config.to_ini_string().unwrap();

    assert_eq!(ini, string);
}
