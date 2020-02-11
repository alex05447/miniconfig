use crate::*;

fn cmp_f64(l: f64, r: f64) -> bool {
    (l - r).abs() < 0.000_001
}

#[test]
fn writer_errors() {
    use BinConfigWriterError::*;

    // EmptyRootTable
    {
        assert_eq!(BinConfigWriter::new(0).err().unwrap(), EmptyRootTable);
    }

    // TableKeyRequired
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        assert_eq!(writer.bool(None, true).err().unwrap(), TableKeyRequired);
    }

    // ArrayKeyNotRequired
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array("array", 1).unwrap();
        assert_eq!(
            writer.bool("bool", true).err().unwrap(),
            ArrayKeyNotRequired
        );
    }

    // MixedArray
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array("array", 2).unwrap();
        writer.bool(None, true).unwrap();
        assert_eq!(
            writer.i64(None, 7).err().unwrap(),
            MixedArray {
                expected: ValueType::Bool,
                found: ValueType::I64
            }
        );
    }

    // NonUniqueKey
    {
        let mut writer = BinConfigWriter::new(2).unwrap();
        writer.bool("bool", true).unwrap();
        assert_eq!(writer.bool("bool", true).err().unwrap(), NonUniqueKey);
    }

    // ArrayOrTableLengthMismatch
    // Underflow, root table.
    {
        let writer = BinConfigWriter::new(1).unwrap();
        assert_eq!(
            writer.finish().err().unwrap(),
            ArrayOrTableLengthMismatch {
                expected: 1,
                found: 0
            }
        );
    }

    // ArrayOrTableLengthMismatch
    // Overflow, root table.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.bool(Some("bool_0"), true).unwrap();
        assert_eq!(
            writer.bool(Some("bool_1"), true).err().unwrap(),
            BinConfigWriterError::ArrayOrTableLengthMismatch {
                expected: 1,
                found: 2
            }
        );
    }

    // ArrayOrTableLengthMismatch
    // Overflow, nested table.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.table(Some("table"), 1).unwrap();
        writer.bool(Some("bool_0"), true).unwrap();
        assert_eq!(
            writer.bool(Some("bool_1"), true).err().unwrap(),
            BinConfigWriterError::ArrayOrTableLengthMismatch {
                expected: 1,
                found: 2
            }
        );
    }

    // ArrayOrTableLengthMismatch
    // Underflow, nested table.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.table(Some("table"), 2).unwrap();
        writer.bool(Some("bool_0"), true).unwrap();
        assert_eq!(
            writer.end().err().unwrap(),
            BinConfigWriterError::ArrayOrTableLengthMismatch {
                expected: 2,
                found: 1
            }
        );
    }

    // ArrayOrTableLengthMismatch
    // Overflow, nested array.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array(Some("array"), 1).unwrap();
        writer.bool(None, true).unwrap();
        assert_eq!(
            writer.bool(None, true).err().unwrap(),
            BinConfigWriterError::ArrayOrTableLengthMismatch {
                expected: 1,
                found: 2
            }
        );
    }

    // ArrayOrTableLengthMismatch
    // Underflow, nested array.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array(Some("array"), 2).unwrap();
        writer.bool(None, true).unwrap();
        assert_eq!(
            writer.end().err().unwrap(),
            BinConfigWriterError::ArrayOrTableLengthMismatch {
                expected: 2,
                found: 1
            }
        );
    }

    // EndCallMismatch
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.bool(Some("bool"), true).unwrap();
        assert_eq!(
            writer.end().err().unwrap(),
            BinConfigWriterError::EndCallMismatch
        );
    }

    // UnfinishedArraysOrTables
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array(Some("array"), 1).unwrap();
        assert_eq!(
            writer.finish().err().unwrap(),
            BinConfigWriterError::UnfinishedArraysOrTables(1)
        );
    }
}

// "array_value = { 54, 12, 78.9 } -- array_value
// bool_value = true
// float_value = 3.14
// int_value = 7
// string_value = \"foo\"
// table_value = {
// \tbar = 2020,
// \tbaz = \"hello\",
// \tfoo = false,
// } -- table_value";

#[test]
fn writer() {
    let mut writer = BinConfigWriter::new(6).unwrap();

    writer.array("array_value", 3).unwrap();
    writer.i64(None, 54).unwrap();
    writer.i64(None, 12).unwrap();
    writer.f64(None, 78.9).unwrap();
    writer.end().unwrap();

    writer.bool("bool_value", true).unwrap();
    writer.f64("float_value", 3.14).unwrap();
    writer.i64("int_value", 7).unwrap();
    writer.string("string_value", "foo").unwrap();

    writer.table("table_value", 3).unwrap();
    writer.i64("bar", 2020).unwrap();
    writer.string("baz", "hello").unwrap();
    writer.bool("foo", false).unwrap();
    writer.end().unwrap();

    let data = writer.finish().unwrap();

    let config = BinConfig::new(data).unwrap();

    let root = config.root();

    let array_value = root.get_array("array_value").unwrap();

    assert_eq!(array_value.len(), 3);
    assert_eq!(array_value.get_i64(0).unwrap(), 54);
    assert!(cmp_f64(array_value.get_f64(0).unwrap(), 54.0));
    assert_eq!(array_value.get_i64(1).unwrap(), 12);
    assert!(cmp_f64(array_value.get_f64(1).unwrap(), 12.0));
    assert_eq!(array_value.get_i64(2).unwrap(), 78);
    assert!(cmp_f64(array_value.get_f64(2).unwrap(), 78.9));

    assert_eq!(root.get_bool("bool_value").unwrap(), true);

    assert!(cmp_f64(root.get_f64("float_value").unwrap(), 3.14));

    assert_eq!(root.get_i64("int_value").unwrap(), 7);

    assert_eq!(root.get_string("string_value").unwrap(), "foo");

    let table_value = root.get_table("table_value").unwrap();

    assert_eq!(table_value.len(), 3);
    assert_eq!(table_value.get_i64("bar").unwrap(), 2020);
    assert!(cmp_f64(table_value.get_f64("bar").unwrap(), 2020.0));
    assert_eq!(table_value.get_string("baz").unwrap(), "hello");
    assert_eq!(table_value.get_bool("foo").unwrap(), false);
}

#[cfg(feature = "ini")]
#[test]
fn to_ini_string() {
    let ini = r#"bool = true
float = 3.14
int = 7
string = "foo"

[other_section]
other_bool = true
other_float = 3.14
other_int = 7
other_string = "foo"

[section]
bool = false
float = 7.62
int = 9
string = "bar""#;

    let mut writer = BinConfigWriter::new(6).unwrap();

    writer.bool("bool", true).unwrap();
    writer.f64("float", 3.14).unwrap();
    writer.i64("int", 7).unwrap();
    writer.string("string", "foo").unwrap();

    writer.table("other_section", 4).unwrap();
    writer.bool("other_bool", true).unwrap();
    writer.f64("other_float", 3.14).unwrap();
    writer.i64("other_int", 7).unwrap();
    writer.string("other_string", "foo").unwrap();
    writer.end().unwrap();

    writer.table("section", 4).unwrap();
    writer.bool("bool", false).unwrap();
    writer.f64("float", 7.62).unwrap();
    writer.i64("int", 9).unwrap();
    writer.string("string", "bar").unwrap();
    writer.end().unwrap();

    let data = writer.finish().unwrap();

    let config = BinConfig::new(data).unwrap();

    let string = config.to_ini_string().unwrap();

    assert_eq!(string, ini);
}