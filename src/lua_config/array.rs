use std::fmt::{Display, Formatter};

use super::util::{
    array_value_type, new_array, set_array_value_type, set_table_len, table_len,
    value_from_lua_value,
};
use crate::{DisplayIndent, LuaArrayGetError, LuaArraySetError, LuaString, LuaTable, Value};

use rlua::Context;

/// Represents a mutable Lua array of [`Value`]'s with integer 0-based (sic!) indices.
///
/// [`Value`]: struct.Value.html
pub struct LuaArray<'lua>(pub(super) rlua::Table<'lua>);

impl<'lua> LuaArray<'lua> {
    /// Creates a new empty [`array`].
    ///
    /// [`array`]: struct.LuaArray.html
    pub fn new(lua: Context<'lua>) -> Self {
        Self(new_array(lua))
    }

    /// Returns the length of the [`array`].
    ///
    /// [`array`]: struct.LuaArray.html
    pub fn len(&self) -> u32 {
        self.len_impl()
    }

    /// Tries to get a reference to a [`value`] in the [`array`] at `index`.
    /// Returns an [`error`] if `index` is out of bounds.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: struct.LuaArrayGetError.html
    pub fn get(
        &self,
        index: u32,
    ) -> Result<Value<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>>, LuaArrayGetError> {
        self.get_impl(index)
    }

