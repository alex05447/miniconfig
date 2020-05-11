use crate::*;

fn cmp_f64(l: f64, r: f64) -> bool {
    (l - r).abs() < 0.000_001
}

#[test]
fn basic_table() {
    // Create an empty config.
    let mut config = DynConfig::new();
    assert_eq!(config.root().len(), 0);

    let mut root = config.root_mut();

    // Empty key.
    assert_eq!(
        root.set("", Value::Bool(true)),
        Err(DynTableSetError::EmptyKey)
    );

    // Add a value.
    assert!(!root.contains("bool"));
    root.set("bool", Value::Bool(true)).unwrap();
    assert_eq!(root.len(), 1);
    assert!(root.contains("bool"));
    assert_eq!(root.get_bool("bool").unwrap(), true);

    // Add a couple more.
    assert!(!root.contains("i64"));
    root.set("i64", Value::I64(7)).unwrap();
    assert_eq!(root.len(), 2);
    assert!(root.contains("i64"));
    assert_eq!(root.get_i64("i64").unwrap(), 7);

    assert!(!root.contains("string"));
    root.set("string", Value::String("foo".into())).unwrap();
    assert_eq!(root.len(), 3);
    assert!(root.contains("string"));
    assert_eq!(root.get_string("string").unwrap(), "foo");

    // Change a value.
    root.set("string", Value::String("bar".into())).unwrap();
    assert_eq!(root.len(), 3);
    assert!(root.contains("string"));
    assert_eq!(root.get_string("string").unwrap(), "bar");

    // Try to remove a nonexistent value.
    assert!(!root.contains("missing"));
    assert_eq!(
        root.set("missing", None),
        Err(DynTableSetError::KeyDoesNotExist)
    );

    // Remove a value.
    root.set("bool", None).unwrap();
    assert_eq!(root.len(), 2);
    assert!(!root.contains("bool"));
    match root.get("bool") {
        Err(DynTableGetError::KeyDoesNotExist) => {}
        _ => panic!("Expected an error."),
    };

    // Add a nested table.
    let mut nested_table = DynTable::new();
    assert!(!nested_table.contains("nested_bool"));
    assert!(!nested_table.contains("nested_int"));
    nested_table.set("nested_bool", Value::Bool(false)).unwrap();
    nested_table.set("nested_int", Value::I64(-9)).unwrap();
    assert_eq!(nested_table.len(), 2);
    assert!(nested_table.contains("nested_bool"));
    assert!(nested_table.contains("nested_int"));

    root.set("table", Value::Table(nested_table)).unwrap();
    assert_eq!(root.len(), 3);
    assert!(root.contains("table"));

    assert_eq!(
        root.get_path(["missing", "nested_missing"].iter())
            .err()
            .unwrap(),
        DynTableGetPathError::PathDoesNotExist(vec!["missing".into()])
    );
    assert_eq!(
        root.get_path(["table", "nested_missing"].iter())
            .err()
            .unwrap(),
        DynTableGetPathError::PathDoesNotExist(vec!["table".into(), "nested_missing".into()])
    );
    assert_eq!(
        root.get_path(["table", "nested_bool", "nested_missing"].iter())
            .err()
            .unwrap(),
        DynTableGetPathError::ValueNotATable {
            path: vec!["table".into(), "nested_bool".into()],
            value_type: ValueType::Bool
        }
    );
    assert_eq!(
        root.get_i64_path(["table", "nested_bool"].iter())
            .err()
            .unwrap(),
        DynTableGetPathError::IncorrectValueType(ValueType::Bool)
    );
    assert_eq!(
        root.get_path(["table", "nested_bool"].iter())
            .unwrap()
            .bool()
            .unwrap(),
        false
    );
    assert_eq!(
        root.get_bool_path(["table", "nested_bool"].iter()).unwrap(),
        false
    );

    // Add a nested array.
    let mut nested_array = DynArray::new();

    nested_array.push(Value::F64(3.14)).unwrap();
    nested_array.push(Value::F64(42.0)).unwrap();
    nested_array.push(Value::F64(-17.235)).unwrap();
    assert_eq!(nested_array.len(), 3);

    assert!(!root.contains("array"));
    root.set("array", Value::Array(nested_array)).unwrap();
    assert_eq!(root.len(), 4);
    assert!(root.contains("array"));

    // Iterate the table.
    for (key, value) in root.iter() {
        match key {
            "i64" => assert_eq!(value.i64().unwrap(), 7),
            "string" => assert_eq!(value.string().unwrap(), "bar"),
            "table" => {
                // Iterate the nested table.
                let nested_table = value.table().unwrap();

                for (key, value) in nested_table.iter() {
                    match key {
                        "nested_bool" => assert_eq!(value.bool().unwrap(), false),
                        "nested_int" => assert_eq!(value.i64().unwrap(), -9),
                        _ => panic!("Invalid key."),
                    }
                }
            }
            "array" => {
                // Iterate the nested array.
                let nested_array = value.array().unwrap();

                for (index, value) in nested_array.iter().enumerate() {
                    match index {
                        0 => assert!(cmp_f64(value.f64().unwrap(), 3.14)),
                        1 => assert!(cmp_f64(value.f64().unwrap(), 42.0)),
                        2 => assert!(cmp_f64(value.f64().unwrap(), -17.235)),
                        _ => panic!("Invalid index."),
                    }
                }
            }
            _ => panic!("Invalid key."),
        }
    }
}

