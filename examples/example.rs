#![allow(clippy::approx_constant)]
#![allow(clippy::cognitive_complexity)]

use miniconfig::*;

use rlua;

fn cmp_f64(l: f64, r: f64) -> bool {
    (l - r).abs() < 0.000_001
}

fn main() {
    let lua = rlua::Lua::new();

    let lua_script = "{
\tarray_value = {
\t\t54,
\t\t12,
\t\t78.9,
\t}, -- array_value
\tbool_value = true,
\tfloat_value = 3.14,
\tint_value = 7,
\tstring_value = \"foo\",
\ttable_value = {
\t\tbar = 2020,
\t\tbaz = \"hello\",
\t\tfoo = false,
\t\t[\"áéíóú\"] = \"42\",
\t}, -- table_value
}";

    let ini_script = r#"array_value = [54, 12, 78.9]
bool_value = true
float_value = 3.14
int_value = 7
string_value = "foo"

[table_value]
bar = 2020
baz = "hello"
foo = false
áéíóú = "42""#;

    let data = lua.context(|lua| {
        // Load from Lua script.
        let lua_config = LuaConfig::from_script(lua, lua_script).unwrap();

        // Use the Lua config.
        let array_value = lua_config.root().get_array("array_value").unwrap();

        assert_eq!(array_value.len(), 3);
        assert_eq!(array_value.get_i64(0).unwrap(), 54);
        assert!(cmp_f64(array_value.get_f64(0).unwrap(), 54.0));
        assert_eq!(array_value.get_i64(1).unwrap(), 12);
        assert!(cmp_f64(array_value.get_f64(1).unwrap(), 12.0));
        assert_eq!(array_value.get_i64(2).unwrap(), 78);
        assert!(cmp_f64(array_value.get_f64(2).unwrap(), 78.9));

        assert_eq!(lua_config.root().get_bool("bool_value").unwrap(), true);

        assert!(cmp_f64(
            lua_config.root().get_f64("float_value").unwrap(),
            3.14
        ));

        assert_eq!(lua_config.root().get_i64("int_value").unwrap(), 7);

        assert_eq!(
            lua_config
                .root()
                .get_string("string_value")
                .unwrap()
                .as_ref(),
            "foo"
        );

        let table_value = lua_config.root().get_table("table_value").unwrap();

        assert_eq!(table_value.len(), 4);
        assert_eq!(table_value.get_i64("bar").unwrap(), 2020);
        assert!(cmp_f64(table_value.get_f64("bar").unwrap(), 2020.0));
        assert_eq!(table_value.get_string("baz").unwrap().as_ref(), "hello");
        assert_eq!(table_value.get_bool("foo").unwrap(), false);
        assert_eq!(table_value.get_string("áéíóú").unwrap().as_ref(), "42");

        // Serialize to (Lua) string.
        assert_eq!(
            lua_script,
            lua_config.to_string(),
            "Source and serialized Lua scripts mismatch."
        );

        // Can't serialize to `.ini` string by default - arrays are not supported.
        assert_eq!(
            lua_config.to_ini_string(),
            Err(ToIniStringError::ArraysNotAllowed)
        );

        // But array support may be enabled.
        assert_eq!(
            lua_config
                .to_ini_string_opts(ToIniStringOptions {
                    arrays: true,
                    ..Default::default()
                })
                .unwrap(),
            ini_script,
            "Serialized `.ini` config mismatch."
        );

        // Serialize to binary config.
        lua_config.to_bin_config().unwrap()
    });

    // Binary config.
    {
        // Load the binary config.
        let bin_config = BinConfig::new(data).unwrap();

        // Use the binary config.
        let array_value = bin_config.root().get_array("array_value").unwrap();

        assert_eq!(array_value.len(), 3);
        assert_eq!(array_value.get_i64(0).unwrap(), 54);
        assert!(cmp_f64(array_value.get_f64(0).unwrap(), 54.0));
        assert_eq!(array_value.get_i64(1).unwrap(), 12);
        assert!(cmp_f64(array_value.get_f64(1).unwrap(), 12.0));
        assert_eq!(array_value.get_i64(2).unwrap(), 78);
        assert!(cmp_f64(array_value.get_f64(2).unwrap(), 78.9));

        assert_eq!(bin_config.root().get_bool("bool_value").unwrap(), true);

        assert!(cmp_f64(
            bin_config.root().get_f64("float_value").unwrap(),
            3.14
        ));

        assert_eq!(bin_config.root().get_i64("int_value").unwrap(), 7);

        assert_eq!(bin_config.root().get_string("string_value").unwrap(), "foo");

        let table_value = bin_config.root().get_table("table_value").unwrap();

        assert_eq!(table_value.len(), 4);
        assert_eq!(table_value.get_i64("bar").unwrap(), 2020);
        assert!(cmp_f64(table_value.get_f64("bar").unwrap(), 2020.0));
        assert_eq!(table_value.get_string("baz").unwrap(), "hello");
        assert_eq!(table_value.get_bool("foo").unwrap(), false);
        assert_eq!(table_value.get_string("áéíóú").unwrap(), "42");

        // Serialize to Lua string.
        assert_eq!(
            lua_script,
            bin_config.to_string(),
            "Source and serialized Lua scripts mismatch."
        );

        // Can't serialize to `.ini` string by default - arrays are not supported.
        assert_eq!(
            bin_config.to_ini_string(),
            Err(ToIniStringError::ArraysNotAllowed)
        );

        // But array support may be enabled.
        assert_eq!(
            bin_config
                .to_ini_string_opts(ToIniStringOptions {
                    arrays: true,
                    ..Default::default()
                })
                .unwrap(),
            ini_script,
            "Serialized `.ini` config mismatch."
        );
    }

    {
        // Load from `.ini`.
        let ini_config = DynConfig::from_ini(IniParser::new(ini_script).arrays(true)).unwrap();

        // Serialize to `.ini`.
        assert_eq!(
            ini_config
                .to_ini_string_opts(ToIniStringOptions {
                    arrays: true,
                    ..Default::default()
                })
                .unwrap(),
            ini_script,
            "Source and serialized `.ini` configs mismatch."
        );
    }
}
