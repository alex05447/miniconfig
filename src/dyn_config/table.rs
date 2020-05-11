use std::collections::{hash_map::Iter as HashMapIter, HashMap};
use std::fmt::{Display, Formatter};
use std::iter::Iterator;
use std::ops::{Deref, DerefMut};

#[cfg(feature = "ini")]
use std::fmt::Write;

use crate::{
    util::{write_lua_key, DisplayLua},
    DynArray, DynArrayMut, DynArrayRef, DynConfigValue, DynConfigValueMut, DynConfigValueRef,
    DynTableGetError, DynTableGetPathError, DynTableSetError, Value,
};

#[cfg(feature = "ini")]
use crate::{
    write_ini_key, write_ini_section, DisplayIni, ToIniStringError, ToIniStringOptions, ValueType,
};

/// Represents a mutable hashmap of [`Value`]'s with string keys.
///
/// [`Value`]: enum.Value.html
#[derive(Clone)]
pub struct DynTable(HashMap<String, DynConfigValue>);

impl DynTable {
    /// Creates a new empty [`table`].
    ///
    /// [`table`]: struct.DynTable.html
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Returns the number of entries in the [`table`].
    ///
    /// [`table`]: struct.DynTable.html
    pub fn len(&self) -> u32 {
        self.len_impl()
    }

    /// Returns `true` if the [`table`] is empty.
    ///
    /// [`table`]: struct.DynTable.html
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the [`table`].
    ///
    /// [`table`]: struct.DynTable.html
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Returns `true` if the [`table`] contains a [`value`] with the string `key`.
    ///
    /// [`table`]: struct.DynTable.html
    /// [`value`]: type.DynConfigValueRef.html
    pub fn contains<K: AsRef<str>>(&self, key: K) -> bool {
        match self.get(key) {
            Ok(_) => true,
            Err(err) => match err {
                DynTableGetError::KeyDoesNotExist => false,
                DynTableGetError::IncorrectValueType(_) => unreachable!(),
            },
        }
    }

