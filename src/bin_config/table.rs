use {
    super::{
        array_or_table::BinArrayOrTable, util::string_hash_fnv1a, value::BinConfigUnpackedValue,
    },
    crate::{
        util::{write_lua_key, DisplayLua},
        BinArray, BinConfigValue, ConfigKey, GetPathError, TableError, Value, ValueType,
    },
    std::{
        borrow::Borrow,
        fmt::{Display, Formatter},
        iter::Iterator,
    },
};

#[cfg(feature = "ini")]
use {
    crate::{write_ini_key, write_ini_section, DisplayIni, ToIniStringError, ToIniStringOptions},
    std::fmt::Write,
};

/// Represents an immutable map of [`Value`]'s with (non-empty) string keys.
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
    pub fn contains<K: AsRef<str>>(&self, key: K) -> bool {
        use TableError::*;

        match self.get(key) {
            Ok(_) => true,
            Err(err) => match err {
                EmptyKey | KeyDoesNotExist => false,
                IncorrectValueType(_) => unreachable!(),
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
    pub fn get<K: AsRef<str>>(&self, key: K) -> Result<BinConfigValue<'t>, TableError> {
        self.get_impl(key.as_ref())
    }

    /// Tries to get an immutable reference to a [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns the [`table`] itself if the `path` is empty.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    /// [`type`]: enum.ValueType.html
    pub fn get_path<'a, K, P>(&self, path: P) -> Result<BinConfigValue<'t>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        BinConfigValue::Table(BinTable(self.0.clone()))
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.BinArray.html
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.BinArray.html
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
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.BinArray.html
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_string<K: AsRef<str>>(&self, key: K) -> Result<&'t str, TableError> {
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
    /// [`string`]: enum.Value.html#variant.I64
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_string_path<'a, K, P>(&self, path: P) -> Result<&str, GetPathError<'a>>
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_array<K: AsRef<str>>(&self, key: K) -> Result<BinArray<'t>, TableError> {
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
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    pub fn get_array_path<'a, K, P>(&self, path: P) -> Result<BinArray<'t>, GetPathError<'a>>
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
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_table<K: AsRef<str>>(&self, key: K) -> Result<BinTable<'t>, TableError> {
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
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_table_path<'a, K, P>(&self, path: P) -> Result<BinTable<'t>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or(GetPathError::IncorrectValueType(val_type))
    }

    /// Returns an iterator over (`key`, [`value`]) pairs of the [`table`], in unspecified order.
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    pub fn iter<'i>(&'i self) -> impl Iterator<Item = (&'t str, BinConfigValue<'t>)> + 'i {
        BinTableIter::new(self)
    }

    pub(super) fn new(table: BinArrayOrTable<'t>) -> Self {
        Self(table)
    }

    fn get_impl(&self, key: &str) -> Result<BinConfigValue<'t>, TableError> {
        use TableError::*;

        if key.is_empty() {
            return Err(EmptyKey);
        }

        // Hash the key string to compare against table keys.
        let key_hash = string_hash_fnv1a(key);

        // For all table elements in order.
        for index in 0..self.len() {
            // Safe to call - the config was validated.
            let (value_key, value) = unsafe { self.0.key_and_value(index) };

            // Compare the hashes first.
            if value_key.hash == key_hash {
                // Hashes match - compare the strings.
                // Safe to call - the key string was validated.
                if key == unsafe { self.0.string(value_key.offset, value_key.len) } {
                    return Ok(self.get_value(value));
                }
            }
        }

        Err(KeyDoesNotExist)
    }

    fn get_value(&self, value: BinConfigUnpackedValue) -> BinConfigValue<'t> {
        use BinConfigUnpackedValue::*;

        match value {
            Bool(val) => Value::Bool(val),
            I64(val) => Value::I64(val),
            F64(val) => Value::F64(val),
            BinConfigUnpackedValue::String { offset, len } => {
                Value::String(unsafe { self.0.string(offset, len) })
            } // Safe to call - the string was validated.
            Array { offset, len } => Value::Array(BinArray::new(BinArrayOrTable::new(
                self.0.base,
                offset,
                len,
            ))),
            Table { offset, len } => Value::Table(BinTable::new(BinArrayOrTable::new(
                self.0.base,
                offset,
                len,
            ))),
        }
    }

    fn fmt_lua_impl(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        writeln!(f, "{{")?;

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order.
        keys.sort_by(|l, r| l.cmp(r));

        // Iterate the table using the sorted keys.
        for key in keys.into_iter() {
            <Self as DisplayLua>::do_indent(f, indent + 1)?;

            write_lua_key(f, key)?;
            " = ".fmt(f)?;

            // Must succeed.
            let value = self.get(key).unwrap();

            let is_array_or_table = matches!(value.get_type(), ValueType::Array | ValueType::Table);

            value.fmt_lua(f, indent + 1)?;

            ",".fmt(f)?;

            if is_array_or_table {
                write!(f, " -- {}", key)?;
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
        _array: bool,
        options: ToIniStringOptions,
    ) -> Result<(), ToIniStringError> {
        use ToIniStringError::*;

        debug_assert!(level < 2);

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order, non-tables first.
        keys.sort_by(|l, r| {
            let l_val = self.get(*l).unwrap();
            let r_val = self.get(*r).unwrap();

            let l_is_a_table = l_val.get_type() == ValueType::Table;
            let r_is_a_table = r_val.get_type() == ValueType::Table;

            if !l_is_a_table && r_is_a_table {
                std::cmp::Ordering::Less
            } else if l_is_a_table && !r_is_a_table {
                std::cmp::Ordering::Greater
            } else {
                l.cmp(r)
            }
        });

        let len = self.len() as usize;

        // Iterate the table using the sorted keys.
        for (key_index, key) in keys.into_iter().enumerate() {
            let last = key_index == len - 1;

            // Must succeed.
            let value = self.get(key).unwrap();

            match value {
                Value::Array(value) => {
                    if options.arrays {
                        let len = value.len() as usize;

                        write_ini_key(w, key, options.escape)?;

                        write!(w, " = [").map_err(|_| WriteError)?;

                        for (array_index, array_value) in value.iter().enumerate() {
                            let last = array_index == len - 1;

                            array_value.fmt_ini(w, level + 1, true, options)?;

                            if !last {
                                write!(w, ", ").map_err(|_| WriteError)?;
                            }
                        }

                        write!(w, "]").map_err(|_| WriteError)?;

                        if !last {
                            writeln!(w).map_err(|_| WriteError)?;
                        }
                    } else {
                        return Err(ArraysNotAllowed);
                    }
                }
                Value::Table(value) => {
                    if level >= 1 {
                        return Err(NestedTablesNotSupported);
                    }

                    if key_index > 0 {
                        writeln!(w).map_err(|_| WriteError)?;
                    }

                    write_ini_section(w, key, options.escape)?;

                    if value.len() > 0 {
                        writeln!(w).map_err(|_| WriteError)?;
                        value.fmt_ini(w, level + 1, false, options)?;
                    }

                    if !last {
                        writeln!(w).map_err(|_| WriteError)?;
                    }
                }
                value => {
                    write_ini_key(w, key, options.escape)?;

                    write!(w, " = ").map_err(|_| WriteError)?;

                    value.fmt_ini(w, level + 1, false, options)?;

                    if !last {
                        writeln!(w).map_err(|_| WriteError)?;
                    }
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
    type Item = (&'t str, BinConfigValue<'t>);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;

        if index < self.table.len() {
            self.index += 1;

            // Safe to call - the config was validated.
            let (key, value) = unsafe { self.table.0.key_and_value(index) };

            // Safe to call - the key string was validated.
            let key = unsafe { self.table.0.string(key.offset, key.len) };

            let value = self.table.get_value(value);

            Some((key, value))
        } else {
            None
        }
    }
}

impl<'t> DisplayLua for BinTable<'t> {
    fn fmt_lua(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(f, indent)
    }
}

#[cfg(feature = "ini")]
impl<'t> DisplayIni for BinTable<'t> {
    fn fmt_ini<W: Write>(
        &self,
        w: &mut W,
        level: u32,
        array: bool,
        options: ToIniStringOptions,
    ) -> Result<(), ToIniStringError> {
        self.fmt_ini_impl(w, level, array, options)
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

    use crate::*;

    #[test]
    fn BinTableError_EmptyKey() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.bool("bool", true).unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config.root().get_bool("").err().unwrap(),
            TableError::EmptyKey
        );

        // But this works.

        assert_eq!(config.root().get_bool("bool").unwrap(), true);
    }

    #[test]
    fn BinTableError_KeyDoesNotExist() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.bool("bool", true).unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config.root().get_bool("missing").err().unwrap(),
            TableError::KeyDoesNotExist
        );

        // But this works.

        assert_eq!(config.root().get_bool("bool").unwrap(), true);
    }

    #[test]
    fn BinTableError_IncorrectValueType() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.f64("f64", 3.14).unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config.root().get_bool("f64").err().unwrap(),
            TableError::IncorrectValueType(ValueType::F64)
        );

        // But this works.

        assert!(cmp_f64(config.root().get_f64("f64").unwrap(), 3.14));
        assert_eq!(config.root().get_i64("f64").unwrap(), 3);
    }
}
