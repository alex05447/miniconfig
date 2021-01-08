use {
    super::util::{new_table, validate_lua_config_table},
    crate::{util::DisplayLua, LuaConfigError, LuaConfigKeyError, LuaTable},
    rlua::{Context, RegistryKey},
    std::fmt::{Display, Formatter, Write},
};

#[cfg(feature = "bin")]
use crate::{BinConfigWriter, BinConfigWriterError};

#[cfg(feature = "ini")]
use crate::{DisplayIni, ToIniStringError, ToIniStringOptions};

#[cfg(feature = "dyn")]
use crate::{DynArray, DynConfig, DynTable};

#[cfg(any(feature = "bin", feature = "dyn"))]
use crate::{LuaArray, LuaString, Value};

/// Represents a mutable config with a root [`Lua table`] within the [`Lua context`].
///
/// [`Lua table`]: struct.LuaTable.html
/// [`Lua context`]: https://docs.rs/rlua/*/rlua/struct.Context.html
#[derive(Clone)]
pub struct LuaConfig<'lua>(LuaTable<'lua>);

impl<'lua> LuaConfig<'lua> {
    /// Creates a new `[config`] with an empty root [`table`].
    ///
    /// [`config`]: struct.LuaConfig.html
    /// [`table`]: struct.LuaTable.html
    pub fn new(lua: Context<'lua>) -> Self {
        LuaConfig(LuaTable::from_valid_table(new_table(lua)))
    }

