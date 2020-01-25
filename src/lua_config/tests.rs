use crate::*;

use rlua;
use rlua_ext;

fn cmp_f64(l: f64, r: f64) -> bool {
    (l - r).abs() < 0.000_001
}

fn assert_script_error<'lua, F: Fn(LuaConfigError)>(
    lua: rlua::Context<'lua>,
    script: &str,
    handler: F,
) {
    match LuaConfigKey::from_script(lua, script) {
        Err(err) => {
            handler(err);
        }
        Ok(_) => panic!("Expected an error."),
    }
}

#[test]
fn script_errors() {
    let lua = rlua::Lua::new();

    lua.context(|lua| {
        assert_script_error(lua, r#" ?!#>& "#, |err| {
            if let LuaConfigError::LuaScriptError(_) = err {
            } else {
                panic!("Wrong error.");
            }
        });

        assert_script_error(
            lua,
            r#"
                table = {
                    foo = true,
                    [1] = 7,
                }
            "#,
            |err| {
                if let LuaConfigError::MixedKeys(path) = err {
                    assert_eq!(path, "table")
                } else {
                    panic!("Wrong error.");
                }
            },
        );

        assert_script_error(
            lua,
            r#"
                array = {
                    true,
                    7,
                    3.14,
                }
            "#,
            |err| {
                if let LuaConfigError::MixedArray {
                    path,
                    expected,
                    found,
                } = err
                {
                    assert_eq!(path, "array.1");
                    assert_eq!(expected, rlua_ext::ValueType::Boolean);
                    assert_eq!(found, rlua_ext::ValueType::Integer);
                } else {
                    panic!("Wrong error.");
                }
            },
        );

        // But this should be fine.
        LuaConfigKey::from_script(
            lua,
            r#"
                array = {
                    -24,
                    7,
                    3.14,
                }
            "#,
        )
        .unwrap();

        assert_script_error(
            lua,
            r#"
                table = {}
                local key = {}
                table[key] = 7
            "#,
            |err| {
                if let LuaConfigError::InvalidKeyType { path, invalid_type } = err {
                    assert_eq!(path, "table");
                    assert_eq!(invalid_type, rlua_ext::ValueType::Table);
                } else {
                    panic!("Wrong error.");
                }
            },
        );

        assert_script_error(
            lua,
            r#"
                table = {
                    table_2 = {}
                }
                local key = {}
                table.table_2[key] = 7
            "#,
            |err| {
                if let LuaConfigError::InvalidKeyType { path, invalid_type } = err {
                    assert_eq!(path, "table.table_2");
                    assert_eq!(invalid_type, rlua_ext::ValueType::Table);
                } else {
                    panic!("Wrong error.");
                }
            },
        );

        assert_script_error(
            lua,
            r#"
                table = {
                    ["\xc0"] = 7
                }
            "#,
            |err| {
                if let LuaConfigError::InvalidKeyUTF8 { path, .. } = err {
                    assert_eq!(path, "table");
                } else {
                    panic!("Wrong error.");
                }
            },
        );

        assert_script_error(
            lua,
            r#"
                table = {
                    [0] = 7
                }
            "#,
            |err| {
                if let LuaConfigError::InvalidArrayIndex(path) = err {
                    assert_eq!(path, "table");
                } else {
                    panic!("Wrong error.");
                }
            },
        );

        assert_script_error(
            lua,
            r#"
                table = {
                    invalid = function () end
                }
            "#,
            |err| {
                if let LuaConfigError::InvalidValueType { path, invalid_type } = err {
                    assert_eq!(path, "table.invalid");
                    assert_eq!(invalid_type, rlua_ext::ValueType::Function);
                } else {
                    panic!("Wrong error.");
                }
            },
        );

        assert_script_error(
            lua,
            r#"
                table = {
                    string = "\xc0"
                }
            "#,
            |err| {
                if let LuaConfigError::InvalidValueUTF8 { path, .. } = err {
                    assert_eq!(path, "table.string");
                } else {
                    panic!("Wrong error.");
                }
            },
        );
    });
}

const SCRIPT: &str = "array_value = { 54, 12, 78.9 } -- array_value
bool_value = true
float_value = 3.14
int_value = 7
string_value = \"foo\"
table_value = {
\tbar = 2020,
\tbaz = \"hello\",
\tfoo = false,
} -- table_value";

#[test]
fn from_script_and_back() {
    let lua = rlua::Lua::new();

    lua.context(|lua| {
        // Load from script.
        let config = LuaConfig::from_script(lua, SCRIPT).unwrap();

        // Serialize to string.
        let string = config.to_string();

        assert_eq!(SCRIPT, string, "Script and serialized config mismatch.");
    });
}

