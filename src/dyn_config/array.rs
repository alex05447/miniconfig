use {
    crate::{util::DisplayLua, *},
    std::{
        borrow::Borrow,
        fmt::{Display, Formatter, Write},
        slice::Iter as VecIter,
    },
};

/// Represents a mutable array of [`Value`]'s with integer 0-based indices.
///
/// [`Value`]: struct.Value.html
#[derive(Clone)]
pub struct DynArray(Vec<DynConfigValue>);

impl DynArray {
    /// Creates a new empty [`array`].
    ///
    /// [`array`]: struct.DynArray.html
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Returns the length of the [`array`].
    ///
    /// [`array`]: struct.DynArray.html
    pub fn len(&self) -> u32 {
        self.len_impl()
    }

    /// Returns `true` if the [`array`] is empty.
    ///
    /// [`array`]: struct.DynArray.html
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the [`array`].
    ///
    /// [`array`]: struct.DynArray.html
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Tries to get an immutable reference to a [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get(&self, index: u32) -> Result<DynConfigValueRef<'_>, ArrayError> {
        self.get_impl(index)
    }

    /// Tries to get an immutable reference to a [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns the [`array`] itself if the `path` is empty.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`array`]: struct.DynArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    /// [`type`]: enum.ValueType.html
    pub fn get_path<'a, K, P>(&self, path: P) -> Result<DynConfigValueRef<'_>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        DynConfigValueRef::Array(&self)
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a [`bool`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.DynConfigValueRef.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_bool(&self, index: u32) -> Result<bool, ArrayError> {
        let val = self.get(index)?;
        val.bool()
            .ok_or_else(|| ArrayError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`bool`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to a [`bool`] [`value`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
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
            .ok_or_else(|| GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`i64`] / [`f64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValueRef.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_i64(&self, index: u32) -> Result<i64, ArrayError> {
        let val = self.get(index)?;
        val.i64()
            .ok_or_else(|| ArrayError::IncorrectValueType(val.get_type()))
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
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
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
            .ok_or_else(|| GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`f64`] / [`i64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValueRef.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_f64(&self, index: u32) -> Result<f64, ArrayError> {
        let val = self.get(index)?;
        val.f64()
            .ok_or_else(|| ArrayError::IncorrectValueType(val.get_type()))
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
            .ok_or_else(|| GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`string`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_string(&self, index: u32) -> Result<&str, ArrayError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| ArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`string`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to a [`string`] [`value`].
    ///
    /// [`string`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_string_path<'a, K, P>(&self, path: P) -> Result<&str, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`](enum.Value.html#variant.Array) [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`array`](enum.Value.html#variant.Array).
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_array(&self, index: u32) -> Result<&DynArray, ArrayError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| ArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`](enum.Value.html#variant.Array) [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to an [`array`](enum.Value.html#variant.Array) [`value`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_array_path<'a, K, P>(&self, path: P) -> Result<&DynArray, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to a [`table`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`table`].
    ///
    /// [`table`]: enum.Value.html#variant.Table
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn get_table(&self, index: u32) -> Result<&DynTable, ArrayError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| ArrayError::IncorrectValueType(val_type))
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
    pub fn get_table_path<'a, K, P>(&self, path: P) -> Result<&DynTable, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Returns an in-order iterator over [`values`] in the [`array`].
    ///
    /// [`values`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    pub fn iter(&self) -> impl Iterator<Item = DynConfigValueRef<'_>> {
        DynArrayIter(self.0.iter())
    }

    /// Tries to get a mutable reference to a [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds.
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`array`].
    ///
    /// [`value`]: type.DynConfigValueMut.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_mut(&mut self, index: u32) -> Result<DynConfigValueMut<'_>, ArrayError> {
        self.get_mut_impl(index)
    }

    /// Tries to get a mutable reference to a [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns the [`array`] itself if the `path` is empty.
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`array`].
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`array`]: struct.DynArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: struct.DynTable.html
    /// [`type`]: enum.ValueType.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_mut_path<'a, K, P>(
        &mut self,
        path: P,
    ) -> Result<DynConfigValueMut<'_>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        DynConfigValueMut::Array(self)
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a mutable reference to an [`array`](enum.Value.html#variant.Array) [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`array`](enum.Value.html#variant.Array).
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`array`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_array_mut(&mut self, index: u32) -> Result<&mut DynArray, ArrayError> {
        let val = self.get_mut(index)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| ArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`](enum.Value.html#variant.Array) [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to an [`array`](enum.Value.html#variant.Array) [`value`].
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`array`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: enum.Value.html#variant.Table
    /// [`array`]: struct.DynArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_array_mut_path<'a, K, P>(
        &mut self,
        path: P,
    ) -> Result<&mut DynArray, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_mut_path(path)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get a mutable reference to a [`table`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`table`].
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`array`].
    ///
    /// [`table`]: enum.Value.html#variant.Table
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_table_mut(&mut self, index: u32) -> Result<&mut DynTable, ArrayError> {
        let val = self.get_mut(index)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| ArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get a mutable reference to a [`table`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to a [`table`] [`value`].
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`array`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: enum.Value.html#variant.Table
    /// [`array`]: struct.DynArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_table_mut_path<'a, K, P>(
        &mut self,
        path: P,
    ) -> Result<&mut DynTable, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_mut_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Changes the [`value`] in the [`array`] at `index` to `value`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if `value` is of invalid type.
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn set(&mut self, index: u32, value: DynConfigValue) -> Result<(), ArrayError> {
        self.set_impl(index, value)
    }

    /// Pushes the [`value`] to the back of the [`array`].
    ///
    /// Returns an [`error`] if `value` is of invalid type.
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn push(&mut self, value: DynConfigValue) -> Result<(), ArrayError> {
        self.push_impl(value)
    }

    /// Pops the [`value`] off the back of the [`array`].
    ///
    /// Returns an [`error`] if the [`array`] is empty.
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: enum.ArrayError.html
    pub fn pop(&mut self) -> Result<DynConfigValue, ArrayError> {
        self.pop_impl()
    }

    fn len_impl(&self) -> u32 {
        self.0.len() as u32
    }

    fn get_impl(&self, index: u32) -> Result<DynConfigValueRef<'_>, ArrayError> {
        use ArrayError::*;

        let len = self.len();

        // Index out of bounds.
        if index >= len {
            Err(IndexOutOfBounds(len))
        } else {
            let value = match unsafe { self.0.get_unchecked(index as usize) } {
                Value::Bool(value) => Value::Bool(*value),
                Value::I64(value) => Value::I64(*value),
                Value::F64(value) => Value::F64(*value),
                Value::String(value) => Value::String(value.as_str()),
                Value::Array(value) => Value::Array(value),
                Value::Table(value) => Value::Table(value),
            };

            Ok(value)
        }
    }

    fn get_mut_impl(&mut self, index: u32) -> Result<DynConfigValueMut<'_>, ArrayError> {
        use ArrayError::*;

        let len = self.len();

        // Index out of bounds.
        if index >= len {
            Err(IndexOutOfBounds(len))
        } else {
            let value = match unsafe { self.0.get_unchecked_mut(index as usize) } {
                Value::Bool(value) => Value::Bool(*value),
                Value::I64(value) => Value::I64(*value),
                Value::F64(value) => Value::F64(*value),
                Value::String(value) => Value::String(value.as_str()),
                Value::Array(value) => Value::Array(value),
                Value::Table(value) => Value::Table(value),
            };

            Ok(value)
        }
    }

    fn validate_value_type<S: Into<String>>(
        &self,
        value: &Value<S, DynArray, DynTable>,
    ) -> Result<(), ArrayError> {
        use ArrayError::*;

        // If array is non-empty and has a value type, ensure the provided value type is compatible.
        if self.len() > 0 {
            let array_value_type = unsafe { self.0.get_unchecked(0) }.get_type();

            if !array_value_type.is_compatible(value.get_type()) {
                return Err(IncorrectValueType(array_value_type));
            }
        }
        // Else the array has no type.

        Ok(())
    }

    fn set_impl(&mut self, index: u32, value: DynConfigValue) -> Result<(), ArrayError> {
        use ArrayError::*;

        // Validate the index.
        let len = self.len();

        if index >= len {
            return Err(IndexOutOfBounds(len));
        }

        // Validate the value type.
        // If array is non-empty and has a value type, ensure the provided value type is compatible.
        // NOTE - a single element array will have its only value replaced, so its type doesn't matter.
        if self.len() > 1 {
            let array_value_type = unsafe { self.0.get_unchecked(0) }.get_type();

            if !array_value_type.is_compatible(value.get_type()) {
                return Err(IncorrectValueType(array_value_type));
            }
        }
        // Else the array has no type.

        let dst = unsafe { self.0.get_unchecked_mut(index as usize) };

        match value {
            Value::Bool(value) => *dst = Value::Bool(value),
            Value::I64(value) => *dst = Value::I64(value),
            Value::F64(value) => *dst = Value::F64(value),
            Value::String(value) => *dst = Value::String(value),
            Value::Array(value) => *dst = Value::Array(value),
            Value::Table(value) => *dst = Value::Table(value),
        }

        Ok(())
    }

    fn push_impl(&mut self, value: DynConfigValue) -> Result<(), ArrayError> {
        // Validate the value type.
        self.validate_value_type(&value)?;

        self.0.push(value);

        Ok(())
    }

    fn pop_impl(&mut self) -> Result<DynConfigValue, ArrayError> {
        self.0.pop().ok_or_else(|| ArrayError::ArrayEmpty)
    }

    fn fmt_lua_impl<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        writeln!(w, "{{ ")?;

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
/// [`values`]: type.DynConfigValue.html
/// [`array`]: struct.DynArray.html
struct DynArrayIter<'a>(VecIter<'a, DynConfigValue>);

impl<'a> Iterator for DynArrayIter<'a> {
    type Item = DynConfigValueRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.0.next() {
            let value = match value {
                Value::Bool(value) => Value::Bool(*value),
                Value::I64(value) => Value::I64(*value),
                Value::F64(value) => Value::F64(*value),
                Value::String(value) => Value::String(value.as_str()),
                Value::Array(value) => Value::Array(value),
                Value::Table(value) => Value::Table(value),
            };

            Some(value)
        } else {
            None
        }
    }
}

impl DisplayLua for DynArray {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(w, indent)
    }
}

impl<'a> DisplayLua for &'a DynArray {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(w, indent)
    }
}

impl<'a> DisplayLua for &'a mut DynArray {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(w, indent)
    }
}