    /// Creates a new [`Lua config`] from the Lua `script`.
    ///
    /// [`Lua config`]: struct.LuaConfig.html
    pub fn from_script<'a>(lua: Context<'lua>, script: &str) -> Result<Self, LuaConfigError<'a>> {
        use LuaConfigError::*;

        let root = lua.create_table().unwrap();

        let script =
            std::iter::once(b"root = " as &[u8]).chain(std::iter::once(script).map(str::as_bytes));

        lua.load_ex(script)
            .set_environment(root.clone())
            .unwrap()
            .exec()
            .map_err(LuaScriptError)?;

        let root = root.get("root").unwrap();

        Self::from_table(lua, root)
    }

    /// Creates a new [`config`] from the Lua `table`.
    ///
    /// [`config`]: struct.LuaConfig.html
    pub fn from_table<'a>(
        lua: Context<'lua>,
        table: rlua::Table<'lua>,
    ) -> Result<Self, LuaConfigError<'a>> {
        validate_lua_config_table(lua, &table)?;

        Ok(LuaConfig(LuaTable::from_valid_table(table)))
    }

    /// Returns the reference to the root [`table`] of the [`config`].
    ///
    /// [`table`]: struct.LuaTable.html
    /// [`config`]: struct.LuaConfig.html
    pub fn root(&self) -> LuaTable<'lua> {
        self.0.clone()
    }

    /// Creates a [`LuaConfigKey`] from this [`config`],
    /// allowing it to exist outside the [`Lua context`].
    ///
    /// The value is valid for the lifetime of the [`Lua state`] or until explicitly [`destroy`]'ed.
    ///
    /// [`LuaConfigKey`]: struct.LuaConfigKey.html
    /// [`config`]: struct.LuaConfig.html
    /// [`Lua context`]: https://docs.rs/rlua/*/rlua/struct.Context.html
    /// [`Lua state`]: https://docs.rs/rlua/*/rlua/struct.Lua.html
    /// [`destroy`]: struct.LuaConfigKey.html#method.destroy
    pub fn key(self, lua: rlua::Context<'lua>) -> LuaConfigKey {
        LuaConfigKey(lua.create_registry_value((self.0).0).unwrap())
    }

    /// Tries to serialize this [`config`] to a Lua script string.
    ///
    /// NOTE: you may also call `to_string` via the [`config`]'s `Display` implementation.
    ///
    /// [`config`]: struct.LuaConfig.html
    pub fn to_lua_string(&self) -> Result<String, std::fmt::Error> {
        let mut result = String::new();

        write!(&mut result, "{}", self)?;

        result.shrink_to_fit();

        Ok(result)
    }

    /// Serializes this [`config`] to a [`dynamic config`].
    ///
    /// [`config`]: struct.LuaConfig.html
    /// [`dynamic config`]: struct.DynConfig.html
    #[cfg(feature = "dyn")]
    pub fn to_dyn_config(&self) -> DynConfig {
        let mut result = DynConfig::new();

        Self::table_to_dyn_table(self.root(), &mut result.root_mut());

        result
    }

    /// Tries to serialize this [`config`] to a [`binary config`].
    ///
    /// [`config`]: struct.LuaConfig.html
    /// [`binary config`]: struct.BinConfig.html
    #[cfg(feature = "bin")]
    pub fn to_bin_config(&self) -> Result<Box<[u8]>, BinConfigWriterError> {
        use BinConfigWriterError::*;

        let root = self.root();

        // The root table is empty - nothing to do.
        if root.len() == 0 {
            return Err(EmptyRootTable);
        }

        let mut writer = BinConfigWriter::new(root.len())?;

        table_to_bin_config(root, &mut writer)?;

        writer.finish()
    }

    /// Tries to serialize this [`config`] to an `.ini` string.
    ///
    /// [`config`]: struct.LuaConfig.html
    #[cfg(feature = "ini")]
    pub fn to_ini_string(&self) -> Result<String, ToIniStringError> {
        self.to_ini_string_opts(Default::default())
    }

    /// Tries to serialize this [`config`] to an `.ini` string using provided [`options`].
    ///
    /// [`config`]: struct.LuaConfig.html
    /// [`options`]: struct.ToIniStringOptions.html
    #[cfg(feature = "ini")]
    pub fn to_ini_string_opts(
        &self,
        options: ToIniStringOptions,
    ) -> Result<String, ToIniStringError> {
        let mut result = String::new();

        self.root().fmt_ini(&mut result, 0, false, options)?;

        result.shrink_to_fit();

        Ok(result)
    }

    #[cfg(feature = "dyn")]
    fn table_to_dyn_table(table: LuaTable<'_>, dyn_table: &mut DynTable) {
        for (key, value) in table.iter() {
            Self::value_to_dyn_table(key.as_ref(), value, dyn_table);
        }
    }

    #[cfg(feature = "dyn")]
    fn array_to_dyn_array(array: LuaArray<'_>, dyn_array: &mut DynArray) {
        for value in array.iter() {
            Self::value_to_dyn_array(value, dyn_array);
        }
    }

    #[cfg(feature = "dyn")]
    fn value_to_dyn_table(
        key: &str,
        value: Value<LuaString<'_>, LuaArray<'_>, LuaTable<'_>>,
        dyn_table: &mut DynTable,
    ) {
        use Value::*;

        match value {
            Bool(value) => {
                dyn_table.set(key, Value::Bool(value)).unwrap();
            }
            I64(value) => {
                dyn_table.set(key, Value::I64(value)).unwrap();
            }
            F64(value) => {
                dyn_table.set(key, Value::F64(value)).unwrap();
            }
            String(value) => {
                dyn_table
                    .set(key, Value::String(value.as_ref().into()))
                    .unwrap();
            }
            Array(value) => {
                let mut array = DynArray::new();
                Self::array_to_dyn_array(value, &mut array);
                dyn_table.set(key, Value::Array(array)).unwrap();
            }
            Table(value) => {
                let mut table = DynTable::new();
                Self::table_to_dyn_table(value, &mut table);
                dyn_table.set(key, Value::Table(table)).unwrap();
            }
        }
    }

    #[cfg(feature = "dyn")]
    fn value_to_dyn_array(
        value: Value<LuaString<'_>, LuaArray<'_>, LuaTable<'_>>,
        dyn_array: &mut DynArray,
    ) {
        use Value::*;

        match value {
            Bool(value) => {
                dyn_array.push(Value::Bool(value)).unwrap();
            }
            I64(value) => {
                dyn_array.push(Value::I64(value)).unwrap();
            }
            F64(value) => {
                dyn_array.push(Value::F64(value)).unwrap();
            }
            String(value) => {
                dyn_array
                    .push(Value::String(value.as_ref().to_owned()))
                    .unwrap();
            }
            Array(value) => {
                let mut array = DynArray::new();
                Self::array_to_dyn_array(value, &mut array);
                dyn_array.push(Value::Array(array)).unwrap();
            }
            Table(value) => {
                let mut table = DynTable::new();
                Self::table_to_dyn_table(value, &mut table);
                dyn_array.push(Value::Table(table)).unwrap();
            }
        }
    }
}

