#![allow(clippy::approx_constant)]
#![allow(clippy::cognitive_complexity)]

use miniconfig::*;

use rlua;

fn cmp_f64(l: f64, r: f64) -> bool {
    (l - r).abs() < 0.000_001
}

fn main() {
    let lua = rlua::Lua::new();

    let script = "array_value = { 54, 12, 78.9 } -- array_value
bool_value = true
float_value = 3.14
int_value = 7
string_value = \"foo\"
table_value = {
\tbar = 2020,
\tbaz = \"hello\",
\tfoo = false,
} -- table_value";

    lua.context(|lua| {
        // Load from script.
        let config = LuaConfig::from_script(lua, script).unwrap();

        // Use the Lua config.
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

        // Serialize to string.
        let string = config.to_string();

        assert_eq!(script, string, "Script and serialized config mismatch.");

        // Serialize to binary config.
        let data = config.to_bin_config().unwrap();

        // Load the binary config.
        let bin_config = BinConfig::new(data).unwrap();

        // Use the binary config.
        let root = bin_config.root();

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
