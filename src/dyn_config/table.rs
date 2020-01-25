use std::collections::{hash_map::Iter as HashMapIter, HashMap};
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};

use crate::{
    DisplayIndent, DynArray, DynArrayMut, DynArrayRef, DynTableGetError, DynTableSetError, Value,
};

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
    pub fn get_mut<'t, 'k, K: Into<&'k str>>(
        &'t mut self,
        key: K,
    ) -> Result<Value<&'t str, DynArrayMut<'t>, DynTableMut<'t>>, DynTableGetError> {
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

impl DisplayIndent for DynTable {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        self.fmt_indent_impl(f, indent, comma)
    }
}

impl<'t> DisplayIndent for DynTableRef<'t> {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        self.fmt_indent_impl(f, indent, comma)
    }
}

impl<'t> DisplayIndent for DynTableMut<'t> {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        self.fmt_indent_impl(f, indent, comma)
    }
}

impl Display for DynTable {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_indent_impl(f, 0, true)
    }
}