/// Represents a mutable config with a root [`Lua table`] as a Lua registry key
/// so it may exist outside the [`Lua context`].
///
/// Returned by [`LuaConfig::key`].
///
/// NOTE: must be explicitly [`destroy`]'ed.
///
/// [`Lua table`]: struct.LuaTable.html
/// [`Lua context`]: https://docs.rs/rlua/*/rlua/struct.Context.html
/// [`LuaConfig::key`]: struct.LuaConfig.html#method.key
/// [`destroy`]: #method.destroy
pub struct LuaConfigKey(RegistryKey);

impl LuaConfigKey {
    /// Dereferences the Lua config key, returning the associated [`LuaConfig`].
    ///
    /// [`LuaConfig`]: struct.LuaConfig.html
    pub fn config<'lua>(&self, lua: Context<'lua>) -> Result<LuaConfig<'lua>, LuaConfigKeyError> {
        use LuaConfigKeyError::*;

        let root = lua.registry_value(&self.0).map_err(|err| match err {
            rlua::Error::MismatchedRegistryKey => LuaStateMismatch,
            _ => panic!("Unknown error."),
        })?;

        Ok(LuaConfig(LuaTable::from_valid_table(root)))
    }

    /// Destroys the Lua config key.
    pub fn destroy(self, lua: Context<'_>) -> Result<(), LuaConfigKeyError> {
        use LuaConfigKeyError::*;

        lua.remove_registry_value(self.0).map_err(|err| match err {
            rlua::Error::MismatchedRegistryKey => LuaStateMismatch,
            _ => panic!("Unknown error."),
        })
    }

    /// Creates a new [`Lua config`] with an empty root [`table`].
    ///
    /// [`Lua config`]: struct.LuaConfigKey.html
    /// [`table`]: struct.LuaTable.html
    pub fn new(lua: Context<'_>) -> Self {
        LuaConfig::new(lua).key(lua)
    }

    /// Creates a new [`Lua config`] from the Lua `script`.
    ///
    /// [`Lua config`]: struct.LuaConfigKey.html
    pub fn from_script<'a>(lua: Context<'_>, script: &str) -> Result<Self, LuaConfigError<'a>> {
        LuaConfig::from_script(lua, script).map(|config| config.key(lua))
    }

    /// Returns the root [`Lua table`] of the config.
    ///
    /// [`Lua table`]: struct.LuaTable.html
    pub fn root<'lua>(&self, lua: Context<'lua>) -> Result<LuaTable<'lua>, LuaConfigKeyError> {
        Ok(self.config(lua)?.root())
    }
}

#[cfg(feature = "bin")]
fn table_to_bin_config(
    table: LuaTable<'_>,
    writer: &mut BinConfigWriter,
) -> Result<(), BinConfigWriterError> {
    // Gather the keys.
    let mut keys: Vec<_> = table.iter().map(|(key, _)| key).collect();

    // Sort the keys in alphabetical order.
    keys.sort_by(|l, r| l.as_ref().cmp(r.as_ref()));

    // Iterate the table using the sorted keys.
    for key in keys.into_iter() {
        let key_str = key.as_ref();

        // Must succeed.
        let value = table.get(key_str).unwrap();

        value_to_bin_config(Some(key_str), value, writer)?;
    }

    Ok(())
}

#[cfg(feature = "bin")]
fn array_to_bin_config(
    array: LuaArray<'_>,
    writer: &mut BinConfigWriter,
) -> Result<(), BinConfigWriterError> {
    // Iterate the array in order.
    for value in array.iter() {
        value_to_bin_config(None, value, writer)?;
    }

    Ok(())
}

#[cfg(feature = "bin")]
fn value_to_bin_config(
    key: Option<&str>,
    value: Value<LuaString<'_>, LuaArray<'_>, LuaTable<'_>>,
    writer: &mut BinConfigWriter,
) -> Result<(), BinConfigWriterError> {
    use Value::*;

    match value {
        Bool(value) => {
            writer.bool(key, value)?;
        }
        I64(value) => {
            writer.i64(key, value)?;
        }
        F64(value) => {
            writer.f64(key, value)?;
        }
        String(value) => {
            writer.string(key, value.as_ref())?;
        }
        Array(value) => {
            writer.array(key, value.len())?;
            array_to_bin_config(value, writer)?;
            writer.end()?;
        }
        Table(value) => {
            writer.table(key, value.len())?;
            table_to_bin_config(value, writer)?;
            writer.end()?;
        }
    }

    Ok(())
}

