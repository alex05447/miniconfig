use std::collections::{hash_map::Iter as HashMapIter, HashMap};
use std::fmt::{Display, Formatter, Write};
use std::ops::{Deref, DerefMut};

use crate::{
    write_lua_key, DisplayLua, DynArray, DynArrayMut, DynArrayRef, DynTableGetError,
    DynTableSetError, Value,
};

#[cfg(feature = "ini")]
use crate::{write_ini_section, write_ini_string, DisplayINI, ToINIStringError, ValueType};

/// Represents a mutable hashmap of [`Value`]'s with string keys.
///
/// [`Value`]: enum.Value.html
pub struct DynTable(HashMap<String, Value<String, DynArray, Self>>);

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

    /// Tries to get an immutable reference to a [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    pub fn get<'t, 'k, K: Into<&'k str>>(
        &'t self,
        key: K,
    ) -> Result<Value<&'t str, DynArrayRef<'t>, DynTableRef<'t>>, DynTableGetError> {
        self.get_impl(key.into())
    }

    /// Tries to get a `bool` [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    pub fn get_bool<'k, K: Into<&'k str>>(&self, key: K) -> Result<bool, DynTableGetError> {
        let val = self.get(key)?;
        val.bool()
            .ok_or_else(|| DynTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `i64` [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    pub fn get_i64<'k, K: Into<&'k str>>(&self, key: K) -> Result<i64, DynTableGetError> {
        let val = self.get(key)?;
        val.i64()
            .ok_or_else(|| DynTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `f64` [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    pub fn get_f64<'k, K: Into<&'k str>>(&self, key: K) -> Result<f64, DynTableGetError> {
        let val = self.get(key)?;
        val.f64()
            .ok_or_else(|| DynTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a string [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    pub fn get_string<'k, K: Into<&'k str>>(&self, key: K) -> Result<&str, DynTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| DynTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`] [`value`] in the [`table`] with the string `key`.
    ///
    /// [`array`]: struct.DynArray.html
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    pub fn get_array<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<DynArrayRef<'_>, DynTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| DynTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`] [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    pub fn get_table<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<DynTableRef<'_>, DynTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| DynTableGetError::IncorrectValueType(val_type))
    }

    /// Returns an [`iterator`] over (`key`, [`value`]) tuples of the [`table`], in unspecified order.
    ///
    /// [`iterator`]: struct.DynTableIter.html
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    pub fn iter(&self) -> DynTableIter<'_> {
        DynTableIter(self.0.iter())
    }

    /// Tries to get a mutable reference to a [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    //pub fn get_mut<'t, 'k, K: Into<&'k str>>(
    pub fn get_mut<'k, K: Into<&'k str>>(
        &mut self,
        key: K,
    ) -> Result<Value<&'_ str, DynArrayMut<'_>, DynTableMut<'_>>, DynTableGetError> {
        self.get_mut_impl(key.into())
    }

    /// If [`value`] is `Some`, inserts or changes the value at `key`.
    /// If [`value`] is `None`, tries to remove the value at `key`.
    /// Returns an [`error`] if the `key` does not exist in this case.
    ///
    /// [`value`]: enum.Value.html
    /// [`error`]: struct.DynTableSetError.html
    pub fn set<'s, V>(&mut self, key: &str, value: V) -> Result<(), DynTableSetError>
    where
        V: Into<Option<Value<&'s str, DynArray, DynTable>>>,
    {
        self.set_impl(key, value.into())
    }

    fn len_impl(&self) -> u32 {
        self.0.len() as u32
    }

    fn get_impl<'t>(
        &'t self,
        key: &str,
    ) -> Result<Value<&'t str, DynArrayRef<'t>, DynTableRef<'t>>, DynTableGetError> {
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

    fn set_impl<'s>(
        &mut self,
        key: &str,
        value: Option<Value<&'s str, DynArray, DynTable>>,
    ) -> Result<(), DynTableSetError> {
        use DynTableSetError::*;

        // Add or modify a value - always succeeds.
        if let Some(value) = value {
            let value = match value {
                Value::Bool(value) => Value::Bool(value),
                Value::I64(value) => Value::I64(value),
                Value::F64(value) => Value::F64(value),
                Value::String(value) => Value::String(value.into()),
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

    fn get_mut_impl<'t>(
        &'t mut self,
        key: &str,
    ) -> Result<Value<&'t str, DynArrayMut<'t>, DynTableMut<'t>>, DynTableGetError> {
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
    fn fmt_ini_impl<W: Write>(&self, w: &mut W, level: u32) -> Result<(), ToINIStringError> {
        use ToINIStringError::*;

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
                Value::Array(_) => return Err(ArraysNotSupported),
                Value::Table(value) => {
                    if level >= 1 {
                        return Err(NestedTablesNotSupported);
                    }

                    if key_index > 0 {
                        writeln!(w).map_err(|_| WriteError)?;
                    }

                    write_ini_section(w, key).map_err(|_| WriteError)?;

                    if value.len() > 0 {
                        writeln!(w).map_err(|_| WriteError)?;
                        value.fmt_ini(w, level + 1)?;
                    }

                    if !last {
                        writeln!(w).map_err(|_| WriteError)?;
                    }
                }
                value => {
                    write_ini_string(w, key, false).map_err(|_| WriteError)?;
                    write!(w, " = ").map_err(|_| WriteError)?;

                    value.fmt_ini(w, level + 1)?;

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
}

impl<'t> std::ops::Deref for DynTableRef<'t> {
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
    pub(super) fn new(inner: &'t mut DynTable) -> Self {
        Self(inner)
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
pub struct DynTableIter<'t>(HashMapIter<'t, String, Value<String, DynArray, DynTable>>);

impl<'t> Iterator for DynTableIter<'t> {
    type Item = (&'t str, Value<&'t str, DynArrayRef<'t>, DynTableRef<'t>>);

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
impl DisplayINI for DynTable {
    fn fmt_ini<W: Write>(&self, w: &mut W, level: u32) -> Result<(), ToINIStringError> {
        self.fmt_ini_impl(w, level)
    }
}

#[cfg(feature = "ini")]
impl<'t> DisplayINI for DynTableRef<'t> {
    fn fmt_ini<W: Write>(&self, w: &mut W, level: u32) -> Result<(), ToINIStringError> {
        self.fmt_ini_impl(w, level)
    }
}

#[cfg(feature = "ini")]
impl<'t> DisplayINI for DynTableMut<'t> {
    fn fmt_ini<W: Write>(&self, w: &mut W, level: u32) -> Result<(), ToINIStringError> {
        self.fmt_ini_impl(w, level)
    }
}
