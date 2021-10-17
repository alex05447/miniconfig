use {
    super::{array_or_table::BinArrayOrTable, value::BinConfigUnpackedValue},
    crate::{util::*, *},
    std::{
        borrow::Borrow,
        fmt::{Display, Formatter, Write},
        iter::Iterator,
    },
};

/// Represents an immutable hash map / table of [`Value`]'s with (non-empty) string keys.
///
/// [`Value`]: enum.Value.html
pub struct BinTable<'t>(pub(super) BinArrayOrTable<'t>);

impl<'t> BinTable<'t> {
    /// Returns the number of entries in the [`table`].
    ///
    /// [`table`]: struct.BinTable.html
    pub fn len(&self) -> u32 {
        self.0.len
    }

    /// Returns `true` if the [`table`] is empty.
    ///
    /// [`table`]: struct.BinTable.html
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the [`table`] contains a [`value`] with the (non-empty) string `key`.
    ///
    /// [`table`]: struct.BinTable.html
    /// [`value`]: type.BinConfigValue.html
    pub fn contains<'a>(&self, key: TableKey<'a>) -> bool {
        use TableError::*;

        match self.get_val(key.as_ref().into()) {
            Ok(_) => true,
            Err(err) => match err {
                EmptyKey | KeyDoesNotExist => false,
                IncorrectValueType(_) => {
                    debug_unreachable!("`get_val()` does not return `IncorrectValueType(_)`")
                }
            },
        }
    }

    /// Tries to get a reference to a [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty or if the [`table`] does not contain the `key`.
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_val<'k>(&self, key: TableKey<'k>) -> Result<BinConfigValue<'t>, TableError> {
        let ne_key = NonEmptyStr::new(key.as_str()).ok_or_else(|| TableError::EmptyKey)?;
        self.get_impl(ne_key, key.key_hash())
    }

    /// Tries to get a reference to a [`value`] in the [`table`] with the (non-empty) string `key`,
    /// and convert it to the user-requested type [`convertible`](TryFromValue) from a [`value`].
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key`,
    /// or if the [`value`] is of incorrect and incompatible type.
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get<'k, V: TryFromValue<&'t str, BinArray<'t>, BinTable<'t>>>(
        &self,
        key: TableKey<'k>,
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    /// [`type`]: enum.ValueType.html
    pub fn get_val_path<'k, K, P>(&self, path: P) -> Result<BinConfigValue<'t>, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        BinConfigValue::Table(BinTable(self.0.clone()))
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    /// [`type`]: enum.ValueType.html
    pub fn get_path<'k, K, P, V>(&self, path: P) -> Result<V, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
        V: TryFromValue<&'t str, BinArray<'t>, BinTable<'t>>,
    {
        V::try_from(self.get_val_path(path)?).map_err(GetPathError::IncorrectValueType)
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_bool<'k>(&self, key: TableKey<'k>) -> Result<bool, TableError> {
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.BinArray.html
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_i64<'k>(&self, key: TableKey<'k>) -> Result<i64, TableError> {
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.BinArray.html
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
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_f64<'k>(&self, key: TableKey<'k>) -> Result<f64, TableError> {
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.BinArray.html
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_string<'k>(&self, key: TableKey<'k>) -> Result<&str, TableError> {
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_string_path<'k, K, P>(&self, path: P) -> Result<&str, GetPathError>
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_array<'k>(&self, key: TableKey<'k>) -> Result<BinArray<'t>, TableError> {
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
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    pub fn get_array_path<'k, K, P>(&self, path: P) -> Result<BinArray<'t>, GetPathError>
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_table<'k>(&self, key: TableKey<'k>) -> Result<BinTable<'t>, TableError> {
        self.get(key)
    }

    /// Tries to get a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to a [`table`](enum.Value.html#variant.Table) [`value`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_table_path<'k, K, P>(&self, path: P) -> Result<BinTable<'t>, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
    }

    /// Returns an iterator over (`key`, [`value`]) pairs of the [`table`], in unspecified order.
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    pub fn iter<'i>(&'i self) -> impl Iterator<Item = (&'t NonEmptyStr, BinConfigValue<'t>)> + 'i {
        BinTableIter::new(self)
    }

    pub(super) fn new(table: BinArrayOrTable<'t>) -> Self {
        Self(table)
    }

    pub(super) fn get_impl(
        &self,
        key: &NonEmptyStr,
        hash: u32,
    ) -> Result<BinConfigValue<'t>, TableError> {
        use TableError::*;

        (0..self.len())
            .find_map(|idx| {
                // Safe to call - the config was validated.
                let (table_key, value) = unsafe { self.0.key_and_value(idx) };

                // Compare the hashes first.
                if table_key.hash == hash {
                    // Hashes match - compare the strings.

                    // Safe to call - the config was validated.
                    let table_key = unsafe { self.0.key_ofset_and_len(table_key.index) };

                    // Safe to call - the key string was validated.
                    if key == unsafe { self.0.string(table_key.offset(), table_key.len()) } {
                        Some(self.get_value(value))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .ok_or_else(|| KeyDoesNotExist)
    }

    fn get_value(&self, value: BinConfigUnpackedValue) -> BinConfigValue<'t> {
        use BinConfigUnpackedValue::*;

        match value {
            Bool(val) => Value::Bool(val),
            I64(val) => Value::I64(val),
            F64(val) => Value::F64(val),
            BinConfigUnpackedValue::String { offset, len } => {
                // Safe to call - the string was validated.
                Value::String(unsafe { self.0.string(offset, len) })
            }
            Array { offset, len } => Value::Array(BinArray::new(BinArrayOrTable::new(
                self.0.base,
                self.0.key_table,
                offset,
                len,
            ))),
            Table { offset, len } => Value::Table(BinTable::new(BinArrayOrTable::new(
                self.0.base,
                self.0.key_table,
                offset,
                len,
            ))),
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
            <Self as DisplayLua>::do_indent(w, indent + 1)?;

            write_lua_key(w, key)?;
            write!(w, " = ")?;

            // Must succeed - all keys are valid.
            let value = unwrap_unchecked(
                self.get_val(TableKey::String(key.into())),
                "failed to get a value from a bin config table with a valid key",
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
        keys.sort_by(|&l, &r| {
            // Must succeed - all keys are valid.
            let l_val = unwrap_unchecked(
                self.get_val(TableKey::String(l.into())),
                "failed to get a value from a bin config table with a valid key",
            );
            let r_val = unwrap_unchecked(
                self.get_val(TableKey::String(r.into())),
                "failed to get a value from a bin config table with a valid key",
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

            // Must succeed - all keys are valid.
            let value = unwrap_unchecked(
                self.get_val(TableKey::String(key.into())),
                "failed to get a value from a bin config table with a valid key",
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

/// Iterator over (`key`, [`value`]) pairs of the [`table`], in unspecified order.
///
/// [`value`]: type.BinConfigValue.html
/// [`table`]: struct.BinTable.html
struct BinTableIter<'i, 't> {
    table: &'i BinTable<'t>,
    index: u32,
}

impl<'i, 't> BinTableIter<'i, 't> {
    fn new(table: &'i BinTable<'t>) -> Self {
        Self { table, index: 0 }
    }
}

impl<'i, 't> Iterator for BinTableIter<'i, 't> {
    type Item = (&'t NonEmptyStr, BinConfigValue<'t>);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;

        if index < self.table.len() {
            self.index += 1;

            // Safe to call - the config was validated.
            let (key, value) = unsafe { self.table.0.key_and_value(index) };

            // Safe to call - the config was validated.
            let key = unsafe { self.table.0.key_ofset_and_len(key.index) };

            // Safe to call - the key string was validated.
            let key = unwrap_unchecked(
                NonEmptyStr::new(unsafe { self.table.0.string(key.offset(), key.len()) }),
                "empty key",
            );

            let value = self.table.get_value(value);

            Some((key, value))
        } else {
            None
        }
    }
}

impl<'t> DisplayLua for BinTable<'t> {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(w, indent)
    }
}

#[cfg(feature = "ini")]
impl<'t> DisplayIni for BinTable<'t> {
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

impl<'t> Display for BinTable<'t> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use {crate::*, ministr_macro::nestr, std::num::NonZeroU32};

    #[test]
    fn BinTableError_EmptyKey() {
        let mut writer = BinConfigWriter::new(NonZeroU32::new(1).unwrap()).unwrap();
        writer.bool(nestr!("bool"), true).unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config.root().get_bool("".into()).err().unwrap(),
            TableError::EmptyKey
        );

        // But this works.

        assert_eq!(config.root().get_bool("bool".into()).unwrap(), true);

        #[cfg(feature = "str_hash")]
        {
            assert_eq!(config.root().get_bool(key!("bool")).unwrap(), true);
        }
    }

    #[test]
    fn BinTableError_KeyDoesNotExist() {
        let mut writer = BinConfigWriter::new(NonZeroU32::new(1).unwrap()).unwrap();
        writer.bool(nestr!("bool"), true).unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config.root().get_bool("missing".into()).err().unwrap(),
            TableError::KeyDoesNotExist
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_bool(key!("missing")).err().unwrap(),
                TableError::KeyDoesNotExist
            );
        }

        // But this works.

        assert_eq!(config.root().get_bool("bool".into()).unwrap(), true);

        #[cfg(feature = "str_hash")]
        {
            assert_eq!(config.root().get_bool(key!("bool")).unwrap(), true);
        }
    }

    #[test]
    fn BinTableError_IncorrectValueType() {
        let mut writer = BinConfigWriter::new(NonZeroU32::new(2).unwrap()).unwrap();

        writer.f64(nestr!("f64"), 3.14).unwrap();
        writer.string(nestr!("string"), "foo").unwrap();

        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();
        let root = config.root();

        assert_eq!(
            config.root().get_bool("f64".into()).err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );
        let val: Result<bool, _> = config.root().get("f64".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_bool(key!("f64")).err().unwrap(),
                TableError::IncorrectValueType(ValueType::F64)
            );
        }
        assert_eq!(
            config.root().get_string("f64".into()).err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );
        let val: Result<&str, _> = root.get("f64".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );
        let val: Result<String, _> = config.root().get("f64".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_string(key!("f64")).err().unwrap(),
                TableError::IncorrectValueType(ValueType::F64)
            );
        }
        assert_eq!(
            config.root().get_array("f64".into()).err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );
        let val: Result<BinArray<'_>, _> = root.get("f64".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_array(key!("f64")).err().unwrap(),
                TableError::IncorrectValueType(ValueType::F64)
            );
        }
        assert_eq!(
            config.root().get_table("f64".into()).err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );
        let val: Result<BinTable<'_>, _> = root.get("f64".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_table(key!("f64")).err().unwrap(),
                TableError::IncorrectValueType(ValueType::F64)
            );
        }

        assert_eq!(
            config.root().get_bool("string".into()).err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        let val: Result<bool, _> = config.root().get("string".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_bool(key!("string")).err().unwrap(),
                TableError::IncorrectValueType(ValueType::String)
            );
        }
        assert_eq!(
            config.root().get_f64("string".into()).err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        let val: Result<f64, _> = root.get("string".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_f64(key!("string")).err().unwrap(),
                TableError::IncorrectValueType(ValueType::String)
            );
        }
        assert_eq!(
            config.root().get_i64("string".into()).err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        let val: Result<i64, _> = root.get("string".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_i64(key!("string")).err().unwrap(),
                TableError::IncorrectValueType(ValueType::String)
            );
        }
        assert_eq!(
            config.root().get_array("string".into()).err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        let val: Result<BinArray<'_>, _> = root.get("string".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_array(key!("string")).err().unwrap(),
                TableError::IncorrectValueType(ValueType::String)
            );
        }
        assert_eq!(
            config.root().get_table("string".into()).err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        let val: Result<BinTable<'_>, _> = root.get("string".into());
        assert_eq!(
            val.err().unwrap(),
            TableError::IncorrectValueType(ValueType::String)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_table(key!("string")).err().unwrap(),
                TableError::IncorrectValueType(ValueType::String)
            );
        }

        // But this works.

        assert!(cmp_f64(config.root().get_f64("f64".into()).unwrap(), 3.14));
        #[cfg(feature = "str_hash")]
        {
            assert!(cmp_f64(config.root().get_f64(key!("f64")).unwrap(), 3.14));
        }
        assert_eq!(config.root().get_i64("f64".into()).unwrap(), 3);
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(config.root().get_i64(key!("f64")).unwrap(), 3);
        }
        assert_eq!(config.root().get_string("string".into()).unwrap(), "foo");
        let string: &str = root.get("string".into()).unwrap();
        assert_eq!(string, "foo");
        let string: String = root.get("string".into()).unwrap();
        assert_eq!(string, "foo");
    }
}