#[test]
fn basic_array() {
    // Create an empty config.
    let mut config = DynConfig::new();

    // Add an array.
    let mut array = DynArray::new();
    assert_eq!(array.len(), 0);

    // Try to get a value.
    match array.get(0) {
        Err(DynArrayGetError::IndexOutOfBounds(0)) => {}
        _ => panic!("Expected an error."),
    };

    // Try to pop a value.
    match array.pop() {
        Err(DynArrayGetError::ArrayEmpty) => {}
        _ => panic!("Expected an error."),
    };

    // Try to set a value.
    match array.set(0, Value::Bool(true)) {
        Err(DynArraySetError::IndexOutOfBounds(0)) => {}
        _ => panic!("Expected an error."),
    };

    // Make it a bool array.
    array.push(Value::Bool(true)).unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array.get_bool(0).unwrap(), true);

    // Try to push an int.
    match array.push(Value::I64(7)) {
        Err(DynArraySetError::InvalidValueType(ValueType::Bool)) => {}
        _ => panic!("Expected an error."),
    };

    // Try to push an float.
    match array.push(Value::F64(3.14)) {
        Err(DynArraySetError::InvalidValueType(ValueType::Bool)) => {}
        _ => panic!("Expected an error."),
    };

    // Try to push a string.
    match array.push(Value::String("foo".into())) {
        Err(DynArraySetError::InvalidValueType(ValueType::Bool)) => {}
        _ => panic!("Expected an error."),
    };

    // Try to push an array.
    match array.push(Value::Array(DynArray::new())) {
        Err(DynArraySetError::InvalidValueType(ValueType::Bool)) => {}
        _ => panic!("Expected an error."),
    };

    // Try to push a table.
    match array.push(Value::Table(DynTable::new())) {
        Err(DynArraySetError::InvalidValueType(ValueType::Bool)) => {}
        _ => panic!("Expected an error."),
    };

    // Push a bool.
    array.push(Value::Bool(false)).unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array.get_bool(0).unwrap(), true);
    assert_eq!(array.get_bool(1).unwrap(), false);

    // Clear it.
    assert_eq!(array.pop().unwrap().bool().unwrap(), false);
    assert_eq!(array.len(), 1);
    assert_eq!(array.pop().unwrap().bool().unwrap(), true);
    assert_eq!(array.len(), 0);

    // Now push an int and make it an int array.
    array.push(Value::I64(7)).unwrap();
    assert_eq!(array.len(), 1);
    assert_eq!(array.get_i64(0).unwrap(), 7);

    // Try to push a bool.
    match array.push(Value::Bool(true)) {
        Err(DynArraySetError::InvalidValueType(ValueType::I64)) => {}
        _ => panic!("Expected an error."),
    };

    // Try to push a string.
    match array.push(Value::String("foo".into())) {
        Err(DynArraySetError::InvalidValueType(ValueType::I64)) => {}
        _ => panic!("Expected an error."),
    };

    // Try to push an array.
    match array.push(Value::Array(DynArray::new())) {
        Err(DynArraySetError::InvalidValueType(ValueType::I64)) => {}
        _ => panic!("Expected an error."),
    };

    // Try to push a table.
    match array.push(Value::Table(DynTable::new())) {
        Err(DynArraySetError::InvalidValueType(ValueType::I64)) => {}
        _ => panic!("Expected an error."),
    };

    // Push a float.
    array.push(Value::F64(3.14)).unwrap();
    assert_eq!(array.len(), 2);
    assert!(cmp_f64(array.get_f64(1).unwrap(), 3.14));

    // Push another int.
    array.push(Value::I64(-9)).unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get_i64(2).unwrap(), -9);

    // Iterate an array.
    for (index, value) in array.iter().enumerate() {
        match index {
            0 => assert_eq!(value.i64().unwrap(), 7),
            1 => assert!(cmp_f64(value.f64().unwrap(), 3.14)),
            2 => assert_eq!(value.i64().unwrap(), -9),
            _ => panic!("Invalid index."),
        }
    }

    config.root_mut().set("array", Value::Array(array)).unwrap();

    // Array of arrays.
    let mut array = DynArray::new();

    for _ in 0..3 {
        let mut nested_array = DynArray::new();

        for _ in 0..3 {
            nested_array.push(Value::Bool(true)).unwrap();
        }
        assert_eq!(nested_array.len(), 3);

        array.push(Value::Array(nested_array)).unwrap();
    }
    assert_eq!(array.len(), 3);

    config
        .root_mut()
        .set("another_array", Value::Array(array))
        .unwrap();

    // Validate the arrays.
    let array = config.root().get_array("array").unwrap();

    for (index, value) in array.iter().enumerate() {
        match index {
            0 => assert_eq!(value.i64().unwrap(), 7),
            1 => assert!(cmp_f64(value.f64().unwrap(), 3.14)),
            2 => assert_eq!(value.i64().unwrap(), -9),
            _ => panic!("Invalid index."),
        }
    }

    let another_array = config.root().get_array("another_array").unwrap();

    for value in another_array.iter() {
        let nested_array = value.array().unwrap();

        for value in nested_array.iter() {
            assert_eq!(value.bool().unwrap(), true);
        }
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

#[cfg(feature = "bin")]
#[test]
fn bin_config() {
    let mut config = DynConfig::new();

    let mut root = config.root_mut();

    let mut array_value = DynArray::new();

    array_value.push(Value::I64(54)).unwrap();
    array_value.push(Value::I64(12)).unwrap();
    array_value.push(Value::F64(78.9)).unwrap();

    root.set("array_value", Value::Array(array_value)).unwrap();

    root.set("bool_value", Value::Bool(true)).unwrap();

    root.set("float_value", Value::F64(3.14)).unwrap();

    root.set("int_value", Value::I64(7)).unwrap();

    root.set("string_value", Value::String("foo".into()))
        .unwrap();

    let mut table_value = DynTable::new();

    table_value.set("bar", Value::I64(2020)).unwrap();
    table_value
        .set("baz", Value::String("hello".into()))
        .unwrap();
    table_value.set("foo", Value::Bool(false)).unwrap();

    root.set("table_value", Value::Table(table_value)).unwrap();

    // Serialize to binary config.
    let data = config.to_bin_config().unwrap();

    // Load the binary config.
    let config = BinConfig::new(data).unwrap();

    let array_value = config.root().get_array("array_value").unwrap();

    assert_eq!(array_value.len(), 3);
    assert_eq!(array_value.get_i64(0).unwrap(), 54);
    assert!(cmp_f64(array_value.get_f64(0).unwrap(), 54.0));
    assert_eq!(array_value.get_i64(1).unwrap(), 12);
    assert!(cmp_f64(array_value.get_f64(1).unwrap(), 12.0));
    assert_eq!(array_value.get_i64(2).unwrap(), 78);
    assert!(cmp_f64(array_value.get_f64(2).unwrap(), 78.9));

    assert_eq!(config.root().get_bool("bool_value").unwrap(), true);

    assert!(cmp_f64(config.root().get_f64("float_value").unwrap(), 3.14));

    assert_eq!(config.root().get_i64("int_value").unwrap(), 7);

    assert_eq!(config.root().get_string("string_value").unwrap(), "foo");

    let table_value = config.root().get_table("table_value").unwrap();

    assert_eq!(table_value.len(), 3);
    assert_eq!(table_value.get_i64("bar").unwrap(), 2020);
    assert!(cmp_f64(table_value.get_f64("bar").unwrap(), 2020.0));
    assert_eq!(table_value.get_string("baz").unwrap(), "hello");
    assert_eq!(table_value.get_bool("foo").unwrap(), false);
}

#[cfg(feature = "ini")]
#[test]
fn to_ini_string() {
    let ini = r#"array = ["foo", "bar", "baz"]
bool = true
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

    let mut config = DynConfig::new();

    let mut array = DynArray::new();

    array.push(Value::String("foo".into())).unwrap();
    array.push(Value::String("bar".into())).unwrap();
    array.push(Value::String("baz".into())).unwrap();

    config.root_mut().set("array", Value::Array(array)).unwrap();

    config.root_mut().set("bool", Value::Bool(true)).unwrap();
    config.root_mut().set("float", Value::F64(3.14)).unwrap();
    config.root_mut().set("int", Value::I64(7)).unwrap();
    config
        .root_mut()
        .set("string", Value::String("foo".into()))
        .unwrap();

    let mut other_section = DynTable::new();

    other_section.set("other_bool", Value::Bool(true)).unwrap();
    other_section.set("other_float", Value::F64(3.14)).unwrap();
    other_section.set("other_int", Value::I64(7)).unwrap();
    other_section
        .set("other_string", Value::String("foo".into()))
        .unwrap();

    config
        .root_mut()
        .set("other_section", Value::Table(other_section))
        .unwrap();

    let mut section = DynTable::new();

    section.set("bool", Value::Bool(false)).unwrap();
    section.set("float", Value::F64(7.62)).unwrap();
    section.set("int", Value::I64(9)).unwrap();
    section.set("string", Value::String("bar".into())).unwrap();

    config
        .root_mut()
        .set("section", Value::Table(section))
        .unwrap();

    let string = config
        .to_ini_string_opts(ToIniStringOptions {
            arrays: true,
            ..Default::default()
        })
        .unwrap();

    assert_eq!(string, ini);
}
