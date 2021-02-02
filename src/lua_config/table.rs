use {
    super::util::{
        clear_table, get_table_len, new_table, set_table_len, value_from_lua_value,
        ValueFromLuaValueError,
    },
    crate::{
        util::{write_lua_key, DisplayLua},
        ConfigKey, GetPathError, LuaArray, LuaConfigValue, LuaString, NonEmptyStr, TableError,
        Value, ValueType,
    },
    rlua::Context,
    std::{
        borrow::Borrow,
        fmt::{Display, Formatter},
    },
};

#[cfg(feature = "ini")]
use {
    crate::{
        write_ini_array, write_ini_table, write_ini_value, DisplayIni, IniPath, ToIniStringError,
        ToIniStringOptions,
    },
    std::fmt::Write,
};

/// Represents a mutable Lua hash table of [`Value`]'s with string keys.
///
/// [`Value`]: enum.Value.html
#[derive(Clone)]
pub struct LuaTable<'lua>(pub(super) rlua::Table<'lua>);

impl<'lua> LuaTable<'lua> {
    /// Creates a new empty [`table`].
    ///
    /// [`table`]: struct.LuaTable.html
    pub fn new(lua: Context<'lua>) -> Self {
        Self(new_table(lua))
    }

    /// Returns the number of entries in the [`table`].
    ///
    /// [`table`]: struct.LuaTable.html
    pub fn len(&self) -> u32 {
        self.len_impl()
    }

    /// Returns `true` if the [`table`] is empty.
    ///
    /// [`table`]: struct.LuaTable.html
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the [`table`].
    ///
    /// [`table`]: struct.LuaTable.html
    pub fn clear(&mut self) {
        clear_table(&self.0);

        set_table_len(&self.0, 0);
    }

    /// Returns `true` if the [`table`] contains a [`value`] with the string `key`.
    ///
    /// [`table`]: struct.LuaTable.html
    /// [`value`]: type.LuaConfigValue.html
    pub fn contains<K: AsRef<str>>(&self, key: K) -> bool {
        use TableError::*;

        match self.get(key) {
            Ok(_) => true,
            Err(err) => match err {
                KeyDoesNotExist | EmptyKey => false,
                IncorrectValueType(_) => unreachable!(),
            },
        }
    }