    /// Tries to get an immutable reference to a [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key`.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: struct.DynTableGetError.html
    pub fn get<K: AsRef<str>>(&self, key: K) -> Result<DynConfigValueRef<'_>, DynTableGetError> {
        self.get_impl(key.as_ref())
    }

    /// Tries to get an immutable reference to a [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns the table itself if the `path` is empty.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// or if any of the non-terminating `path` elements is not a [`table`].
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        mut path: P,
    ) -> Result<DynConfigValueRef<'_>, DynTableGetPathError> {
        let mut _path = Vec::new();
        let table = DynTableRef::new(self);

        if let Some(key) = path.next() {
            let key = key.as_ref();

            _path.push(key.into());
            let mut value = Self::get_table_value(table, key, _path.clone())?;

            while let Some(key) = path.next() {
                let key = key.as_ref();

                match value {
                    Value::Table(table) => {
                        _path.push(key.into());
                        value = Self::get_table_value(table, key, _path.clone())?;
                    }
                    value => {
                        return Err(DynTableGetPathError::ValueNotATable {
                            path: _path,
                            value_type: value.get_type(),
                        })
                    }
                }
            }

            Ok(value)
        } else {
            Ok(Value::Table(table))
        }
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: struct.DynTableGetError.html
    pub fn get_bool<K: AsRef<str>>(&self, key: K) -> Result<bool, DynTableGetError> {
        let val = self.get(key)?;
        val.bool()
            .ok_or(DynTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// if any of the non-terminating `path` elements is not a [`table`],
    /// or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_bool_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        path: P,
    ) -> Result<bool, DynTableGetPathError> {
        let val = self.get_path(path)?;
        val.bool()
            .ok_or(DynTableGetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`i64`].
    ///
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: struct.DynTableGetError.html
    pub fn get_i64<K: AsRef<str>>(&self, key: K) -> Result<i64, DynTableGetError> {
        let val = self.get(key)?;
        val.i64()
            .ok_or(DynTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`i64`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// if any of the non-terminating `path` elements is not a [`table`],
    /// or if value is not an [`i64`].
    ///
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_i64_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        path: P,
    ) -> Result<i64, DynTableGetPathError> {
        let val = self.get_path(path)?;
        val.i64()
            .ok_or(DynTableGetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`f64`].
    ///
    /// [`f64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: struct.DynTableGetError.html
    pub fn get_f64<K: AsRef<str>>(&self, key: K) -> Result<f64, DynTableGetError> {
        let val = self.get(key)?;
        val.f64()
            .ok_or(DynTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// if any of the non-terminating `path` elements is not a [`table`],
    /// or if value is not an [`f64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_f64_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        path: P,
    ) -> Result<f64, DynTableGetPathError> {
        let val = self.get_path(path)?;
        val.f64()
            .ok_or(DynTableGetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`string`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: struct.DynTableGetError.html
    pub fn get_string<K: AsRef<str>>(&self, key: K) -> Result<&str, DynTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.string()
            .ok_or(DynTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`string`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// if any of the non-terminating `path` elements is not a [`table`],
    /// or if value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_string_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        path: P,
    ) -> Result<&str, DynTableGetPathError> {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.string()
            .ok_or(DynTableGetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: struct.DynTableGetError.html
    pub fn get_array<K: AsRef<str>>(&self, key: K) -> Result<DynArrayRef<'_>, DynTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.array()
            .ok_or(DynTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// if any of the non-terminating `path` elements is not a [`table`],
    /// or if value is not an [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_array_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        path: P,
    ) -> Result<DynArrayRef<'_>, DynTableGetPathError> {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.array()
            .ok_or(DynTableGetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: struct.DynTableGetError.html
    pub fn get_table<K: AsRef<str>>(&self, key: K) -> Result<DynTableRef<'_>, DynTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.table()
            .ok_or(DynTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// if any of the non-terminating `path` elements is not a [`table`],
    /// or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_table_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        path: P,
    ) -> Result<DynTableRef<'_>, DynTableGetPathError> {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or(DynTableGetPathError::IncorrectValueType(val_type))
    }

    /// Returns an iterator over (`key`, [`value`]) pairs of the [`table`], in unspecified order.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    pub fn iter(&self) -> impl Iterator<Item = (&'_ str, DynConfigValueRef<'_>)> {
        DynTableIter(self.0.iter())
    }

    /// Tries to get a mutable reference to a [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key`.
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`table`].
    ///
    /// [`value`]: type.DynConfigValueMut.html
    /// [`table`]: struct.DynTable.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    /// [`error`]: struct.DynTableGetError.html
    pub fn get_mut<K: AsRef<str>>(
        &mut self,
        key: K,
    ) -> Result<DynConfigValueMut<'_>, DynTableGetError> {
        self.get_mut_impl(key.as_ref())
    }

    /// Tries to get a mutable reference to a [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// or if any of the non-terminating `path` elements is not a [`table`].
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_mut_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &mut self,
        mut path: P,
    ) -> Result<DynConfigValueMut<'_>, DynTableGetPathError> {
        let mut _path = Vec::new();
        let table = DynTableMut::new(self);

        if let Some(key) = path.next() {
            let key = key.as_ref();

            _path.push(key.into());
            let mut value = Self::get_table_value_mut(table, key, _path.clone())?;

            while let Some(key) = path.next() {
                let key = key.as_ref();

                match value {
                    Value::Table(table) => {
                        _path.push(key.into());
                        value = Self::get_table_value_mut(table, key, _path.clone())?;
                    }
                    value => {
                        return Err(DynTableGetPathError::ValueNotATable {
                            path: _path,
                            value_type: value.get_type(),
                        })
                    }
                }
            }

            Ok(value)
        } else {
            Ok(Value::Table(table))
        }
    }

    /// Tries to get a mutable reference to an [`array`] [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: struct.DynTableGetError.html
    pub fn get_array_mut<K: AsRef<str>>(
        &mut self,
        key: K,
    ) -> Result<DynArrayMut<'_>, DynTableGetError> {
        let val = self.get_mut(key)?;
        let val_type = val.get_type();
        val.array()
            .ok_or(DynTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a mutable reference to an [`array`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// if any of the non-terminating `path` elements is not a [`table`],
    /// or if value is not an [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_array_mut_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &mut self,
        path: P,
    ) -> Result<DynArrayMut<'_>, DynTableGetPathError> {
        let val = self.get_mut_path(path)?;
        let val_type = val.get_type();
        val.array()
            .ok_or(DynTableGetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] with the string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: struct.DynTableGetError.html
    pub fn get_table_mut<K: AsRef<str>>(
        &mut self,
        key: K,
    ) -> Result<DynTableMut<'_>, DynTableGetError> {
        let val = self.get_mut(key)?;
        let val_type = val.get_type();
        val.table()
            .ok_or(DynTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a mutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested table keys.
    /// All keys except the last one must correspond to a [`table`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the full `path`,
    /// if any of the non-terminating `path` elements is not a [`table`],
    /// or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`error`]: struct.DynTableGetPathError.html
    pub fn get_table_mut_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &mut self,
        path: P,
    ) -> Result<DynTableMut<'_>, DynTableGetPathError> {
        let val = self.get_mut_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or(DynTableGetPathError::IncorrectValueType(val_type))
    }

    /// If [`value`] is `Some`, inserts or changes the value at `key`.
    /// If [`value`] is `None`, tries to remove the value at `key`.
    /// Returns an [`error`] if the `key` does not exist in this case.
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`error`]: struct.DynTableSetError.html
    pub fn set<'s, V>(&mut self, key: &str, value: V) -> Result<(), DynTableSetError>
    where
        V: Into<Option<DynConfigValue>>,
    {
        self.set_impl(key, value.into())
    }

    fn len_impl(&self) -> u32 {
        self.0.len() as u32
    }

    fn get_impl(&self, key: &str) -> Result<DynConfigValueRef<'_>, DynTableGetError> {
        if let Some(value) = self.0.get(key) {
            let value = match value {
                Value::Bool(value) => Value::Bool(*value),
                Value::I64(value) => Value::I64(*value),
                Value::F64(value) => Value::F64(*value),
                Value::String(value) => Value::String(value.as_str()),
                Value::Array(value) => Value::Array(DynArrayRef::new(value)),
                Value::Table(value) => Value::Table(DynTableRef::new(value)),
            };

            Ok(value)
        } else {
            Err(DynTableGetError::KeyDoesNotExist)
        }
    }

    fn set_impl(
        &mut self,
        key: &str,
        value: Option<DynConfigValue>,
    ) -> Result<(), DynTableSetError> {
        use DynTableSetError::*;

        if key.is_empty() {
            return Err(EmptyKey);
        }

        // Add or modify a value - always succeeds.
        if let Some(value) = value {
            let value = match value {
                Value::Bool(value) => Value::Bool(value),
                Value::I64(value) => Value::I64(value),
                Value::F64(value) => Value::F64(value),
                Value::String(value) => Value::String(value),
                Value::Array(value) => Value::Array(value),
                Value::Table(value) => Value::Table(value),
            };

            // Modify.
            if let Some(cur_value) = self.0.get_mut(key) {
                *cur_value = value;

            // Add.
            } else {
                self.0.insert(key.to_owned(), value);
            }

        // (Try to) remove a value.
        // Succeeds if key existed.
        } else if self.0.remove(key).is_none() {
            return Err(KeyDoesNotExist);
        }

        Ok(())
    }

    fn get_mut_impl(&mut self, key: &str) -> Result<DynConfigValueMut<'_>, DynTableGetError> {
        if let Some(value) = self.0.get_mut(key) {
            let value = match value {
                Value::Bool(value) => Value::Bool(*value),
                Value::I64(value) => Value::I64(*value),
                Value::F64(value) => Value::F64(*value),
                Value::String(value) => Value::String(value.as_str()),
                Value::Array(value) => Value::Array(DynArrayMut::new(value)),
                Value::Table(value) => Value::Table(DynTableMut::new(value)),
            };

            Ok(value)
        } else {
            Err(DynTableGetError::KeyDoesNotExist)
        }
    }

    fn get_table_value<'t>(
        table: DynTableRef<'t>,
        key: &str,
        path: Vec<String>,
    ) -> Result<DynConfigValueRef<'t>, DynTableGetPathError> {
        table.get(key).map_err(|err| match err {
            DynTableGetError::KeyDoesNotExist => DynTableGetPathError::PathDoesNotExist(path),
            DynTableGetError::IncorrectValueType(_) => unreachable!(),
        })
    }

    fn get_table_value_mut<'t>(
        table: DynTableMut<'t>,
        key: &str,
        path: Vec<String>,
    ) -> Result<DynConfigValueMut<'t>, DynTableGetPathError> {
        table.get_mut(key).map_err(|err| match err {
            DynTableGetError::KeyDoesNotExist => DynTableGetPathError::PathDoesNotExist(path),
            DynTableGetError::IncorrectValueType(_) => unreachable!(),
        })
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