impl<'lua> Display for LuaConfig<'lua> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.root().fmt_lua(f, 0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use {crate::*, rlua_ext::ValueType as LuaValueType};

    fn lua_config(script: &str) -> Result<(), LuaConfigError> {
        let lua = rlua::Lua::new();

        lua.context(|lua| LuaConfigKey::from_script(lua, script))?;

        Ok(())
    }

    fn lua_config_error(script: &str) -> LuaConfigError {
        lua_config(script).err().expect("expected an error")
    }

    #[test]
    fn LuaConfigError_LuaScriptError() {
        assert!(matches!(
            lua_config_error(r#" ?!#>& "#),
            LuaConfigError::LuaScriptError(_)
        ));
    }

    #[test]
    fn LuaConfigError_MixedKeys() {
        assert!(matches!(
            lua_config_error(
                r#"{
                        foo = true,
                        [1] = 7,
                    }"#,
            ),
            LuaConfigError::MixedKeys(path) if path == ConfigPath::new()
        ));

        assert!(matches!(
            lua_config_error(
                r#"{
                        table = {
                            foo = true,
                            [1] = 7,
                        }
                    }"#,
            ),
            LuaConfigError::MixedKeys(path) if path == ConfigPath(vec!["table".into()])
        ));

        assert!(matches!(
            lua_config_error(
                r#"{
                        table = {
                            nested_table = {
                                foo = true,
                                [1] = 7,
                            }
                        }
                    }"#,
            ),
            LuaConfigError::MixedKeys(path) if path == ConfigPath(vec!["table".into(), "nested_table".into()])
        ));

        // But this should work.

        lua_config(
            r#"{
                    foo = true,
                    bar = 7,
                }"#,
        )
        .unwrap();

        lua_config(
            r#"{
                    array = {
                        true,
                        false,
                    }
                }"#,
        )
        .unwrap();
    }

    #[test]
    fn LuaConfigError_MixedArray() {
        assert!(matches!(
            lua_config_error(
                r#"{
                        array = {
                            true,
                            7,
                            3.14,
                        }
                    }"#,
            ),
            LuaConfigError::MixedArray { path, expected, found } if path == ConfigPath(vec!["array".into(), 1.into()]) && expected == LuaValueType::Boolean && found == LuaValueType::Integer
        ));

        assert!(matches!(
            lua_config_error(
                r#"{
                        table = {
                            array = {
                                true,
                                7,
                                3.14,
                            }
                        }
                    }"#,
            ),
            LuaConfigError::MixedArray { path, expected, found } if path == ConfigPath(vec!["table".into(), "array".into(), 1.into()]) && expected == LuaValueType::Boolean && found == LuaValueType::Integer
        ));

        // But this should work.

        lua_config(
            r#"{
                array = {
                    -24,
                    7,
                    3.14,
                }
            }"#,
        )
        .unwrap();
    }

    #[test]
    fn LuaConfigError_InvalidKeyType() {
        assert!(matches!(
            lua_config_error(
                r#"{
                        [3.14] = true,
                    }"#,
            ),
            LuaConfigError::InvalidKeyType { path, invalid_type } if path == ConfigPath::new() && invalid_type == LuaValueType::Number
        ));

        assert!(matches!(
            lua_config_error(
                r#"{
                        table = {
                            [3.14] = true,
                        }
                    }"#,
            ),
            LuaConfigError::InvalidKeyType { path, invalid_type } if path == ConfigPath(vec!["table".into()]) && invalid_type == LuaValueType::Number
        ));

        assert!(matches!(
            lua_config_error(
                r#"{
                        table = {
                            nested_table = {
                                [3.14] = true,
                            }
                        }
                    }"#,
            ),
            LuaConfigError::InvalidKeyType { path, invalid_type } if path == ConfigPath(vec!["table".into(), "nested_table".into()]) && invalid_type == LuaValueType::Number
        ));
    }

    #[test]
    fn LuaConfigError_InvalidKeyUTF8() {
        assert!(matches!(
            lua_config_error(
                r#"{
                        ["\xc0"] = 7
                    }"#,
            ),
            LuaConfigError::InvalidKeyUTF8 { path, .. } if path == ConfigPath::new()
        ));

        assert!(matches!(
            lua_config_error(
                r#"{
                        table = {
                            ["\xc0"] = 7
                        }
                    }"#,
            ),
            LuaConfigError::InvalidKeyUTF8 { path, .. } if path == ConfigPath(vec!["table".into()])
        ));
    }

    #[test]
    fn LuaConfigError_EmptyKey() {
        assert!(matches!(
            lua_config_error(
                r#"{
                        table = {
                            [""] = 7
                        }
                    }"#,
            ),
            LuaConfigError::EmptyKey(path) if path == ConfigPath(vec!["table".into()])
        ));
    }

    #[test]
    fn LuaConfigError_InvalidArrayIndex() {
        assert!(matches!(
            lua_config_error(
                r#"{
                    table = {
                        [0] = 7
                    }
                }"#,
            ),
            LuaConfigError::InvalidArrayIndex(path) if path == ConfigPath(vec!["table".into()])
        ));

        // But this should work.

        lua_config(
            r#"{
                table = {
                    [1] = 7
                }
            }"#,
        )
        .unwrap();

        lua_config(
            r#"{
                table = {
                    7
                }
            }"#,
        )
        .unwrap();
    }

    #[test]
    fn LuaConfigError_InvalidValueType() {
        assert!(matches!(
            lua_config_error(
                r#"{
                    table = {
                        invalid = function () end
                    }
                }"#,
            ),
            LuaConfigError::InvalidValueType { path, invalid_type } if path == ConfigPath(vec!["table".into(), "invalid".into()]) && invalid_type == LuaValueType::Function
        ));

        assert!(matches!(
            lua_config_error(
                r#"{
                    array = {
                        [1] = function () end
                    }
                }"#,
            ),
            LuaConfigError::InvalidValueType { path, invalid_type } if path == ConfigPath(vec!["array".into(), 0.into()]) && invalid_type == LuaValueType::Function
        ));
    }

    #[test]
    fn LuaConfigError_InvalidValueUTF8() {
        assert!(matches!(
            lua_config_error(
                r#"{
                    table = {
                        string = "\xc0"
                    }
                }"#,
            ),
            LuaConfigError::InvalidValueUTF8 { path, .. } if path == ConfigPath(vec!["table".into(), "string".into()])
        ));
    }

    const SCRIPT: &str = "{