impl Display for DynArray {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use crate::*;

    #[test]
    fn len_empty_clear() {
        let mut array = DynArray::new();

        assert_eq!(array.len(), 0);
        assert!(array.is_empty());

        array.push(true.into()).unwrap();

        assert_eq!(array.len(), 1);
        assert!(!array.is_empty());

        array.push(false.into()).unwrap();

        assert_eq!(array.len(), 2);
        assert!(!array.is_empty());

        array.clear();

        assert_eq!(array.len(), 0);
        assert!(array.is_empty());
    }

    #[test]
    fn DynArrayError_IndexOutOfBounds() {
        let mut array = DynArray::new();

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
            array.get_mut(0).err().unwrap(),
            ArrayError::IndexOutOfBounds(0)
        );
        assert_eq!(
            array.get_table_mut(0).err().unwrap(),
            ArrayError::IndexOutOfBounds(0)
        );
        assert_eq!(
            array.get_array_mut(0).err().unwrap(),
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
    }

    #[test]
    fn DynArrayError_ArrayEmpty() {
        let mut array = DynArray::new();

        assert_eq!(array.pop().err().unwrap(), ArrayError::ArrayEmpty);

        // But this works.

        array.push(true.into()).unwrap();

        assert_eq!(array.pop().unwrap().bool().unwrap(), true);
    }

    #[test]
    fn DynArrayError_IncorrectValueType() {
        let mut array = DynArray::new();

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
            array.push(DynTable::new().into()).err().unwrap(),
            ArrayError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            array.push(DynArray::new().into()).err().unwrap(),
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
            array.set(0, DynTable::new().into()).err().unwrap(),
            ArrayError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            array.set(0, DynArray::new().into()).err().unwrap(),
            ArrayError::IncorrectValueType(ValueType::Bool)
        );

        // But this works.

        array.clear();

        array.push(7.into()).unwrap();
        array.push(3.14.into()).unwrap();
    }

    #[test]
    fn basic() {
        // Create an empty array.
        let mut array = DynArray::new();
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
            let mut nested_array = DynArray::new();
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
    }
}
