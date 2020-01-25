use std::fmt::{Display, Formatter};

use super::util::{new_table, validate_lua_config_table};
use crate::{DisplayIndent, LuaArray, LuaConfigError, LuaConfigKeyError, LuaTable, Value};

#[cfg(feature = "bin")]
use crate::{BinConfigWriter, BinConfigWriterError};

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

        lua.load(script)
            .set_environment(root.clone())
            .unwrap()
            .exec()
            .map_err(LuaScriptError)?;

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

    #[cfg(feature = "bin")]
    fn table_to_bin_config(
        table: LuaTable<'_>,
        writer: &mut BinConfigWriter,
    ) -> Result<(), BinConfigWriterError> {
        // Gather the keys.
        let mut key_strins: Vec<_> = table.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order.
        key_strins.sort_by(|l, r| l.as_ref().cmp(r.as_ref()));

        // Iterate the table using the sorted keys.
        for key_string in key_strins.into_iter() {
            let key_string = key_string.as_ref();

            // Must succeed.
            let value = table.get(key_string).unwrap();

            Self::value_to_bin_config(Some(key_string), value, writer)?;
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
        self.root().fmt_indent(f, 0, false)
    }
}

impl<'lua> Display for Value<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_indent(f, 0, true)
    }
}