    /// Tries to get a reference to a [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty or if [`table`] does not contain the `key`.
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`error`]: enum.TableError.html
    pub fn get<K: AsRef<str>>(&self, key: K) -> Result<LuaConfigValue<'lua>, TableError> {
        self.get_impl(key.as_ref())
    }

    /// Tries to get a reference to a [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns the [`table`] itself if the `path` is empty.
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    /// [`type`]: enum.ValueType.html
    pub fn get_path<'a, K, P>(&self, path: P) -> Result<LuaConfigValue<'lua>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        LuaConfigValue::Table(self.clone())
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_bool<K: AsRef<str>>(&self, key: K) -> Result<bool, TableError> {
        let val = self.get(key)?;
        val.bool()
            .ok_or(TableError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to a [`bool`] [`value`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_bool_path<'a, K, P>(&self, path: P) -> Result<bool, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        val.bool()
            .ok_or(GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not an [`i64`] / [`f64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_i64<K: AsRef<str>>(&self, key: K) -> Result<i64, TableError> {
        let val = self.get(key)?;
        val.i64()
            .ok_or(TableError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to an [`i64`] / [`f64`] [`value`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_i64_path<'a, K, P>(&self, path: P) -> Result<i64, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        val.i64()
            .ok_or(GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not an [`f64`] / [`i64`].
    ///
    /// [`f64`]: enum.Value.html#variant.I64
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_f64<K: AsRef<str>>(&self, key: K) -> Result<f64, TableError> {
        let val = self.get(key)?;
        val.f64()
            .ok_or(TableError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to an [`i64`] / [`f64`] [`value`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.DynArray.html
    pub fn get_f64_path<'a, K, P>(&self, path: P) -> Result<f64, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        val.f64()
            .ok_or(GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`string`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_string<K: AsRef<str>>(&self, key: K) -> Result<LuaString<'lua>, TableError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.string().ok_or(TableError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`string`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to a [`string`] [`value`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_string_path<'a, K, P>(&self, path: P) -> Result<LuaString<'lua>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.string()
            .ok_or(GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not an [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_array<K: AsRef<str>>(&self, key: K) -> Result<LuaArray<'lua>, TableError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.array().ok_or(TableError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to an [`array`] [`value`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    pub fn get_array_path<'a, K, P>(&self, path: P) -> Result<LuaArray<'lua>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.array()
            .ok_or(GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_table<K: AsRef<str>>(&self, key: K) -> Result<LuaTable<'lua>, TableError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.table().ok_or(TableError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to a [`table`](enum.Value.html#variant.Table) [`value`].
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_table_path<'a, K, P>(&self, path: P) -> Result<LuaTable<'lua>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or(GetPathError::IncorrectValueType(val_type))
    }

    /// Returns an iterator over ([`key`], [`value`]) pairs of the [`table`], in unspecified order.
    ///
    /// [`key`]: struct.LuaString.html
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    pub fn iter(&self) -> impl Iterator<Item = (LuaString<'lua>, LuaConfigValue<'lua>)> {
        LuaTableIter(self.0.clone().pairs())
    }

    /// If [`value`] is `Some`, inserts or changes the value at (non-empty) string `key`.
    ///
    /// If [`value`] is `None`, tries to remove the value at `key`.
    /// Returns an [`error`] if the `key` does not exist in this case.
    ///
    /// [`value`]: enum.Value.html
    /// [`error`]: enum.TableError.html
    pub fn set<'s, K, V>(&mut self, key: K, value: V) -> Result<(), TableError>
    where
        K: AsRef<str>,
        V: Into<Option<Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>>>,
    {
        self.set_impl(key.as_ref(), value.into())
    }

    pub(super) fn from_valid_table(table: rlua::Table<'lua>) -> Self {
        Self(table)
    }

    fn len_impl(&self) -> u32 {
        get_table_len(&self.0)
    }

    fn get_impl(&self, key: &str) -> Result<LuaConfigValue<'lua>, TableError> {
        use TableError::*;

        if key.is_empty() {
            return Err(EmptyKey);
        }

        let value: rlua::Value = self.0.get(key).map_err(|_| KeyDoesNotExist)?;

        value_from_lua_value(value).map_err(|err| match err {
            ValueFromLuaValueError::KeyDoesNotExist => KeyDoesNotExist,
            _ => unreachable!(),
        })
    }

    fn set_impl<'s>(
        &mut self,
        key: &str,
        value: Option<Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>>,
    ) -> Result<(), TableError> {
        use TableError::*;

        if key.is_empty() {
            return Err(EmptyKey);
        }

        let contains_key = self.contains_key(key);

        // Add or modify a value - always succeeds.
        if let Some(value) = value {
            match value {
                Value::Bool(value) => self.0.set(key, value).unwrap(),
                Value::F64(value) => self.0.set(key, value).unwrap(),
                Value::I64(value) => self.0.set(key, value).unwrap(),
                Value::String(value) => self.0.set(key, value).unwrap(),
                Value::Array(value) => self.0.set(key, value.0).unwrap(),
                Value::Table(value) => self.0.set(key, value.0).unwrap(),
            }

            // Change table length on value added.
            if !contains_key {
                set_table_len(&self.0, get_table_len(&self.0) + 1);
            }

            Ok(())

        // (Try to) remove a value.
        // Succeeds if key existed.
        } else if contains_key {
            self.0.set(key, rlua::Value::Nil).unwrap();

            // Change table length on value removed.
            let len = self.len_impl();
            debug_assert!(len > 0);
            set_table_len(&self.0, len - 1);

            Ok(())

        // Else tried to remove a non-existant key.
        } else {
            Err(KeyDoesNotExist)
        }
    }

    fn contains_key(&self, key: &str) -> bool {
        if let Ok(value) = self.0.get::<_, rlua::Value<'_>>(key) {
            !matches!(value, rlua::Value::Nil)
        } else {
            false
        }
    }

    fn fmt_lua_impl(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        writeln!(f, "{{")?;

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order.
        keys.sort_by(|l, r| l.as_ref().cmp(r.as_ref()));

        // Iterate the table using the sorted keys.
        for key in keys.into_iter() {
            let key = unsafe { NonEmptyStr::new_unchecked(key.as_ref()) };

            <Self as DisplayLua>::do_indent(f, indent + 1)?;

            write_lua_key(f, key)?;
            " = ".fmt(f)?;

            // Must succeed.
            let value = self.get(key).unwrap();

            let is_array_or_table = matches!(value.get_type(), ValueType::Array | ValueType::Table);

            value.fmt_lua(f, indent + 1)?;

            ",".fmt(f)?;

            if is_array_or_table {
                write!(f, " -- {}", key.as_ref())?;
            }

            writeln!(f)?;
        }

        <Self as DisplayLua>::do_indent(f, indent)?;
        write!(f, "}}")?;

        Ok(())
    }

    #[cfg(feature = "ini")]
    fn fmt_ini_impl<W: Write>(
        &self,
        w: &mut W,
        level: u32,
        array: bool,
        path: &mut IniPath,
        options: ToIniStringOptions,
    ) -> Result<(), ToIniStringError> {
        debug_assert!(options.nested_sections || level < 2);

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order, non-tables first.
        keys.sort_by(|l, r| {
            let l_val = self.get(l.as_ref()).unwrap();
            let r_val = self.get(r.as_ref()).unwrap();

            let l_is_a_table = l_val.get_type() == ValueType::Table;
            let r_is_a_table = r_val.get_type() == ValueType::Table;

            if !l_is_a_table && r_is_a_table {
                std::cmp::Ordering::Less
            } else if l_is_a_table && !r_is_a_table {
                std::cmp::Ordering::Greater
            } else {
                l.as_ref().cmp(r.as_ref())
            }
        });

        let len = self.len() as usize;

        // Iterate the table using the sorted keys.
        for (key_index, key) in keys.into_iter().enumerate() {
            let last = key_index == len - 1;

            let key = unsafe { NonEmptyStr::new_unchecked(key.as_ref()) };

            // Must succeed.
            let value = self.get(key).unwrap();

            match value {
                Value::Array(value) => {
                    write_ini_array(
                        w,
                        key,
                        value.iter(),
                        value.len() as usize,
                        last,
                        level,
                        path,
                        options,
                    )?;
                }
                Value::Table(value) => {
                    write_ini_table(
                        w,
                        key,
                        key_index as u32,
                        &value,
                        value.len(),
                        last,
                        level,
                        path,
                        options,
                    )?;
                }
                value => {
                    write_ini_value(w, key, &value, last, level, array, path, options)?;
                }
            }
        }

        Ok(())
    }
}

/// Iterator over ([`key`], [`value`]) tuples of the [`table`], in unspecified order.
///
/// [`key`]: struct.LuaString.html
/// [`value`]: type.LuaConfigValue.html
/// [`table`]: struct.LuaTable.html
struct LuaTableIter<'lua>(rlua::TablePairs<'lua, rlua::Value<'lua>, rlua::Value<'lua>>);

impl<'lua> std::iter::Iterator for LuaTableIter<'lua> {
    type Item = (LuaString<'lua>, LuaConfigValue<'lua>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pair) = self.0.next() {
            if let Ok((key, value)) = pair {
                // Must succeed - all table keys are valid UTF-8 strings.
                let key = if let rlua::Value::String(key) = key {
                    LuaString::new(key)
                } else {
                    unreachable!();
                };

                // Must succeed.
                let value = value_from_lua_value(value).unwrap();

                Some((key, value))
            } else {
                None // Stop on iteration error (this should never happen?).
            }
        } else {
            None
        }
    }
}

impl<'lua> DisplayLua for LuaTable<'lua> {
    fn fmt_lua(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(f, indent)
    }
}

#[cfg(feature = "ini")]
impl<'lua> DisplayIni for LuaTable<'lua> {
    fn fmt_ini<W: Write>(
        &self,
        w: &mut W,
        level: u32,
        array: bool,
        path: &mut IniPath,
        options: ToIniStringOptions,
    ) -> Result<(), ToIniStringError> {
        self.fmt_ini_impl(w, level, array, path, options)
    }
}

impl<'lua> Display for LuaTable<'lua> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use crate::*;

    #[test]
    fn len_empty_clear() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            assert_eq!(table.len(), 0);
            assert!(table.is_empty());

            table.set("foo", Some(true.into())).unwrap();

            assert_eq!(table.len(), 1);
            assert!(!table.is_empty());

            table.set("bar", Some(7.into())).unwrap();

            assert_eq!(table.len(), 2);
            assert!(!table.is_empty());

            table.clear();

            assert_eq!(table.len(), 0);
            assert!(table.is_empty());
        });
    }

    #[test]
    fn contains() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            assert!(!table.contains("foo"));

            table.set("foo", Some(true.into())).unwrap();

            assert!(table.contains("foo"));

            table.clear();

            assert!(!table.contains("foo"));
        });
    }

    #[test]
    fn LuaTableError_EmptyKey() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            assert_eq!(table.get("").err().unwrap(), TableError::EmptyKey);
            assert_eq!(table.get_bool("").err().unwrap(), TableError::EmptyKey);
            assert_eq!(table.get_i64("").err().unwrap(), TableError::EmptyKey);
            assert_eq!(table.get_f64("").err().unwrap(), TableError::EmptyKey);
            assert_eq!(table.get_string("").err().unwrap(), TableError::EmptyKey);
            assert_eq!(table.get_table("").err().unwrap(), TableError::EmptyKey);
            assert_eq!(table.get_array("").err().unwrap(), TableError::EmptyKey);

            assert_eq!(
                table.set("", Some(true.into())).err().unwrap(),
                TableError::EmptyKey
            );
        });
    }

    #[test]
    fn LuaTableError_KeyDoesNotExist() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            assert_eq!(table.get("foo").err().unwrap(), TableError::KeyDoesNotExist);
            assert_eq!(
                table.get_bool("foo").err().unwrap(),
                TableError::KeyDoesNotExist
            );
            assert_eq!(
                table.get_i64("foo").err().unwrap(),
                TableError::KeyDoesNotExist
            );
            assert_eq!(
                table.get_f64("foo").err().unwrap(),
                TableError::KeyDoesNotExist
            );
            assert_eq!(
                table.get_string("foo").err().unwrap(),
                TableError::KeyDoesNotExist
            );
            assert_eq!(
                table.get_table("foo").err().unwrap(),
                TableError::KeyDoesNotExist
            );
            assert_eq!(
                table.get_array("foo").err().unwrap(),
                TableError::KeyDoesNotExist
            );

            assert_eq!(
                table.set("foo", None).err().unwrap(),
                TableError::KeyDoesNotExist
            );

            // But this works.

            table.set("foo", Some(true.into())).unwrap();

            assert_eq!(table.get("foo").unwrap().bool().unwrap(), true);
        });
    }

    #[test]
    fn LuaTableError_IncorrectValueType() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut table = LuaTable::new(lua);

            table.set("foo", Some(true.into())).unwrap();

            assert_eq!(
                table.get_i64("foo").err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table.get_f64("foo").err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table.get_string("foo").err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table.get_table("foo").err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table.get_array("foo").err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );

            // But this works.

            table.set("bar", Some(3.14.into())).unwrap();

            assert_eq!(table.get_i64("bar").unwrap(), 3);
            assert!(cmp_f64(table.get_f64("bar").unwrap(), 3.14));

            table.set("baz", Some((-7).into())).unwrap();

            assert_eq!(table.get_i64("baz").unwrap(), -7);
            assert!(cmp_f64(table.get_f64("baz").unwrap(), -7.0));
        });
    }

    #[test]
    fn basic() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            // Create an empty table.
            let mut table = LuaTable::new(lua);
            assert_eq!(table.len(), 0);
            assert!(table.is_empty());

            // Add a value.
            assert!(!table.contains("bool"));
            table.set("bool", Some(true.into())).unwrap();
            assert_eq!(table.len(), 1);
            assert!(!table.is_empty());
            assert!(table.contains("bool"));
            assert_eq!(table.get_bool("bool").unwrap(), true);

            // Add a couple more.
            assert!(!table.contains("i64"));
            table.set("i64", Some(7.into())).unwrap();
            assert_eq!(table.len(), 2);
            assert!(!table.is_empty());
            assert!(table.contains("i64"));
            assert_eq!(table.get_i64("i64").unwrap(), 7);

            assert!(!table.contains("string"));
            table.set("string", Some("foo".into())).unwrap();
            assert_eq!(table.len(), 3);
            assert!(!table.is_empty());
            assert!(table.contains("string"));
            assert_eq!(table.get_string("string").unwrap().as_ref(), "foo");

            // Change a value.
            table.set("string", Some("bar".into())).unwrap();
            assert_eq!(table.len(), 3);
            assert!(!table.is_empty());
            assert!(table.contains("string"));
            assert_eq!(table.get_string("string").unwrap().as_ref(), "bar");

            // Remove a value.
            table.set("bool", None).unwrap();
            assert_eq!(table.len(), 2);
            assert!(!table.is_empty());
            assert!(!table.contains("bool"));

            // Add a nested table with some values.
            let mut nested_table = LuaTable::new(lua);
            assert_eq!(nested_table.len(), 0);
            assert!(nested_table.is_empty());

            assert!(!nested_table.contains("nested_bool"));
            nested_table.set("nested_bool", Some(false.into())).unwrap();
            assert!(nested_table.contains("nested_bool"));

            assert!(!nested_table.contains("nested_int"));
            nested_table.set("nested_int", Some((-9).into())).unwrap();
            assert!(nested_table.contains("nested_int"));

            assert_eq!(nested_table.len(), 2);
            assert!(!nested_table.is_empty());

            assert!(!table.contains("table"));
            table.set("table", Some(nested_table.into())).unwrap();
            assert_eq!(table.len(), 3);
            assert!(!table.is_empty());
            assert!(table.contains("table"));

            assert_eq!(
                table
                    .get_path(&["table".into(), "nested_bool".into()])
                    .unwrap()
                    .bool()
                    .unwrap(),
                false
            );
            assert_eq!(
                table
                    .get_bool_path(&["table".into(), "nested_bool".into()])
                    .unwrap(),
                false
            );
            assert_eq!(
                table
                    .get_path(&["table".into(), "nested_int".into()])
                    .unwrap()
                    .i64()
                    .unwrap(),
                -9
            );
            assert_eq!(
                table
                    .get_i64_path(&["table".into(), "nested_int".into()])
                    .unwrap(),
                -9
            );
            assert!(cmp_f64(
                table
                    .get_path(&["table".into(), "nested_int".into()])
                    .unwrap()
                    .f64()
                    .unwrap(),
                -9.0
            ));
            assert!(cmp_f64(
                table
                    .get_f64_path(&["table".into(), "nested_int".into()])
                    .unwrap(),
                -9.0
            ));

            // Add a nested array with some values.
            let mut nested_array = LuaArray::new(lua);
            assert_eq!(nested_array.len(), 0);
            assert!(nested_array.is_empty());

            nested_array.push(3.14.into()).unwrap();
            nested_array.push(42.0.into()).unwrap();
            nested_array.push((-17.235).into()).unwrap();
            assert_eq!(nested_array.len(), 3);
            assert!(!nested_array.is_empty());

            assert!(!table.contains("array"));
            table.set("array", Value::Array(nested_array)).unwrap();
            assert_eq!(table.len(), 4);
            assert!(!table.is_empty());
            assert!(table.contains("array"));

            assert_eq!(table.get_i64_path(&["array".into(), 0.into()]).unwrap(), 3);
            assert!(cmp_f64(
                table.get_f64_path(&["array".into(), 0.into()]).unwrap(),
                3.14
            ));

            assert_eq!(table.get_i64_path(&["array".into(), 1.into()]).unwrap(), 42);
            assert!(cmp_f64(
                table.get_f64_path(&["array".into(), 1.into()]).unwrap(),
                42.0
            ));

            assert_eq!(
                table.get_i64_path(&["array".into(), 2.into()]).unwrap(),
                -17
            );
            assert!(cmp_f64(
                table.get_f64_path(&["array".into(), 2.into()]).unwrap(),
                -17.235
            ));

            // Iterate the table.
            for (key, value) in table.iter() {
                match key.as_ref() {
                    "i64" => assert_eq!(value.i64().unwrap(), 7),
                    "string" => assert_eq!(value.string().unwrap().as_ref(), "bar"),
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
        });
    }
}