    /// Returns an in-order [`iterator`] over [`values`] in the [`array`].
    ///
    /// [`iterator`]: struct.LuaArrayIter.html
    /// [`values`]: enum.Value.html
    /// [`array`]: struct.LuaArray.html
    pub fn iter(&self) -> LuaArrayIter<'lua> {
        LuaArrayIter(self.0.clone().sequence_values())
    }

    /// Changes the [`value`] in the [`array`] at `index` to `value`.
    /// Returns an [`error`] if `index` is out of bounds or if `value` is of invalid type.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: struct.LuaArrayGetError.html
    pub fn set<'s>(
        &mut self,
        index: u32,
        value: Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>,
    ) -> Result<(), LuaArraySetError> {
        self.set_impl(index, value)
    }

    /// Pushes the [`value`] to the back of the [`array`].
    /// Returns an [`error`] if `value` is of invalid type.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.LuaArray.html
    pub fn push<'s>(
        &mut self,
        value: Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>,
    ) -> Result<(), LuaArraySetError> {
        self.push_impl(value)
    }

    /// Pops the [`value`] off the back of the [`array`].
    /// Returns an [`error`] if the [`array`] is empty.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: struct.LuaArraySetError.html
    pub fn pop(
        &mut self,
    ) -> Result<Value<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>>, LuaArrayGetError> {
        self.pop_impl()
    }

    pub(super) fn from_valid_table(table: rlua::Table<'lua>) -> Self {
        Self(table)
    }

    fn len_impl(&self) -> u32 {
        table_len(&self.0)
    }

    fn get_impl(
        &self,
        index: u32,
    ) -> Result<Value<LuaString<'lua>, Self, LuaTable<'lua>>, LuaArrayGetError> {
        use LuaArrayGetError::*;

        let len = table_len(&self.0);

        if index >= len {
            return Err(IndexOutOfBounds(len));
        }

        // `+ 1` because of Lua array indexing.
        // Must succeed.
        let value: rlua::Value = self.0.get(index + 1).unwrap();

        Ok(value_from_lua_value(value).unwrap())
    }

    fn validate_value_type<'s>(
        &self,
        len: u32,
        value: &Value<&'s str, Self, LuaTable<'lua>>,
    ) -> Result<(), LuaArraySetError> {
        use LuaArraySetError::*;

        let value_type = value.get_type();
        let array_value_type = array_value_type(&self.0);

        // If array is non-empty and has a value type, ensure the provided value type is compatible.
        if let Some(array_value_type) = array_value_type {
            debug_assert!(len > 0);

            if !array_value_type.is_compatible(value_type) {
                return Err(InvalidValueType(array_value_type));
            }

        // Else the array must've been empty - update its value type.
        } else {
            debug_assert_eq!(len, 0);

            set_array_value_type(&self.0, Some(value_type));
        }

        Ok(())
    }

    fn set_impl<'s>(
        &mut self,
        index: u32,
        value: Value<&'s str, Self, LuaTable<'lua>>,
    ) -> Result<(), LuaArraySetError> {
        use LuaArraySetError::*;

        // Validate the index.
        let len = table_len(&self.0);

        if index >= len {
            return Err(IndexOutOfBounds(len));
        }

        // Validate the value type.
        self.validate_value_type(len, &value)?;

        // `+ 1` because of Lua array indexing.
        let index = index + 1;

        match value {
            Value::Bool(value) => self.0.set(index, value).unwrap(),
            Value::F64(value) => self.0.set(index, value).unwrap(),
            Value::I64(value) => self.0.set(index, value).unwrap(),
            Value::String(value) => self.0.set(index, value).unwrap(),
            Value::Array(value) => self.0.set(index, value.0).unwrap(),
            Value::Table(value) => self.0.set(index, value.0).unwrap(),
        }

        Ok(())
    }

    fn push_impl<'s>(
        &mut self,
        value: Value<&'s str, Self, LuaTable<'lua>>,
    ) -> Result<(), LuaArraySetError> {
        let len = table_len(&self.0);

        // Validate the value type.
        self.validate_value_type(len, &value)?;

        // `+ 1` because of Lua array indexing.
        let index = len + 1;

        match value {
            Value::Bool(value) => self.0.set(index, value).unwrap(),
            Value::F64(value) => self.0.set(index, value).unwrap(),
            Value::I64(value) => self.0.set(index, value).unwrap(),
            Value::String(value) => self.0.set(index, value).unwrap(),
            Value::Array(value) => self.0.set(index, value.0).unwrap(),
            Value::Table(value) => self.0.set(index, value.0).unwrap(),
        }

        set_table_len(&self.0, len + 1);

        Ok(())
    }

    fn pop_impl(
        &mut self,
    ) -> Result<Value<LuaString<'lua>, Self, LuaTable<'lua>>, LuaArrayGetError> {
        use LuaArrayGetError::*;

        let len = table_len(&self.0);

        if len > 0 {
            let new_len = len - 1;

            // Decrement the array length.
            set_table_len(&self.0, new_len);

            // If the array is now empty, reset its value type.
            if new_len == 0 {
                set_array_value_type(&self.0, None);
            }

            // Last element has index `len` because of Lua array indexing.
            // Must succeed.
            let value: rlua::Value = self.0.get(len).unwrap();
            self.0.set(len, rlua::Value::Nil).unwrap();

            Ok(value_from_lua_value(value).unwrap())
        } else {
            Err(ArrayEmpty)
        }
    }

    fn fmt_indent_impl(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        if comma {
            write!(f, "{{ ")?;
        }

        let len = self.len();

        // Iterate the array.
        for (index, value) in self.iter().enumerate() {
            value.fmt_indent(f, indent + 1, true)?;

            let last = (index as u32) == len - 1;

            if comma && !last {
                write!(f, ", ")?;
            }
        }

        if comma {
            write!(f, " }}")?;
        }

        Ok(())
    }
}

/// In-order iterator over [`values`] in the [`array`].
///
/// [`values`]: enum.Value.html
/// [`array`]: struct.LuaArray.html
pub struct LuaArrayIter<'lua>(rlua::TableSequence<'lua, rlua::Value<'lua>>);

impl<'lua> std::iter::Iterator for LuaArrayIter<'lua> {
    type Item = Value<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.0.next() {
            if let Ok(value) = value {
                // Must succeed.
                Some(value_from_lua_value(value).unwrap())
            } else {
                None // Stop on iteration error (this should never happen).
            }
        } else {
            None
        }
    }
}

impl<'lua> DisplayIndent for LuaArray<'lua> {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        self.fmt_indent_impl(f, indent, comma)
    }
}

impl<'lua> Display for LuaArray<'lua> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_indent_impl(f, 0, true)
    }
}
