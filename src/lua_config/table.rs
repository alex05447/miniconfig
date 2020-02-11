use std::fmt::{Display, Formatter, Write};

use super::util::{
    new_table, set_table_len, table_len, value_from_lua_value, ValueFromLuaValueError,
};
use crate::{
    write_lua_key, DisplayLua, LuaArray, LuaString, LuaTableGetError, LuaTableSetError, Value,
    ValueType,
};

#[cfg(feature = "ini")]
use crate::{write_ini_section, write_ini_string, DisplayINI, ToINIStringError};

use rlua::Context;

/// Represents a mutable Lua hash table of [`Value`]'s with string keys.
///
/// [`Value`]: enum.Value.html
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

    /// Tries to get a reference to a [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    pub fn get<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<Value<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>>, LuaTableGetError> {
        self.get_impl(key.into())
    }

    /// Tries to get a `bool` [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    pub fn get_bool<'k, K: Into<&'k str>>(&self, key: K) -> Result<bool, LuaTableGetError> {
        let val = self.get(key)?;
        val.bool()
            .ok_or_else(|| LuaTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `i64` [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    pub fn get_i64<'k, K: Into<&'k str>>(&self, key: K) -> Result<i64, LuaTableGetError> {
        let val = self.get(key)?;
        val.i64()
            .ok_or_else(|| LuaTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `f64` [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    pub fn get_f64<'k, K: Into<&'k str>>(&self, key: K) -> Result<f64, LuaTableGetError> {
        let val = self.get(key)?;
        val.f64()
            .ok_or_else(|| LuaTableGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a string [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    pub fn get_string<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<LuaString<'lua>, LuaTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| LuaTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`] [`value`] in the [`table`] with the string `key`.
    ///
    /// [`array`]: struct.LuaArray.html
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    pub fn get_array<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<LuaArray<'lua>, LuaTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| LuaTableGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`] [`value`] in the [`table`] with the string `key`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    pub fn get_table<'k, K: Into<&'k str>>(
        &self,
        key: K,
    ) -> Result<LuaTable<'lua>, LuaTableGetError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| LuaTableGetError::IncorrectValueType(val_type))
    }

    /// Returns an [`iterator`] over ([`key`], [`value`]) tuples of the [`table`], in unspecified order.
    ///
    /// [`iterator`]: struct.LuaTableIter.html
    /// [`key`]: struct.LuaString.html
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    pub fn iter(&self) -> LuaTableIter<'lua> {
        LuaTableIter(self.0.clone().pairs())
    }

    /// If [`value`] is `Some`, inserts or changes the value at `key`.
    /// If [`value`] is `None`, tries to remove the value at `key`.
    /// Returns an [`error`] if the `key` does not exist in this case.
    ///
    /// [`value`]: enum.Value.html
    /// [`error`]: struct.LuaTableSetError.html
    pub fn set<'s, V>(&mut self, key: &str, value: V) -> Result<(), LuaTableSetError>
    where
        V: Into<Option<Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>>>,
    {
        self.set_impl(key, value.into())
    }

    pub(super) fn clone(&self) -> LuaTable<'lua> {
        Self(self.0.clone())
    }

    pub(super) fn from_valid_table(table: rlua::Table<'lua>) -> Self {
        Self(table)
    }

    fn len_impl(&self) -> u32 {
        table_len(&self.0)
    }

    fn get_impl(
        &self,
        key: &str,
    ) -> Result<Value<LuaString<'lua>, LuaArray<'lua>, Self>, LuaTableGetError> {
        use LuaTableGetError::*;

        let value: rlua::Value = self.0.get(key).map_err(|_| KeyDoesNotExist)?;

        value_from_lua_value(value).map_err(|err| match err {
            ValueFromLuaValueError::KeyDoesNotExist => KeyDoesNotExist,
            _ => unreachable!(),
        })
    }

    fn set_impl<'s>(
        &mut self,
        key: &str,
        value: Option<Value<&'s str, LuaArray<'lua>, Self>>,
    ) -> Result<(), LuaTableSetError> {
        use LuaTableSetError::*;

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
                set_table_len(&self.0, table_len(&self.0) + 1);
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
            match value {
                rlua::Value::Nil => false,
                _ => true,
            }
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
            let key = key.as_ref();

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

            let key = key.as_ref();

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

/// Iterator over (`key`, [`value`]) tuples of the [`table`], in unspecified order.
///
/// [`value`]: enum.Value.html
/// [`table`]: struct.LuaTable.html
pub struct LuaTableIter<'lua>(rlua::TablePairs<'lua, rlua::Value<'lua>, rlua::Value<'lua>>);

impl<'lua> std::iter::Iterator for LuaTableIter<'lua> {
    type Item = (
        LuaString<'lua>,
        Value<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>>,
    );

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

impl<'lua> Display for LuaTable<'lua> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
    }
}

#[cfg(feature = "ini")]
impl<'lua> DisplayINI for LuaTable<'lua> {
    fn fmt_ini<W: Write>(&self, w: &mut W, level: u32) -> Result<(), ToINIStringError> {
        self.fmt_ini_impl(w, level)
    }
}