/// Represents an immutable reference to a [`table`].
///
/// [`table`]: struct.DynTable.html
pub struct DynTableRef<'t>(&'t DynTable);

impl<'t> DynTableRef<'t> {
    pub(super) fn new(inner: &'t DynTable) -> Self {
        Self(inner)
    }

    // Needed to return a value with `'t` lifetime
    // instead of that with `self` lifetime if deferred to `Deref`.
    pub fn get<K: AsRef<str>>(&self, key: K) -> Result<DynConfigValueRef<'t>, DynTableGetError> {
        self.0.get(key)
    }

    // Needed to return a value with `'t` lifetime
    // instead of that with `self` lifetime if deferred to `Deref`.
    pub fn get_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        path: P,
    ) -> Result<DynConfigValueRef<'t>, DynTableGetPathError> {
        self.0.get_path(path)
    }

    // Needed to return a table with `'t` lifetime
    // instead of that with `self` lifetime if deferred to `Deref`.
    pub fn get_table<K: AsRef<str>>(&self, key: K) -> Result<DynTableRef<'t>, DynTableGetError> {
        self.0.get_table(key)
    }

    // Needed to return a table with `'t` lifetime
    // instead of that with `self` lifetime if deferred to `Deref`.
    pub fn get_table_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        path: P,
    ) -> Result<DynTableRef<'t>, DynTableGetPathError> {
        self.0.get_table_path(path)
    }

    // Needed to return an array with `'t` lifetime
    // instead of that with `self` lifetime if deferred to `Deref`.
    pub fn get_array<K: AsRef<str>>(&self, key: K) -> Result<DynArrayRef<'t>, DynTableGetError> {
        self.0.get_array(key)
    }

    // Needed to return an array with `'t` lifetime
    // instead of that with `self` lifetime if deferred to `Deref`.
    pub fn get_array_path<K: AsRef<str>, P: Iterator<Item = K>>(
        &self,
        path: P,
    ) -> Result<DynArrayRef<'t>, DynTableGetPathError> {
        self.0.get_array_path(path)
    }
}

