use std::fmt::{Display, Formatter};
use std::iter::Iterator;

#[cfg(feature = "ini")]
use std::fmt::Write;

use crate::{
    util::{write_lua_key, DisplayLua},
    BinArray, BinConfigValue, BinTableGetError, Value,
};

#[cfg(feature = "ini")]
use crate::{
    write_ini_key, write_ini_section, DisplayIni, ToIniStringError, ToIniStringOptions, ValueType,
};

use super::array_or_table::BinArrayOrTable;
use super::util::string_hash_fnv1a;
use super::value::BinConfigUnpackedValue;

/// Represents an immutable map of [`Value`]'s with string keys.
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

    /// Tries to get a reference to a [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key`.
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: struct.BinTableGetError.html
    pub fn get<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<BinConfigValue<'t>, BinTableGetError> {
        self.get_impl(key.into())
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: struct.BinTableGetError.html
    pub fn get_bool<'k, K: Into<&'k str>>(&self, key: K) -> Result<bool, BinTableGetError> {
        let val = self.get(key)?;
        val.bool()
            .ok_or(BinTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`i64`].
    ///
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: struct.BinTableGetError.html
    pub fn get_i64<'k, K: Into<&'k str>>(&self, key: K) -> Result<i64, BinTableGetError> {
        let val = self.get(key)?;
        val.i64()
            .ok_or(BinTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`f64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: struct.BinTableGetError.html
    pub fn get_f64<'k, K: Into<&'k str>>(&self, key: K) -> Result<f64, BinTableGetError> {
        let val = self.get(key)?;
        val.f64()
            .ok_or(BinTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`string`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: struct.BinTableGetError.html
    pub fn get_string<'k, K: Into<&'k str>>(&self, key: K) -> Result<&'t str, BinTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.string()
            .ok_or(BinTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: struct.BinTableGetError.html
    pub fn get_array<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<BinArray<'t>, BinTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.array()
            .ok_or(BinTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: struct.BinTable.html
    /// [`error`]: struct.BinTableGetError.html
    pub fn get_table<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<BinTable<'t>, BinTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.table()
            .ok_or(BinTableGetError::IncorrectValueType(val_type))
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

    fn get_impl(&self, key: &str) -> Result<BinConfigValue<'t>, BinTableGetError> {
        use BinTableGetError::*;

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

        // Sort the keys.
        keys.sort_by(|l, r| l.cmp(r));

        // Iterate the table using the sorted keys.
        for key in keys.into_iter() {
            <Self as DisplayLua>::do_indent(f, indent + 1)?;

            write_lua_key(f, key)?;
            write!(f, " = ")?;

            // Must succeed.
            let value = self.get(key).unwrap();

            let is_array_or_table = match value {
                Value::Table(_) | Value::Array(_) => true,
                _ => false,
            };

            value.fmt_lua(f, indent + 1)?;

            write!(f, ",")?;

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

impl<'t> Display for BinTable<'t> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
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
