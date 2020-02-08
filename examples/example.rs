#![allow(clippy::approx_constant)]
#![allow(clippy::cognitive_complexity)]

use miniconfig::*;

use rlua;

fn cmp_f64(l: f64, r: f64) -> bool {
    (l - r).abs() < 0.000_001
}

fn main() {
    let lua = rlua::Lua::new();

    let script = "{
\tarray_value = {
\t\t54,
\t\t12,
\t\t78.9,
\t}, -- array_value
\tbool_value = true,
\tfloat_value = 3.14,
\tint_value = 7,
\tstring_value = \"foo{}[];#:=\",
\ttable_value = {
\t\tbar = 2020,
\t\tbaz = \"hello\",
\t\tfoo = false,
\t\t[\"áéíóú\"] = \"42\",
\t}, -- table_value
}";

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
string = "bar"
"#;

    // Serialize to binary config.
    let data = lua.context(|lua| {
        // Load from script.
        let config = LuaConfig::from_script(lua, script).unwrap();

        // Use the Lua config.
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

        assert_eq!(
            root.get_string("string_value").unwrap().as_ref(),
            "foo{}[];#:="
        );

        let table_value = root.get_table("table_value").unwrap();

        assert_eq!(table_value.len(), 4);
        assert_eq!(table_value.get_i64("bar").unwrap(), 2020);
        assert!(cmp_f64(table_value.get_f64("bar").unwrap(), 2020.0));
        assert_eq!(table_value.get_string("baz").unwrap().as_ref(), "hello");
        assert_eq!(table_value.get_bool("foo").unwrap(), false);
        assert_eq!(table_value.get_string("áéíóú").unwrap().as_ref(), "42");

        // Serialize to string.
        let string = config.to_string();
        assert_eq!(script, string, "Script and serialized config mismatch.");

        // Serialize to binary config.
        config.to_bin_config().unwrap()
    });

    // Load the binary config.
    let bin_config = BinConfig::new(data).unwrap();

    // Use the binary config.
    let root = bin_config.root();

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

    assert_eq!(root.get_string("string_value").unwrap(), "foo{}[];#:=");

    let table_value = root.get_table("table_value").unwrap();

    assert_eq!(table_value.len(), 4);
    assert_eq!(table_value.get_i64("bar").unwrap(), 2020);
    assert!(cmp_f64(table_value.get_f64("bar").unwrap(), 2020.0));
    assert_eq!(table_value.get_string("baz").unwrap(), "hello");
    assert_eq!(table_value.get_bool("foo").unwrap(), false);

    // Load from INI.
    let config = DynConfig::from_ini(ini).unwrap();

    // Serialize to INI.
    let string = config.to_ini_string().unwrap();
    assert_eq!(string, ini);
}
