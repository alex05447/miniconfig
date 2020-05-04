use std::fmt::{Display, Formatter, Write};

use super::util::{new_table, validate_lua_config_table};
use crate::{util::DisplayLua, LuaArray, LuaConfigError, LuaConfigKeyError, LuaTable, Value};

#[cfg(feature = "bin")]
use crate::{BinConfigWriter, BinConfigWriterError};

#[cfg(feature = "ini")]
use crate::{DisplayIni, ToIniStringError, ToIniStringOptions};

#[cfg(feature = "dyn")]
use crate::{DynArray, DynArrayMut, DynConfig, DynTable, DynTableMut};

use rlua::{Context, RegistryKey};

/// Represents a Lua-interned string.
pub struct LuaString<'lua>(rlua::String<'lua>);

impl<'lua> LuaString<'lua> {
    pub(super) fn new(string: rlua::String<'lua>) -> Self {
        Self(string)
    }
}

impl<'lua> AsRef<str> for LuaString<'lua> {
    fn as_ref(&self) -> &str {
        // Guaranteed to be a valid UTF-8 string because 1) we validate the config on construction
        // and 2) only accept valid UTF-8 strings when modifying the config.
        unsafe { std::str::from_utf8_unchecked(self.0.as_bytes()) }
    }
}

/// A [`value`] returned when accessing a Lua [`array`] or [`table`].
///
/// [`value`]: enum.Value.html
/// [`array`]: struct.LuaArray.html
/// [`table`]: struct.LuaTable.html
pub type LuaConfigValue<'lua> = Value<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>>;

/// Represents a mutable config with a root [`Lua table`] within the [`Lua context`].
///
/// [`Lua table`]: struct.LuaTable.html
/// [`Lua context`]: https://docs.rs/rlua/*/rlua/struct.Context.html
pub struct LuaConfig<'lua>(LuaTable<'lua>);

impl<'lua> LuaConfig<'lua> {
    /// Creates a new `[config`] with an empty root [`table`].
    ///
    /// [`config`]: struct.LuaConfig.html
    /// [`table`]: struct.LuaTable.html
    pub fn new(lua: Context<'lua>) -> Self {
        LuaConfig(LuaTable::from_valid_table(new_table(lua)))
    }

    /// Creates a new [`config`] from the Lua `script`.
    ///
    /// [`config`]: struct.LuaConfig.html
    pub fn from_script(lua: Context<'lua>, script: &str) -> Result<Self, LuaConfigError> {
        use LuaConfigError::*;

        let root = lua.create_table().unwrap();

        let script = format!("root = {}", script);

        lua.load(&script)
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
    pub fn from_table(
        lua: Context<'lua>,
        table: rlua::Table<'lua>,
    ) -> Result<Self, LuaConfigError> {
        let mut path = String::new();
        validate_lua_config_table(lua, &table, &mut path)?;

        Ok(LuaConfig(LuaTable::from_valid_table(table)))
    }

    /// Returns the mutable reference to the root [`table`] of the [`config`].
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

        Self::table_to_bin_config(root, &mut writer)?;

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

            Self::value_to_bin_config(Some(key_str), value, writer)?;
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
            Self::value_to_bin_config(None, value, writer)?;
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
                Self::array_to_bin_config(value, writer)?;
                writer.end()?;
            }
            Table(value) => {
                writer.table(key, value.len())?;
                Self::table_to_bin_config(value, writer)?;
                writer.end()?;
            }
        }

        Ok(())
    }

    #[cfg(feature = "dyn")]
    fn table_to_dyn_table(table: LuaTable<'_>, dyn_table: &mut DynTableMut<'_>) {
        for (key, value) in table.iter() {
            Self::value_to_dyn_table(key.as_ref(), value, dyn_table);
        }
    }

    #[cfg(feature = "dyn")]
    fn array_to_dyn_array(array: LuaArray<'_>, dyn_array: &mut DynArrayMut<'_>) {
        for value in array.iter() {
            Self::value_to_dyn_array(value, dyn_array);
        }
    }

    #[cfg(feature = "dyn")]
    fn value_to_dyn_table(
        key: &str,
        value: Value<LuaString<'_>, LuaArray<'_>, LuaTable<'_>>,
        dyn_table: &mut DynTableMut<'_>,
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
                dyn_table.set(key, Value::Array(DynArray::new())).unwrap();
                let mut array = dyn_table.get_mut(key).unwrap().array().unwrap();
                Self::array_to_dyn_array(value, &mut array);
            }
            Table(value) => {
                dyn_table.set(key, Value::Table(DynTable::new())).unwrap();
                let mut table = dyn_table.get_mut(key).unwrap().table().unwrap();
                Self::table_to_dyn_table(value, &mut table);
            }
        }
    }

    #[cfg(feature = "dyn")]
    fn value_to_dyn_array(
        value: Value<LuaString<'_>, LuaArray<'_>, LuaTable<'_>>,
        dyn_array: &mut DynArrayMut<'_>,
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
                dyn_array.push(Value::Array(DynArray::new())).unwrap();
                let last = dyn_array.len() - 1;
                let mut array = dyn_array.get_mut(last).unwrap().array().unwrap();
                Self::array_to_dyn_array(value, &mut array);
            }
            Table(value) => {
                dyn_array.push(Value::Table(DynTable::new())).unwrap();
                let last = dyn_array.len() - 1;
                let mut table = dyn_array.get_mut(last).unwrap().table().unwrap();
                Self::table_to_dyn_table(value, &mut table);
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

    /// Creates a new Lua config with an empty root [`table`].
    ///
    /// [`table`]: struct.LuaTable.html
    pub fn new(lua: Context<'_>) -> Self {
        LuaConfig::new(lua).key(lua)
    }

    /// Creates a new Lua config from the Lua `script`.
    pub fn from_script(lua: Context<'_>, script: &str) -> Result<Self, LuaConfigError> {
        LuaConfig::from_script(lua, script).map(|config| config.key(lua))
    }

    /// Returns the root [`Lua table`] of the config.
    ///
    /// [`Lua table`]: struct.LuaTable.html
    pub fn root<'lua>(&self, lua: Context<'lua>) -> Result<LuaTable<'lua>, LuaConfigKeyError> {
        Ok(self.config(lua)?.root())
    }
}

impl<'lua> Display for LuaConfig<'lua> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.root().fmt_lua(f, 0)
    }
}

impl<'lua> Display for Value<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}