impl<'t> Deref for DynTableRef<'t> {
    type Target = DynTable;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Represents a mutable reference to a [`table`].
///
/// [`table`]: struct.DynTable.html
pub struct DynTableMut<'t>(&'t mut DynTable);

impl<'t> DynTableMut<'t> {
    pub(crate) fn new(inner: &'t mut DynTable) -> Self {
        Self(inner)
    }

    pub fn get_mut<K: AsRef<str>>(self, key: K) -> Result<DynConfigValueMut<'t>, DynTableGetError> {
        self.0.get_mut(key)
    }

    pub fn get_mut_path<K: AsRef<str>, P: Iterator<Item = K>>(
        self,
        path: P,
    ) -> Result<DynConfigValueMut<'t>, DynTableGetPathError> {
        self.0.get_mut_path(path)
    }

    pub fn get_array_mut<K: AsRef<str>>(self, key: K) -> Result<DynArrayMut<'t>, DynTableGetError> {
        self.0.get_array_mut(key)
    }

    pub fn get_array_mut_path<K: AsRef<str>, P: Iterator<Item = K>>(
        self,
        path: P,
    ) -> Result<DynArrayMut<'t>, DynTableGetPathError> {
        self.0.get_array_mut_path(path)
    }

    pub fn get_table_mut<K: AsRef<str>>(self, key: K) -> Result<DynTableMut<'t>, DynTableGetError> {
        self.0.get_table_mut(key)
    }

    pub fn get_table_mut_path<K: AsRef<str>, P: Iterator<Item = K>>(
        self,
        path: P,
    ) -> Result<DynTableMut<'t>, DynTableGetPathError> {
        self.0.get_table_mut_path(path)
    }
}

impl<'t> Deref for DynTableMut<'t> {
    type Target = DynTable;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'t> DerefMut for DynTableMut<'t> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

/// Iterator over (`key`, [`value`]) tuples of the [`table`], in unspecified order.
///
/// [`value`]: enum.Value.html
/// [`table`]: struct.DynTable.html
struct DynTableIter<'t>(HashMapIter<'t, String, Value<String, DynArray, DynTable>>);

impl<'t> Iterator for DynTableIter<'t> {
    type Item = (&'t str, DynConfigValueRef<'t>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((key, value)) = self.0.next() {
            let value = match value {
                Value::Bool(value) => Value::Bool(*value),
                Value::I64(value) => Value::I64(*value),
                Value::F64(value) => Value::F64(*value),
                Value::String(value) => Value::String(value.as_str()),
                Value::Array(value) => Value::Array(DynArrayRef::new(value)),
                Value::Table(value) => Value::Table(DynTableRef::new(value)),
            };

            Some((key.as_str(), value))
        } else {
            None
        }
    }
}

impl DisplayLua for DynTable {
    fn fmt_lua(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(f, indent)
    }
}

impl<'t> DisplayLua for DynTableRef<'t> {
    fn fmt_lua(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(f, indent)
    }
}

impl<'t> DisplayLua for DynTableMut<'t> {
    fn fmt_lua(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(f, indent)
    }
}

impl Display for DynTable {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
    }
}

#[cfg(feature = "ini")]
impl DisplayIni for DynTable {
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

#[cfg(feature = "ini")]
impl<'t> DisplayIni for DynTableRef<'t> {
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

#[cfg(feature = "ini")]
impl<'t> DisplayIni for DynTableMut<'t> {
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
