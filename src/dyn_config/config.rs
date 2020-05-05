use std::fmt::{Display, Formatter, Write};

use crate::{
    util::DisplayLua, DynArray, DynArrayMut, DynArrayRef, DynTable, DynTableMut, DynTableRef, Value,
};

#[cfg(feature = "bin")]
use crate::{BinConfigWriter, BinConfigWriterError};

#[cfg(feature = "ini")]
use crate::{
    DisplayIni, IniConfig, IniError, IniParser, IniValue, ToIniStringError, ToIniStringOptions,
};

/// A [`value`] returned when accessing a dynamic [`array`] or [`table`].
///
/// [`value`]: enum.Value.html
/// [`array`]: struct.DynArray.html
/// [`table`]: struct.DynTable.html
pub type DynConfigValue = Value<String, DynArray, DynTable>;

/// A [`value`] returned when accessing a dynamic [`array`] or [`table`] by reference.
///
/// [`value`]: enum.Value.html
/// [`array`]: struct.DynArray.html
/// [`table`]: struct.DynTable.html
pub type DynConfigValueRef<'at> = Value<&'at str, DynArrayRef<'at>, DynTableRef<'at>>;

/// A [`value`] returned when accessing a dynamic [`array`] or [`table`] by mutable reference.
///
/// [`value`]: enum.Value.html
/// [`array`]: struct.DynArray.html
/// [`table`]: struct.DynTable.html
pub type DynConfigValueMut<'at> = Value<&'at str, DynArrayMut<'at>, DynTableMut<'at>>;

/// Represents a mutable config with a root hashmap [`table`].
///
/// [`table`]: struct.DynTable.html
pub struct DynConfig(DynTable);

impl DynConfig {
    /// Creates a new [`config`] with an empty root [`table`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`table`]: struct.DynTable.html
    pub fn new() -> Self {
        Self(DynTable::new())
    }

    /// Returns the immutable reference to the root [`table`] of the [`config`].
    ///
    /// [`table`]: struct.DynTable.html
    /// [`config`]: struct.DynConfig.html
    pub fn root(&self) -> DynTableRef<'_> {
        DynTableRef::new(&self.0)
    }

    /// Returns the mutable reference to the root [`table`] of the [`config`].
    ///
    /// [`table`]: struct.DynTable.html
    /// [`config`]: struct.DynConfig.html
    pub fn root_mut(&mut self) -> DynTableMut<'_> {
        DynTableMut::new(&mut self.0)
    }

    /// Tries to serialize this [`config`] to a [`binary config`].
    ///
    /// [`config`]: struct.DynConfig.html
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

    /// Creates a new [`config`] from the [`.ini parser`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`.ini parser`]: struct.IniParser.html
    #[cfg(feature = "ini")]
    pub fn from_ini(parser: IniParser) -> Result<Self, IniError> {
        let mut config = DynConfig::new();

        parser.parse(&mut config)?;

        Ok(config)
    }

    /// Tries to serialize this [`config`] to a Lua script string.
    ///
    /// NOTE: you may also call `to_string` via the [`config`]'s `Display` implementation.
    ///
    /// [`config`]: struct.DynConfig.html
    pub fn to_lua_string(&self) -> Result<String, std::fmt::Error> {
        let mut result = String::new();

        write!(&mut result, "{}", self)?;

        result.shrink_to_fit();

        Ok(result)
    }

    /// Tries to serialize this [`config`] to an `.ini` string.
    ///
    /// [`config`]: struct.DynConfig.html
    #[cfg(feature = "ini")]
    pub fn to_ini_string(&self) -> Result<String, ToIniStringError> {
        self.to_ini_string_opts(Default::default())
    }

    /// Tries to serialize this [`config`] to an `.ini` string using provided [`options`].
    ///
    /// [`config`]: struct.DynConfig.html
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

    #[cfg(feature = "bin")]
    fn table_to_bin_config(
        table: DynTableRef<'_>,
        writer: &mut BinConfigWriter,
    ) -> Result<(), BinConfigWriterError> {
        // Gather the keys.
        let mut keys: Vec<_> = table.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order.
        keys.sort_by(|l, r| l.cmp(r));

        // Iterate the table using the sorted keys.
        for key in keys.into_iter() {
            // Must succeed.
            let value = table.get(key).unwrap();

            Self::value_to_bin_config(Some(key), value, writer)?;
        }

        Ok(())
    }

    #[cfg(feature = "bin")]
    fn array_to_bin_config(
        array: DynArrayRef<'_>,
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
        value: DynConfigValueRef<'_>,
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

impl Display for DynConfig {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.root().fmt_lua(f, 0)
    }
}

impl<'a> Display for Value<&'a str, DynArray, DynTable> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}

impl<'a> Display for DynConfigValueRef<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}

impl<'a> Display for DynConfigValueMut<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}

#[cfg(feature = "ini")]
impl IniConfig for DynConfig {
    fn contains_section(&self, section: &str) -> bool {
        self.root().get_table(section).is_ok()
    }

    fn add_section(&mut self, section: &str, _overwrite: bool) {
        self.root_mut()
            .set(section, Value::Table(DynTable::new()))
            .unwrap();
    }

    fn contains_key(&self, section: Option<&str>, key: &str) -> bool {
        if let Some(section) = section {
            self.root().get_table(section).unwrap().get(key).is_ok()
        } else {
            self.root().get(key).is_ok()
        }
    }

    fn add_value(
        &mut self,
        section: Option<&str>,
        key: &str,
        value: IniValue<&str>,
        _overwrite: bool,
    ) {
        let mut table = if let Some(section) = section {
            self.root_mut().get_table_mut(section).unwrap()
        } else {
            self.root_mut()
        };

        match value {
            IniValue::Bool(value) => table.set(key, Value::Bool(value)),
            IniValue::I64(value) => table.set(key, Value::I64(value)),
            IniValue::F64(value) => table.set(key, Value::F64(value)),
            IniValue::String(value) => table.set(key, Value::String(value.into())),
        }
        .unwrap();
    }

    fn add_array(
        &mut self,
        section: Option<&str>,
        key: &str,
        mut array: Vec<IniValue<String>>,
        _overwrite: bool,
    ) {
        let mut table = if let Some(section) = section {
            self.root_mut().get_table_mut(section).unwrap()
        } else {
            self.root_mut()
        };

        let mut dyn_array = DynArray::new();

        for value in array.drain(0..array.len()) {
            dyn_array
                .push(match value {
                    IniValue::Bool(value) => Value::Bool(value),
                    IniValue::I64(value) => Value::I64(value),
                    IniValue::F64(value) => Value::F64(value),
                    IniValue::String(value) => Value::String(value),
                })
                .unwrap();
        }

        table.set(key, Value::Array(dyn_array)).unwrap();
    }
}