#[test]
fn basic_table() {
    let lua = rlua::Lua::new();

    let key = lua.context(|lua| {
        // Create an empty config.
        let config = LuaConfig::new(lua);
        assert_eq!(config.root().len(), 0);

        let mut root = config.root();

        // Add a value.
        root.set("bool", Value::Bool(true)).unwrap();
        assert_eq!(root.len(), 1);
        assert_eq!(root.get("bool").unwrap().bool().unwrap(), true);

        // Add a couple more.
        root.set("i64", Value::I64(7)).unwrap();
        assert_eq!(root.len(), 2);
        assert_eq!(root.get("i64").unwrap().i64().unwrap(), 7);

        root.set("string", Value::String("foo")).unwrap();
        assert_eq!(root.len(), 3);
        assert_eq!(root.get("string").unwrap().string().unwrap(), "foo");

        // Change a value.
        root.set("string", Value::String("bar")).unwrap();
        assert_eq!(root.len(), 3);
        assert_eq!(root.get("string").unwrap().string().unwrap(), "bar");

        // Try to remove a nonexistent value.
        assert_eq!(
            root.set("missing", None),
            Err(LuaTableSetError::KeyDoesNotExist)
        );

        // Remove a value.
        root.set("bool", None).unwrap();
        assert_eq!(root.len(), 2);
        match root.get("bool") {
            Err(LuaTableGetError::KeyDoesNotExist) => {}
            _ => panic!("Expected an error."),
        };

        // Add a nested table.
        let mut nested_table = LuaTable::new(lua);
        nested_table.set("nested_bool", Value::Bool(false)).unwrap();
        nested_table.set("nested_int", Value::I64(-9)).unwrap();
        assert_eq!(nested_table.len(), 2);

        root.set("table", Value::Table(nested_table)).unwrap();
        assert_eq!(root.len(), 3);

        // Add a nested array.
        let mut nested_array = LuaArray::new(lua);

        nested_array.push(Value::F64(3.14)).unwrap();
        nested_array.push(Value::F64(42.0)).unwrap();
        nested_array.push(Value::F64(-17.235)).unwrap();
        assert_eq!(nested_array.len(), 3);

        root.set("array", Value::Array(nested_array)).unwrap();
        assert_eq!(root.len(), 4);

        // Iterate the table.
        for (key, value) in root.iter() {
            match key.as_ref() {
                "i64" => assert_eq!(value.i64().unwrap(), 7),
                "string" => assert_eq!(value.string().unwrap(), "bar"),
                "table" => {
                    // Iterate the nested table.
                    let nested_table = value.table().unwrap();

                    for (key, value) in nested_table.iter() {
                        match key.as_ref() {
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

        // Create a registry key from the config.
        config.key(lua)
    });

    // Attempt to use the config with the wrong Lua state.
    let another_lua = rlua::Lua::new();

    another_lua.context(|lua| match key.config(lua) {
        Err(LuaConfigKeyError::LuaStateMismatch) => {}
        _ => panic!("Expected an error."),
    });

    // Destroy the config key.
    lua.context(|lua| {
        key.destroy(lua).unwrap();
    });
}

#[test]
fn basic_array() {
    let lua = rlua::Lua::new();

    lua.context(|lua| {
        // Create an empty config.
        let config = LuaConfig::new(lua);

        // Add an array.
        let mut array = LuaArray::new(lua);
        assert_eq!(array.len(), 0);

        // Try to get a value.
        match array.get(0) {
            Err(LuaArrayGetError::IndexOutOfBounds(0)) => {}
            _ => panic!("Expected an error."),
        };

        // Try to pop a value.
        match array.pop() {
            Err(LuaArrayGetError::ArrayEmpty) => {}
            _ => panic!("Expected an error."),
        };

        // Try to set a value.
        match array.set(0, Value::Bool(true)) {
            Err(LuaArraySetError::IndexOutOfBounds(0)) => {}
            _ => panic!("Expected an error."),
        };

        // Make it a bool array.
        array.push(Value::Bool(true)).unwrap();
        assert_eq!(array.len(), 1);
        assert_eq!(array.get(0).unwrap().bool().unwrap(), true);

        // Try to push an int.
        match array.push(Value::I64(7)) {
            Err(LuaArraySetError::InvalidValueType(ValueType::Bool)) => {}
            _ => panic!("Expected an error."),
        };

        // Try to push an float.
        match array.push(Value::F64(3.14)) {
            Err(LuaArraySetError::InvalidValueType(ValueType::Bool)) => {}
            _ => panic!("Expected an error."),
        };

        // Try to push a string.
        match array.push(Value::String("foo")) {
            Err(LuaArraySetError::InvalidValueType(ValueType::Bool)) => {}
            _ => panic!("Expected an error."),
        };

        // Try to push an array.
        match array.push(Value::Array(LuaArray::new(lua))) {
            Err(LuaArraySetError::InvalidValueType(ValueType::Bool)) => {}
            _ => panic!("Expected an error."),
        };

        // Try to push a table.
        match array.push(Value::Table(LuaTable::new(lua))) {
            Err(LuaArraySetError::InvalidValueType(ValueType::Bool)) => {}
            _ => panic!("Expected an error."),
        };

        // Push a bool.
        array.push(Value::Bool(false)).unwrap();
        assert_eq!(array.len(), 2);
        assert_eq!(array.get(0).unwrap().bool().unwrap(), true);
        assert_eq!(array.get(1).unwrap().bool().unwrap(), false);

        // Clear it.
        assert_eq!(array.pop().unwrap().bool().unwrap(), false);
        assert_eq!(array.len(), 1);
        assert_eq!(array.pop().unwrap().bool().unwrap(), true);
        assert_eq!(array.len(), 0);

        // Now push an int and make it an int array.
        array.push(Value::I64(7)).unwrap();
        assert_eq!(array.len(), 1);
        assert_eq!(array.get(0).unwrap().i64().unwrap(), 7);

        // Try to push a bool.
        match array.push(Value::Bool(true)) {
            Err(LuaArraySetError::InvalidValueType(ValueType::I64)) => {}
            _ => panic!("Expected an error."),
        };

        // Try to push a string.
        match array.push(Value::String("foo")) {
            Err(LuaArraySetError::InvalidValueType(ValueType::I64)) => {}
            _ => panic!("Expected an error."),
        };

        // Try to push an array.
        match array.push(Value::Array(LuaArray::new(lua))) {
            Err(LuaArraySetError::InvalidValueType(ValueType::I64)) => {}
            _ => panic!("Expected an error."),
        };

        // Try to push a table.
        match array.push(Value::Table(LuaTable::new(lua))) {
            Err(LuaArraySetError::InvalidValueType(ValueType::I64)) => {}
            _ => panic!("Expected an error."),
        };

        // Push a float.
        array.push(Value::F64(3.14)).unwrap();
        assert_eq!(array.len(), 2);
        assert!(cmp_f64(
            array.get(1).unwrap().f64().unwrap(),
            3.14
        ));

        // Push another int.
        array.push(Value::I64(-9)).unwrap();
        assert_eq!(array.len(), 3);
        assert_eq!(array.get(2).unwrap().i64().unwrap(), -9);

        // Iterate an array.
        for (index, value) in array.iter().enumerate() {
            match index {
                0 => assert_eq!(value.i64().unwrap(), 7),
                1 => assert!(cmp_f64(value.f64().unwrap(), 3.14)),
                2 => assert_eq!(value.i64().unwrap(), -9),
                _ => panic!("Invalid index."),
            }
        }

        config.root().set("array", Value::Array(array)).unwrap();

        // Array of arrays.
        let mut array = LuaArray::new(lua);

        for _ in 0..3 {
            let mut nested_array = LuaArray::new(lua);

            for _ in 0..3 {
                nested_array.push(Value::Bool(true)).unwrap();
            }
            assert_eq!(nested_array.len(), 3);

            array.push(Value::Array(nested_array)).unwrap();
        }
        assert_eq!(array.len(), 3);

        config
            .root()
            .set("another_array", Value::Array(array))
            .unwrap();

        // Validate the arrays.
        let root = config.root();
        let array = root.get("array").unwrap().array().unwrap();

        for (index, value) in array.iter().enumerate() {
            match index {
                0 => assert_eq!(value.i64().unwrap(), 7),
                1 => assert!(cmp_f64(value.f64().unwrap(), 3.14)),
                2 => assert_eq!(value.i64().unwrap(), -9),
                _ => panic!("Invalid index."),
            }
        }

        let another_array = root.get("another_array").unwrap().array().unwrap();

        for value in another_array.iter() {
            let nested_array = value.array().unwrap();

            for value in nested_array.iter() {
                assert_eq!(value.bool().unwrap(), true);
            }
        }
    });
}

#[cfg(feature = "bin")]
#[test]
fn bin_config() {
    let lua = rlua::Lua::new();

    lua.context(|lua| {
        // Load from script.
        let config = LuaConfig::from_script(lua, SCRIPT).unwrap();

        // Serialize to binary config.
        let data = config.to_bin_config().unwrap();

        // Load the binary config.
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
    });
}