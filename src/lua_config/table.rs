use {
    super::util::*,
    crate::{util::*, *},
    rlua::Context,
    std::{
        borrow::Borrow,
        convert::TryInto,
        fmt::{Display, Formatter, Write},
    },
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

        match self.get_val(key) {
            Ok(_) => true,
            Err(err) => match err {
                KeyDoesNotExist | EmptyKey => false,
                IncorrectValueType(_) => {
                    debug_unreachable!("`get_val()` does not return `IncorrectValueType(_)`")
                }
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
    pub fn get_val<K: AsRef<str>>(&self, key: K) -> Result<LuaConfigValue<'lua>, TableError> {
        self.get_impl(key.as_ref().try_into().map_err(|_| TableError::EmptyKey)?)
    }

    /// Tries to get a reference to a [`value`] in the [`table`] with the (non-empty) string `key`,
    /// and convert it to the user-requested type [`convertible`](TryFromValue) from a [`value`].
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key`,
    /// or if the [`value`] is of incorrect and incompatible type.
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`error`]: enum.TableError.html
    pub fn get<K: AsRef<str>, V: TryFromValue<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>>>(
        &self,
        key: K,
    ) -> Result<V, TableError> {
        V::try_from(self.get_val(key)?).map_err(TableError::IncorrectValueType)
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
    pub fn get_val_path<'k, K, P>(&self, path: P) -> Result<LuaConfigValue<'lua>, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        LuaConfigValue::Table(self.clone())
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a reference to a [`value`] in the [`table`] at `path`,
    /// and convert it to the user-requested type [`convertible`](TryFromValue) from a [`value`].
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
    pub fn get_path<'k, K, P, V>(&self, path: P) -> Result<V, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
        V: TryFromValue<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>>,
    {
        V::try_from(self.get_val_path(path)?).map_err(GetPathError::IncorrectValueType)
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
        self.get(key)
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
    pub fn get_bool_path<'k, K, P>(&self, path: P) -> Result<bool, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
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
        self.get(key)
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
    pub fn get_i64_path<'k, K, P>(&self, path: P) -> Result<i64, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
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
        self.get(key)
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
    pub fn get_f64_path<'k, K, P>(&self, path: P) -> Result<f64, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
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
        self.get(key)
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
    pub fn get_string_path<'k, K, P>(&self, path: P) -> Result<LuaString<'lua>, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
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
        self.get(key)
    }

    /// Tries to get an [`array`] [`value`] in the [`table`] at `path`.
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
    pub fn get_array_path<'k, K, P>(&self, path: P) -> Result<LuaArray<'lua>, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
    }

    /// Tries to get a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`table`]: struct.LuaTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_table<K: AsRef<str>>(&self, key: K) -> Result<LuaTable<'lua>, TableError> {
        self.get(key)
    }

    /// Tries to get a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] at `path`.
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
    pub fn get_table_path<'k, K, P>(&self, path: P) -> Result<LuaTable<'lua>, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
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

    pub(crate) fn get_impl(&self, key: &NonEmptyStr) -> Result<LuaConfigValue<'lua>, TableError> {
        use TableError::*;

        let value: rlua::Value = self.0.raw_get(key.as_str()).map_err(|_| KeyDoesNotExist)?;

        value_from_lua_value(value).map_err(|err| match err {
            ValueFromLuaValueError::KeyDoesNotExist => KeyDoesNotExist,
            ValueFromLuaValueError::InvalidValueType(_) => {
                debug_unreachable!("invalid type values may not exist in a valid Lua config table")
            }
        })
    }

    /// The caller guarantees `key` and `value` are valid.
    fn set_table_value<'s>(
        table: &rlua::Table<'lua>,
        key: &str,
        value: Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>,
    ) {
        // Must succeed - key and value are valid.
        unwrap_unchecked(
            match value {
                Value::Bool(value) => table.raw_set(key, value),
                Value::F64(value) => table.raw_set(key, value),
                Value::I64(value) => table.raw_set(key, value),
                Value::String(value) => table.raw_set(key, value),
                Value::Array(value) => table.raw_set(key, value.0),
                Value::Table(value) => table.raw_set(key, value.0),
            },
            "failed to set a value in the Lua table",
        );
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
            Self::set_table_value(&self.0, key, value);

            // Change table length on value added.
            if !contains_key {
                set_table_len(&self.0, get_table_len(&self.0) + 1);
            }

            Ok(())

        // (Try to) remove a value.
        // Succeeds if key existed.
        } else if contains_key {
            // Must succeed.
            unwrap_unchecked(
                self.0.raw_set(key, rlua::Value::Nil),
                "failed to set a value in the Lua table",
            );

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
        if let Ok(value) = self.0.raw_get::<_, rlua::Value<'_>>(key) {
            !matches!(value, rlua::Value::Nil)
        } else {
            false
        }
    }

    fn fmt_lua_impl<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        writeln!(w, "{{")?;

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order.
        keys.sort_by(|l, r| l.as_ref().cmp(r.as_ref()));

        // Iterate the table using the sorted keys.
        for key in keys.into_iter() {
            let key = unwrap_unchecked(NonEmptyStr::new(key.as_ref()), "empty key");

            <Self as DisplayLua>::do_indent(w, indent + 1)?;

            write_lua_key(w, key)?;
            write!(w, " = ")?;

            // Must succeed - all keys are valid.
            let value = unwrap_unchecked(
                self.get_val(key),
                "failed to get a value from a Lua config table with a valid key",
            );

            let is_array_or_table = matches!(value.get_type(), ValueType::Array | ValueType::Table);

            value.fmt_lua(w, indent + 1)?;

            write!(w, ",")?;

            if is_array_or_table {
                write!(w, " -- {}", key.as_ref())?;
            }

            writeln!(w)?;
        }

        <Self as DisplayLua>::do_indent(w, indent)?;
        write!(w, "}}")?;

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
        debug_assert!(options.nested_sections() || level < 2);

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order, non-tables first.
        keys.sort_by(|l, r| {
            // Must succeed - all keys are valid.
            let l_val = unwrap_unchecked(
                self.get_val(l.as_ref()),
                "failed to get a value from a Lua config table with a valid key",
            );
            let r_val = unwrap_unchecked(
                self.get_val(r.as_ref()),
                "failed to get a value from a Lua config table with a valid key",
            );

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

            let key = unwrap_unchecked(NonEmptyStr::new(key.as_ref()), "empty key");

            // Must succeed - all keys are valid.
            let value = unwrap_unchecked(
                self.get_val(key),
                "failed to get a value from a Lua config table with a valid key",
            );

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
                    let has_non_tables = value
                        .iter()
                        .any(|(_, val)| val.get_type() != ValueType::Table);

                    write_ini_table(
                        w,
                        key,
                        key_index as u32,
                        &value,
                        value.len(),
                        has_non_tables,
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
                    debug_unreachable!("expected a string table key");
                };

                // Must succeed - the table only contains valid values.
                let value = unwrap_unchecked(
                    value_from_lua_value(value),
                    "failed to get a value from a Lua config table with a valid key",
                );

                Some((key, value))
            } else {
                debug_assert!(false, "unexpected error when iterating a Lua table");
                None // Stop on iteration error (this should never happen?).
            }
        } else {
            None
        }
    }
}

impl<'lua> DisplayLua for LuaTable<'lua> {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(w, indent)
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

            assert_eq!(table.get_val("").err().unwrap(), TableError::EmptyKey);
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

            assert_eq!(
                table.get_val("foo").err().unwrap(),
                TableError::KeyDoesNotExist
            );
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

            assert_eq!(table.get_val("foo").unwrap().bool().unwrap(), true);
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
            let val: Result<i64, _> = table.get("foo");
            assert_eq!(
                val.err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table.get_f64("foo").err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            let val: Result<f64, _> = table.get("foo");
            assert_eq!(
                val.err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table.get_string("foo").err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            let val: Result<LuaString<'_>, _> = table.get("foo");
            assert_eq!(
                val.err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table.get_table("foo").err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            let val: Result<LuaTable<'_>, _> = table.get("foo");
            assert_eq!(
                val.err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                table.get_array("foo").err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );
            let val: Result<LuaArray<'_>, _> = table.get("foo");
            assert_eq!(
                val.err().unwrap(),
                TableError::IncorrectValueType(ValueType::Bool)
            );

            // But this works.

            assert_eq!(table.get_bool("foo").unwrap(), true);
            let val: bool = table.get("foo").unwrap();
            assert_eq!(val, true);

            table.set("bar", Some(3.14.into())).unwrap();

            assert_eq!(table.get_i64("bar").unwrap(), 3);
            let val: i64 = table.get("bar").unwrap();
            assert_eq!(val, 3);

            assert!(cmp_f64(table.get_f64("bar").unwrap(), 3.14));
            let val: f64 = table.get("bar").unwrap();
            assert!(cmp_f64(val, 3.14));

            table.set("baz", Some((-7).into())).unwrap();

            assert_eq!(table.get_i64("baz").unwrap(), -7);
            let val: i64 = table.get("baz").unwrap();
            assert_eq!(val, -7);

            assert!(cmp_f64(table.get_f64("baz").unwrap(), -7.0));
            let val: f64 = table.get("baz").unwrap();
            assert!(cmp_f64(val, -7.0));
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
                    .get_val_path(&["table".into(), "nested_bool".into()])
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
                    .get_val_path(&["table".into(), "nested_int".into()])
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
                    .get_val_path(&["table".into(), "nested_int".into()])
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
