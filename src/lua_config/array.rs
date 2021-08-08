use {
    super::util::{
        clear_array, get_array_value_type, get_table_len, new_array, set_array_value_type,
        set_table_len, value_from_lua_value,
    },
    crate::{
        util::{unwrap_unchecked, DisplayLua},
        ArrayError, ConfigKey, GetPathError, LuaConfigValue, LuaString, LuaTable, Value, ValueType,
    },
    rlua::Context,
    std::{
        borrow::Borrow,
        fmt::{Display, Formatter, Write},
    },
};

/// Represents a mutable Lua array of [`Value`]'s with integer 0-based (sic!) indices.
///
/// [`Value`]: struct.Value.html
#[derive(Clone)]
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

    /// Returns `true` if the [`array`] is empty.
    ///
    /// [`array`]: struct.LuaArray.html
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the [`array`].
    ///
    /// [`array`]: struct.DynArray.html
    pub fn clear(&mut self) {
        clear_array(&self.0);

        set_array_value_type(&self.0, None);
        set_table_len(&self.0, 0);
    }

    /// Tries to get a reference to a [`value`] in the [`array`] at `0`-based `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds.
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get(&self, index: u32) -> Result<LuaConfigValue<'lua>, ArrayError> {
        self.get_impl(index)
    }

    /// Tries to get a reference to a [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns the [`array`] itself if the `path` is empty.
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    /// [`type`]: enum.ValueType.html
    pub fn get_path<'a, K, P>(&self, path: P) -> Result<LuaConfigValue<'lua>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        LuaConfigValue::Array(self.clone())
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a [`bool`] [`value`] in the [`array`] at `0`-based `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_bool(&self, index: u32) -> Result<bool, ArrayError> {
        let val = self.get(index)?;
        val.bool()
            .ok_or(ArrayError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`bool`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to a [`bool`] [`value`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_bool_path<'a, K, P>(&self, path: P) -> Result<bool, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        val.bool()
            .ok_or(GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`array`] at `0`-based `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`i64`] / [`f64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_i64(&self, index: u32) -> Result<i64, ArrayError> {
        let val = self.get(index)?;
        val.i64()
            .ok_or(ArrayError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to an [`i64`] / [`f64`] [`value`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_i64_path<'a, K, P>(&self, path: P) -> Result<i64, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        val.i64()
            .ok_or(GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`array`] at `0`-based `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`f64`] / [`i64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_f64(&self, index: u32) -> Result<f64, ArrayError> {
        let val = self.get(index)?;
        val.f64()
            .ok_or(ArrayError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to an [`f64`] / [`i64`] [`value`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_f64_path<'a, K, P>(&self, path: P) -> Result<f64, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        val.f64()
            .ok_or(GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`string`] [`value`] in the [`array`] at `0`-based `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_string(&self, index: u32) -> Result<LuaString<'lua>, ArrayError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.string().ok_or(ArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`string`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to a [`string`] [`value`].
    ///
    /// [`string`]: enum.Value.html#variant.I64
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_string_path<'a, K, P>(&self, path: P) -> Result<LuaString<'lua>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.string()
            .ok_or(GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`](enum.Value.html#variant.Array) [`value`] in the [`array`] at `0`-based `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`array`](enum.Value.html#variant.Array).
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_array(&self, index: u32) -> Result<LuaArray<'lua>, ArrayError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.array().ok_or(ArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`](enum.Value.html#variant.Array) [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to an [`array`](enum.Value.html#variant.Array) [`value`].
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_array_path<'a, K, P>(&self, path: P) -> Result<LuaArray<'lua>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.array()
            .ok_or(GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`] [`value`] in the [`array`] at `0`-based `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`table`].
    ///
    /// [`table`]: enum.Value.html#variant.Table
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_table(&self, index: u32) -> Result<LuaTable<'lua>, ArrayError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.table().ok_or(ArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to a [`table`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to a [`table`] [`value`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: enum.Value.html#variant.Table
    /// [`array`]: struct.DynArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    pub fn get_table_path<'a, K, P>(&self, path: P) -> Result<LuaTable<'lua>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or(GetPathError::IncorrectValueType(val_type))
    }

    /// Returns an in-order iterator over [`values`] in the [`array`].
    ///
    /// [`values`]: enum.Value.html
    /// [`array`]: struct.LuaArray.html
    pub fn iter(&self) -> impl Iterator<Item = LuaConfigValue<'lua>> {
        LuaArrayIter(self.0.clone().sequence_values())
    }

    /// Changes the [`value`] in the [`array`] at `index` to `value`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if `value` is of incorrect type.
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn set<'s>(
        &mut self,
        index: u32,
        value: Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>,
    ) -> Result<(), ArrayError> {
        self.set_impl(index, value)
    }

    /// Pushes the [`value`] to the back of the [`array`].
    ///
    /// Returns an [`error`] if `value` is of incorrect type.
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html#variant.IncorrectValueType
    pub fn push<'s>(
        &mut self,
        value: Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>,
    ) -> Result<(), ArrayError> {
        self.push_impl(value)
    }

    /// Pops the [`value`] off the back of the [`array`].
    ///
    /// Returns an [`error`] if the [`array`] is empty.
    ///
    /// [`value`]: type.LuaConfigValue.html
    /// [`array`]: struct.LuaArray.html
    /// [`error`]: enum.ArrayError.html#variant.ArrayEmpty
    pub fn pop(&mut self) -> Result<LuaConfigValue<'lua>, ArrayError> {
        self.pop_impl()
    }

    pub(super) fn from_valid_table(table: rlua::Table<'lua>) -> Self {
        Self(table)
    }

    fn len_impl(&self) -> u32 {
        get_table_len(&self.0)
    }

    fn get_impl(&self, index: u32) -> Result<LuaConfigValue<'lua>, ArrayError> {
        use ArrayError::*;

        let len = self.len();

        if index >= len {
            return Err(IndexOutOfBounds(len));
        }

        // `+ 1` because of Lua array indexing.
        // Must succeed - the index is valid.
        let value: rlua::Value = unwrap_unchecked(self.0.get(index + 1));

        // Must succeed - the array only contains valid values.
        Ok(unwrap_unchecked(value_from_lua_value(value)))
    }

    fn validate_value_type<'s>(
        &self,
        len: u32,
        value: &Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>,
    ) -> Result<(), ArrayError> {
        use ArrayError::*;

        let value_type = value.get_type();
        let array_value_type = get_array_value_type(&self.0);

        // If the array is non-empty and has a value type, ensure the provided value type is compatible.
        if let Some(array_value_type) = array_value_type {
            debug_assert!(len > 0);

            if !array_value_type.is_compatible(value_type) {
                return Err(IncorrectValueType(array_value_type));
            }

        // Else the array must've been empty - update its value type.
        } else {
            debug_assert_eq!(len, 0);

            set_array_value_type(&self.0, Some(value_type));
        }

        Ok(())
    }

    /// The caller guarantees `index` and `value` are valid.
    fn set_array_value<'s>(
        array: &rlua::Table<'lua>,
        index: u32,
        value: Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>,
    ) {
        // Must succeed - index and value are valid.
        unwrap_unchecked(match value {
            Value::Bool(value) => array.raw_set(index, value),
            Value::F64(value) => array.raw_set(index, value),
            Value::I64(value) => array.raw_set(index, value),
            Value::String(value) => array.raw_set(index, value),
            Value::Array(value) => array.raw_set(index, value.0),
            Value::Table(value) => array.raw_set(index, value.0),
        });
    }

    fn set_impl<'s>(
        &mut self,
        index: u32,
        value: Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>,
    ) -> Result<(), ArrayError> {
        use ArrayError::*;

        // Validate the index.
        let len = self.len();

        if index >= len {
            return Err(IndexOutOfBounds(len));
        }

        // Validate the value type.
        self.validate_value_type(len, &value)?;

        // `+ 1` because of Lua array indexing.
        let index = index + 1;

        Self::set_array_value(&self.0, index, value);

        Ok(())
    }

    fn push_impl<'s>(
        &mut self,
        value: Value<&'s str, LuaArray<'lua>, LuaTable<'lua>>,
    ) -> Result<(), ArrayError> {
        let len = self.len();

        // Validate the value type.
        self.validate_value_type(len, &value)?;

        // `+ 1` because of Lua array indexing.
        let index = len + 1;

        Self::set_array_value(&self.0, index, value);

        // Increment the array length.
        set_table_len(&self.0, len + 1);

        Ok(())
    }

    fn pop_impl(&mut self) -> Result<LuaConfigValue<'lua>, ArrayError> {
        use ArrayError::*;

        let len = self.len();

        if len > 0 {
            let new_len = len - 1;

            // Decrement the array length.
            set_table_len(&self.0, new_len);

            // If the array is now empty, reset its value type.
            if new_len == 0 {
                set_array_value_type(&self.0, None);
            }

            // Last element has index `len` because of Lua array indexing.
            // Must succeed - the index is valid.
            let value: rlua::Value = unwrap_unchecked(self.0.get(len));
            // Must succeed - the index is valid.
            unwrap_unchecked(self.0.set(len, rlua::Value::Nil));
            // Must succeed - the value is valid.
            Ok(unwrap_unchecked(value_from_lua_value(value)))
        } else {
            Err(ArrayEmpty)
        }
    }

    fn fmt_lua_impl<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        writeln!(w, "{{")?;

        // Iterate the array.
        for (index, value) in self.iter().enumerate() {
            <Self as DisplayLua>::do_indent(w, indent + 1)?;

            value.fmt_lua(w, indent + 1)?;

            write!(w, ",")?;

            let is_array_or_table = matches!(value.get_type(), ValueType::Array | ValueType::Table);

            if is_array_or_table {
                write!(w, " -- [{}]", index)?;
            }

            writeln!(w)?;
        }

        <Self as DisplayLua>::do_indent(w, indent)?;
        write!(w, "}}")?;

        Ok(())
    }
}

