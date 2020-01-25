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
        assert_eq!(writer.bool("bool", true).err().unwrap(), ArrayKeyNotRequired);
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
        assert_eq!(writer.finish().err().unwrap(), ArrayOrTableLengthMismatch{ expected: 1, found: 0 });
    }

    // ArrayOrTableLengthMismatch
    // Overflow, root table.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.bool(Some("bool_0"), true).unwrap();
        assert_eq!(writer.bool(Some("bool_1"), true).err().unwrap(), BinConfigWriterError::ArrayOrTableLengthMismatch{ expected: 1, found: 2 });
    }

    // ArrayOrTableLengthMismatch
    // Overflow, nested table.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.table(Some("table"), 1).unwrap();
        writer.bool(Some("bool_0"), true).unwrap();
        assert_eq!(writer.bool(Some("bool_1"), true).err().unwrap(), BinConfigWriterError::ArrayOrTableLengthMismatch{ expected: 1, found: 2 });
    }

    // ArrayOrTableLengthMismatch
    // Underflow, nested table.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.table(Some("table"), 2).unwrap();
        writer.bool(Some("bool_0"), true).unwrap();
        assert_eq!(writer.end().err().unwrap(), BinConfigWriterError::ArrayOrTableLengthMismatch{ expected: 2, found: 1 });
    }

    // ArrayOrTableLengthMismatch
    // Overflow, nested array.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array(Some("array"), 1).unwrap();
        writer.bool(None, true).unwrap();
        assert_eq!(writer.bool(None, true).err().unwrap(), BinConfigWriterError::ArrayOrTableLengthMismatch{ expected: 1, found: 2 });
    }

    // ArrayOrTableLengthMismatch
    // Underflow, nested array.
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array(Some("array"), 2).unwrap();
        writer.bool(None, true).unwrap();
        assert_eq!(writer.end().err().unwrap(), BinConfigWriterError::ArrayOrTableLengthMismatch{ expected: 2, found: 1 });
    }

    // EndCallMismatch
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.bool(Some("bool"), true).unwrap();
        assert_eq!(writer.end().err().unwrap(), BinConfigWriterError::EndCallMismatch);
    }

    // UnfinishedArraysOrTables
    {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array(Some("array"), 1).unwrap();
        assert_eq!(writer.finish().err().unwrap(), BinConfigWriterError::UnfinishedArraysOrTables(1));
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

    let array_value = root.get("array_value").unwrap().array().unwrap();

    assert_eq!(array_value.len(), 3);
    assert_eq!(array_value.get(0).unwrap().i64().unwrap(), 54);
    assert!(cmp_f64(array_value.get(0).unwrap().f64().unwrap(), 54.0));
    assert_eq!(array_value.get(1).unwrap().i64().unwrap(), 12);
    assert!(cmp_f64(array_value.get(1).unwrap().f64().unwrap(), 12.0));
    assert_eq!(array_value.get(2).unwrap().i64().unwrap(), 78);
    assert!(cmp_f64(array_value.get(2).unwrap().f64().unwrap(), 78.9));

    assert_eq!(root.get("bool_value").unwrap().bool().unwrap(), true);

    assert!(cmp_f64(root.get("float_value").unwrap().f64().unwrap(), 3.14));

    assert_eq!(root.get("int_value").unwrap().i64().unwrap(), 7);

    assert_eq!(root.get("string_value").unwrap().string().unwrap(), "foo");

    let table_value = root.get("table_value").unwrap().table().unwrap();

    assert_eq!(table_value.len(), 3);
    assert_eq!(table_value.get("bar").unwrap().i64().unwrap(), 2020);
    assert!(cmp_f64(table_value.get("bar").unwrap().f64().unwrap(), 2020.0));
    assert_eq!(table_value.get("baz").unwrap().string().unwrap(), "hello");
    assert_eq!(table_value.get("foo").unwrap().bool().unwrap(), false);
}