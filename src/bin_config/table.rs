use std::fmt::{Display, Formatter};
use std::iter::Iterator;

use super::array_or_table::BinArrayOrTable;
use super::util::string_hash_fnv1a;
use super::value::BinConfigValue;
use crate::{BinArray, BinTableGetError, DisplayIndent, Value};

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

    /// Tries to get an immutable reference to a [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.BinTable.html
    pub fn get<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<Value<&'t str, BinArray<'t>, BinTable<'t>>, BinTableGetError> {
        self.get_impl(key.into())
    }

    /// Tries to get a `bool` [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.BinTable.html
    pub fn get_bool<'k, K: Into<&'k str>>(&self, key: K) -> Result<bool, BinTableGetError> {
        let val = self.get(key)?;
        val.bool()
            .ok_or_else(|| BinTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `i64` [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.BinTable.html
    pub fn get_i64<'k, K: Into<&'k str>>(&self, key: K) -> Result<i64, BinTableGetError> {
        let val = self.get(key)?;
        val.i64()
            .ok_or_else(|| BinTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `f64` [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.BinTable.html
    pub fn get_f64<'k, K: Into<&'k str>>(&self, key: K) -> Result<f64, BinTableGetError> {
        let val = self.get(key)?;
        val.f64()
            .ok_or_else(|| BinTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a string [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.BinTable.html
    pub fn get_string<'k, K: Into<&'k str>>(&self, key: K) -> Result<&str, BinTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| BinTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`] [`value`] in the [`table`] with the string `key`.
    ///
    /// [`array`]: struct.BinArray.html
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.BinTable.html
    pub fn get_array<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<BinArray<'_>, BinTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| BinTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`] [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.BinTable.html
    pub fn get_table<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<BinTable<'_>, BinTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| BinTableGetError::IncorrectValueType(val_type))
    }

    /// Returns an [`iterator`] over (`key`, [`value`]) tuples of the [`table`], in unspecified order.
    ///
    /// [`iterator`]: struct.BinTableIter.html
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.BinTable.html
    pub fn iter(&self) -> BinTableIter<'_, 't> {
        BinTableIter::new(self)
    }

    pub(super) fn new(table: BinArrayOrTable<'t>) -> Self {
        Self(table)
    }

    fn get_impl(
        &self,
        key: &str,
    ) -> Result<Value<&'t str, BinArray<'t>, BinTable<'t>>, BinTableGetError> {
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

    fn get_value(&self, value: BinConfigValue) -> Value<&'t str, BinArray<'t>, BinTable<'t>> {
        use BinConfigValue::*;

        match value {
            Bool(val) => Value::Bool(val),
            I64(val) => Value::I64(val),
            F64(val) => Value::F64(val),
            String { offset, len } => Value::String(unsafe { self.0.string(offset, len) }), // Safe to call - the string was validated.
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

    fn fmt_indent_impl(&self, f: &mut Formatter, indent: u32, mut comma: bool) -> std::fmt::Result {
        if indent == 0 {
            comma = false
        };

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        if comma {
            writeln!(f, "{{")?;
        }

        // Sort the keys.
        keys.sort_by(|l, r| l.cmp(r));

        let len = self.len();

        // Iterate the table using the sorted keys.
        for (key_index, key) in keys.into_iter().enumerate() {
            let key = key;

            // Must succeed.
            // We either skipped invalid values or errored out above.
            let value = self.get(key).map_err(|_| std::fmt::Error)?;

            <Self as DisplayIndent>::do_indent(f, indent)?;

            write!(f, "{} = ", key)?;

            let is_table = match value {
                Value::Table(_) | Value::Array(_) => true,
                _ => false,
            };

            value.fmt_indent(f, indent, true)?;

            if comma {
                write!(f, ",")?;
            }

            if is_table {
                write!(f, " -- {}", key)?;
            }

            let last = (key_index as u32) == len - 1;

            if !last {
                writeln!(f)?;
            }
        }

        if comma {
            debug_assert!(indent > 0);
            <Self as DisplayIndent>::do_indent(f, indent - 1)?;
            write!(f, "\n}}")?;
        }

        Ok(())
    }
}

/// Iterator over (`key`, [`value`]) tuples of the [`table`], in unspecified order.
///
/// [`value`]: enum.Value.html
/// [`table`]: struct.BinTable.html
pub struct BinTableIter<'i, 't> {
    table: &'i BinTable<'t>,
    index: u32,
}

impl<'i, 't> BinTableIter<'i, 't> {
    fn new(table: &'i BinTable<'t>) -> Self {
        Self { table, index: 0 }
    }
}

impl<'i, 't> Iterator for BinTableIter<'i, 't> {
    type Item = (&'t str, Value<&'t str, BinArray<'t>, BinTable<'t>>);

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

impl<'t> DisplayIndent for BinTable<'t> {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        self.fmt_indent_impl(f, indent, comma)
    }
}

impl<'t> Display for BinTable<'t> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_indent_impl(f, 0, true)
    }
}