/// In-order iterator over [`values`] in the [`array`].
///
/// [`values`]: type.LuaConfigValue.html
/// [`array`]: struct.LuaArray.html
struct LuaArrayIter<'lua>(rlua::TableSequence<'lua, rlua::Value<'lua>>);

impl<'lua> std::iter::Iterator for LuaArrayIter<'lua> {
    type Item = LuaConfigValue<'lua>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.0.next() {
            if let Ok(value) = value {
                // Must succeed - the array only contains valid values.
                Some(unwrap_unchecked(value_from_lua_value(value)))
            } else {
                debug_assert!(false, "unexpected error when iterating a Lua array");
                None // Stop on iteration error (this should never happen).
            }
        } else {
            None
        }
    }
}

impl<'lua> DisplayLua for LuaArray<'lua> {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(w, indent)
    }
}

impl<'lua> Display for LuaArray<'lua> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use crate::*;

    #[test]
    fn LuaArrayError_IndexOutOfBounds() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut array = LuaArray::new(lua);

            assert_eq!(array.get(0).err().unwrap(), ArrayError::IndexOutOfBounds(0));
            assert_eq!(
                array.get_bool(0).err().unwrap(),
                ArrayError::IndexOutOfBounds(0)
            );
            assert_eq!(
                array.get_i64(0).err().unwrap(),
                ArrayError::IndexOutOfBounds(0)
            );
            assert_eq!(
                array.get_f64(0).err().unwrap(),
                ArrayError::IndexOutOfBounds(0)
            );
            assert_eq!(
                array.get_string(0).err().unwrap(),
                ArrayError::IndexOutOfBounds(0)
            );
            assert_eq!(
                array.get_table(0).err().unwrap(),
                ArrayError::IndexOutOfBounds(0)
            );
            assert_eq!(
                array.get_array(0).err().unwrap(),
                ArrayError::IndexOutOfBounds(0)
            );

            assert_eq!(
                array.set(0, true.into()).err().unwrap(),
                ArrayError::IndexOutOfBounds(0)
            );

            // But this works.

            array.push(true.into()).unwrap();

            assert_eq!(array.get(0).unwrap().bool().unwrap(), true);
            assert_eq!(array.get_bool(0).unwrap(), true);
        });
    }

    #[test]
    fn LuaArrayError_ArrayEmpty() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut array = LuaArray::new(lua);

            assert_eq!(array.pop().err().unwrap(), ArrayError::ArrayEmpty);

            // But this works.

            array.push(true.into()).unwrap();

            assert_eq!(array.pop().unwrap().bool().unwrap(), true);
        });
    }

    #[test]
    fn LuaArrayError_IncorrectValueType() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            let mut array = LuaArray::new(lua);

            array.push(true.into()).unwrap();
            array.push(false.into()).unwrap();

            assert_eq!(
                array.push(7.into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                array.push(3.14.into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                array.push("foo".into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                array.push(LuaTable::new(lua).into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                array.push(LuaArray::new(lua).into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );

            assert_eq!(
                array.set(0, 7.into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                array.set(0, 3.14.into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                array.set(0, "foo".into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                array.set(0, LuaTable::new(lua).into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );
            assert_eq!(
                array.set(0, LuaArray::new(lua).into()).err().unwrap(),
                ArrayError::IncorrectValueType(ValueType::Bool)
            );

            // But this works.

            array.clear();

            array.push(7.into()).unwrap();
            array.push(3.14.into()).unwrap();
        });
    }

    #[test]
    fn basic() {
        let lua = rlua::Lua::new();

        lua.context(|lua| {
            // Create an empty array.
            let mut array = LuaArray::new(lua);
            assert_eq!(array.len(), 0);
            assert!(array.is_empty());

            // Make it a bool array.
            array.push(true.into()).unwrap();
            assert_eq!(array.len(), 1);
            assert!(!array.is_empty());

            assert_eq!(array.get(0).unwrap().bool().unwrap(), true);
            assert_eq!(array.get_bool(0).unwrap(), true);

            // Push a bool.
            array.push(false.into()).unwrap();
            assert_eq!(array.len(), 2);
            assert!(!array.is_empty());
            assert_eq!(array.get(1).unwrap().bool().unwrap(), false);
            assert_eq!(array.get_bool(1).unwrap(), false);

            // Clear it.
            assert_eq!(array.pop().unwrap().bool().unwrap(), false);
            assert_eq!(array.len(), 1);
            assert!(!array.is_empty());
            assert_eq!(array.pop().unwrap().bool().unwrap(), true);
            assert_eq!(array.len(), 0);
            assert!(array.is_empty());

            // Now push an int and make it an int / float array.
            array.push(7.into()).unwrap();
            assert_eq!(array.len(), 1);
            assert!(!array.is_empty());

            assert_eq!(array.get(0).unwrap().i64().unwrap(), 7);
            assert_eq!(array.get_i64(0).unwrap(), 7);

            assert!(cmp_f64(array.get(0).unwrap().f64().unwrap(), 7.0));
            assert!(cmp_f64(array.get_f64(0).unwrap(), 7.0));

            // Push a float.
            array.push(3.14.into()).unwrap();
            assert_eq!(array.len(), 2);
            assert!(!array.is_empty());

            assert_eq!(array.get(1).unwrap().i64().unwrap(), 3);
            assert_eq!(array.get_i64(1).unwrap(), 3);

            assert!(cmp_f64(array.get(1).unwrap().f64().unwrap(), 3.14));
            assert!(cmp_f64(array.get_f64(1).unwrap(), 3.14));

            // Push another int.
            array.push((-9).into()).unwrap();
            assert_eq!(array.len(), 3);
            assert!(!array.is_empty());

            assert_eq!(array.get(2).unwrap().i64().unwrap(), -9);
            assert_eq!(array.get_i64(2).unwrap(), -9);

            assert!(cmp_f64(array.get(2).unwrap().f64().unwrap(), -9.0));
            assert!(cmp_f64(array.get_f64(2).unwrap(), -9.0));

            // Iterate the array.
            for (index, value) in array.iter().enumerate() {
                match index {
                    0 => assert_eq!(value.i64().unwrap(), 7),
                    1 => assert!(cmp_f64(value.f64().unwrap(), 3.14)),
                    2 => assert_eq!(value.i64().unwrap(), -9),
                    _ => panic!("Invalid index."),
                }
            }

            // Array of arrays.
            array.clear();
            assert_eq!(array.len(), 0);
            assert!(array.is_empty());

            for _ in 0..3 {
                let mut nested_array = LuaArray::new(lua);
                assert_eq!(nested_array.len(), 0);
                assert!(nested_array.is_empty());

                for _ in 0..3 {
                    nested_array.push(true.into()).unwrap();
                }
                assert_eq!(nested_array.len(), 3);
                assert!(!nested_array.is_empty());

                array.push(nested_array.into()).unwrap();
            }
            assert_eq!(array.len(), 3);
            assert!(!array.is_empty());

            for i in 0..3 {
                for j in 0..3 {
                    assert_eq!(array.get_bool_path(&[i.into(), j.into()]).unwrap(), true);
                }
            }

            // Iterate the array.
            for value in array.iter() {
                let nested_array = value.array().unwrap();

                for value in nested_array.iter() {
                    assert_eq!(value.bool().unwrap(), true);
                }
            }
        });
    }
}