\tarray_of_tables_value = {
\t\t{
\t\t\tfoo = 1,
\t\t}, -- [0]
\t\t{
\t\t\tbar = 2,
\t\t}, -- [1]
\t\t{
\t\t\tbaz = 3,
\t\t}, -- [2]
\t}, -- array_of_tables_value
\tarray_value = {
\t\t54,
\t\t12,
\t\t78.9,
\t}, -- array_value
\tbool_value = true,
\t[\"fancy 'value'\"] = \"\\t'\\\"\",
\tfloat_value = 3.14,
\tint_value = 7,
\tstring_value = \"foo{}[];#:=\",
\ttable_value = {
\t\tbar = 2020,
\t\tbaz = \"hello\",
\t\tfoo = false,
\t}, -- table_value
}";

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

    #[cfg(feature = "bin")]
    #[test]
    fn to_bin_config() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            // Load from script.
            let lua_config = LuaConfig::from_script(lua, SCRIPT).unwrap();

            // Serialize to binary config.
            let bin_data = lua_config.to_bin_config().unwrap();

            // Load the binary config.
            let bin_config = BinConfig::new(bin_data).unwrap();

            let array_value = bin_config.root().get_array("array_value".into()).unwrap();

            assert_eq!(array_value.len(), 3);
            assert_eq!(array_value.get_i64(0).unwrap(), 54);
            assert!(cmp_f64(array_value.get_f64(0).unwrap(), 54.0));
            assert_eq!(array_value.get_i64(1).unwrap(), 12);
            assert!(cmp_f64(array_value.get_f64(1).unwrap(), 12.0));
            assert_eq!(array_value.get_i64(2).unwrap(), 78);
            assert!(cmp_f64(array_value.get_f64(2).unwrap(), 78.9));

            assert_eq!(
                bin_config.root().get_bool("bool_value".into()).unwrap(),
                true
            );

            assert_eq!(
                bin_config
                    .root()
                    .get_string("fancy 'value'".into())
                    .unwrap(),
                "\t'\""
            );

            assert!(cmp_f64(
                bin_config.root().get_f64("float_value".into()).unwrap(),
                3.14
            ));

            assert_eq!(bin_config.root().get_i64("int_value".into()).unwrap(), 7);

            assert_eq!(
                bin_config.root().get_string("string_value".into()).unwrap(),
                "foo{}[];#:="
            );

            let table_value = bin_config.root().get_table("table_value".into()).unwrap();

            assert_eq!(table_value.len(), 3);
            assert_eq!(table_value.get_i64("bar".into()).unwrap(), 2020);
            assert!(cmp_f64(table_value.get_f64("bar".into()).unwrap(), 2020.0));
            assert_eq!(table_value.get_string("baz".into()).unwrap(), "hello");
            assert_eq!(table_value.get_bool("foo".into()).unwrap(), false);
        });
    }

    #[cfg(feature = "ini")]
    #[test]
    fn to_ini_string() {
        let script = r#"
{
    array = { "foo", "bar", "baz" },
    bool = true,
    float = 3.14,
    int = 7,
    -- "foo"
    string = "\x66\x6f\x6f",

    ["'other' section"] = {
        other_bool = true,
        other_float = 3.14,
        other_int = 7,
        other_string = "foo",
    },

    section = {
        bool = false,
        float = 7.62,
        int = 9,
        string = "bar",
    },
}
"#;

        let ini = r#"array = ["foo", "bar", "baz"]
bool = true
float = 3.14
int = 7
string = "foo"

["'other' section"]
other_bool = true
other_float = 3.14
other_int = 7
other_string = "foo"

[section]
bool = false
float = 7.62
int = 9
string = "bar""#;

        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let config = LuaConfig::from_script(lua, script).unwrap();

            assert_eq!(
                ini,
                config
                    .to_ini_string_opts(ToIniStringOptions {
                        arrays: true,
                        ..Default::default()
                    })
                    .unwrap()
            );
        });
    }

    #[cfg(feature = "dyn")]
    #[test]
    fn to_dyn_config() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            // Load from script.
            let config = LuaConfig::from_script(lua, SCRIPT).unwrap();

            // Serialize to dynamic config.
            let dyn_config = config.to_dyn_config();

            let array_value = dyn_config.root().get_array("array_value").unwrap();

            assert_eq!(array_value.len(), 3);
            assert_eq!(array_value.get_i64(0).unwrap(), 54);
            assert!(cmp_f64(array_value.get_f64(0).unwrap(), 54.0));
            assert_eq!(array_value.get_i64(1).unwrap(), 12);
            assert!(cmp_f64(array_value.get_f64(1).unwrap(), 12.0));
            assert_eq!(array_value.get_i64(2).unwrap(), 78);
            assert!(cmp_f64(array_value.get_f64(2).unwrap(), 78.9));

            assert_eq!(dyn_config.root().get_bool("bool_value").unwrap(), true);

            assert!(cmp_f64(
                dyn_config.root().get_f64("float_value").unwrap(),
                3.14
            ));

            assert_eq!(dyn_config.root().get_i64("int_value").unwrap(), 7);

            assert_eq!(
                dyn_config.root().get_string("string_value").unwrap(),
                "foo{}[];#:="
            );

            let table_value = dyn_config.root().get_table("table_value").unwrap();

            assert_eq!(table_value.len(), 3);
            assert_eq!(table_value.get_i64("bar").unwrap(), 2020);
            assert!(cmp_f64(table_value.get_f64("bar").unwrap(), 2020.0));
            assert_eq!(table_value.get_string("baz").unwrap(), "hello");
            assert_eq!(table_value.get_bool("foo").unwrap(), false);
        });
    }

    #[test]
    fn GetPathError_EmptyKey() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            assert_eq!(
                table.get_path(&["".into()]).err().unwrap(),
                GetPathError::EmptyKey(ConfigPath::new())
            );

            let mut other_table = LuaTable::new(lua);
            other_table.set("bar", Some(true.into())).unwrap();

            table.set("foo", Some(other_table.into())).unwrap();

            assert_eq!(
                table.get_path(&["foo".into(), "".into()]).err().unwrap(),
                GetPathError::EmptyKey(ConfigPath(vec!["foo".into()]))
            );

            // But this works.

            assert_eq!(
                table.get_bool_path(&["foo".into(), "bar".into()]).unwrap(),
                true,
            );
        });
    }

    #[test]
    fn GetPathError_PathDoesNotExist() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            let mut foo = LuaTable::new(lua);
            let mut bar = LuaArray::new(lua);
            let mut baz = LuaTable::new(lua);

            baz.set("bob", Some(true.into())).unwrap();
            bar.push(baz.into()).unwrap();
            foo.set("bar", Some(bar.into())).unwrap();
            table.set("foo", Some(foo.into())).unwrap();

            assert_eq!(
                table.get_path(&["foo".into(), "baz".into()]).err().unwrap(),
                GetPathError::KeyDoesNotExist(ConfigPath(vec!["foo".into(), "baz".into()]))
            );

            assert_eq!(
                table
                    .get_path(&["foo".into(), "bar".into(), 0.into(), "bill".into()])
                    .err()
                    .unwrap(),
                GetPathError::KeyDoesNotExist(ConfigPath(vec![
                    "foo".into(),
                    "bar".into(),
                    0.into(),
                    "bill".into()
                ]))
            );

            // But this works.

            assert_eq!(
                table
                    .get_bool_path(&["foo".into(), "bar".into(), 0.into(), "bob".into()])
                    .unwrap(),
                true
            );
        });
    }

    #[test]
    fn GetPathError_IndexOutOfBounds() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            let mut array = LuaArray::new(lua);
            array.push(true.into()).unwrap();

            table.set("array", Some(array.into())).unwrap();

            assert_eq!(
                table.get_path(&["array".into(), 1.into()]).err().unwrap(),
                GetPathError::IndexOutOfBounds {
                    path: ConfigPath(vec!["array".into(), 1.into()]),
                    len: 1
                }
            );

            // But this works.

            assert_eq!(
                table.get_bool_path(&["array".into(), 0.into()]).unwrap(),
                true
            );
        });
    }

    #[test]
    fn GetPathError_ValueNotAnArray() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            let mut other_table = LuaTable::new(lua);
            other_table.set("array", Some(true.into())).unwrap();

            table.set("table", Some(other_table.into())).unwrap();

            assert_eq!(
                table
                    .get_path(&["table".into(), "array".into(), 1.into()])
                    .err()
                    .unwrap(),
                GetPathError::ValueNotAnArray {
                    path: ConfigPath(vec!["table".into(), "array".into()]),
                    value_type: ValueType::Bool
                }
            );

            // But this works.

            assert_eq!(
                table
                    .get_bool_path(&["table".into(), "array".into()])
                    .unwrap(),
                true,
            );
        });
    }

    #[test]
    fn GetPathError_ValueNotATable() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            let mut array = LuaArray::new(lua);
            array.push(true.into()).unwrap();

            table.set("array", Some(array.into())).unwrap();

            assert_eq!(
                table
                    .get_path(&["array".into(), 0.into(), "foo".into()])
                    .err()
                    .unwrap(),
                GetPathError::ValueNotATable {
                    path: ConfigPath(vec!["array".into(), 0.into()]),
                    value_type: ValueType::Bool
                }
            );

            // But this works.

            assert_eq!(
                table.get_bool_path(&["array".into(), 0.into()]).unwrap(),
                true,
            );
        });
    }

    #[test]
    fn GetPathError_IncorrectValueType() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            let mut other_table = LuaTable::new(lua);
            other_table.set("foo", Some(true.into())).unwrap();
            other_table.set("bar", Some(3.14.into())).unwrap();

            table.set("table", Some(other_table.into())).unwrap();

            assert_eq!(
                table
                    .get_i64_path(&["table".into(), "foo".into()])
                    .err()
                    .unwrap(),
                GetPathError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table
                    .get_f64_path(&["table".into(), "foo".into()])
                    .err()
                    .unwrap(),
                GetPathError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table
                    .get_string_path(&["table".into(), "foo".into()])
                    .err()
                    .unwrap(),
                GetPathError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table
                    .get_array_path(&["table".into(), "foo".into()])
                    .err()
                    .unwrap(),
                GetPathError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table
                    .get_table_path(&["table".into(), "foo".into()])
                    .err()
                    .unwrap(),
                GetPathError::IncorrectValueType(ValueType::Bool)
            );

            // But this works.

            assert_eq!(
                table
                    .get_bool_path(&["table".into(), "foo".into()])
                    .unwrap(),
                true
            );

            assert_eq!(
                table.get_i64_path(&["table".into(), "bar".into()]).unwrap(),
                3
            );
            assert!(cmp_f64(
                table.get_f64_path(&["table".into(), "bar".into()]).unwrap(),
                3.14
            ));
        });
    }
}
