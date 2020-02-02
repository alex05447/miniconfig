use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::slice::Iter as VecIter;

use crate::{
    DisplayIndent, DynArrayGetError, DynArraySetError, DynTable, DynTableMut, DynTableRef, Value,
};

/// Represents a mutable array of [`Value`]'s with integer 0-based indices.
///
/// [`Value`]: struct.Value.html
pub struct DynArray(Vec<Value<String, DynArray, DynTable>>);

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

    /// Tries to get a reference to a [`value`] in the [`array`] at `index`.
    /// Returns an [`error`] if `index` is out of bounds.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: struct.DynArrayGetError.html
    pub fn get(
        &self,
        index: u32,
    ) -> Result<Value<&'_ str, DynArrayRef<'_>, DynTableRef<'_>>, DynArrayGetError> {
        self.get_impl(index)
    }

    /// Tries to get a `bool` [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    pub fn get_bool(
        &self,
        index: u32,
    ) -> Result<bool, DynArrayGetError> {
        let val = self.get(index)?;
        val.bool().ok_or_else(|| DynArrayGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `i64` [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    pub fn get_i64(
        &self,
        index: u32,
    ) -> Result<i64, DynArrayGetError> {
        let val = self.get(index)?;
        val.i64().ok_or_else(|| DynArrayGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `f64` [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    pub fn get_f64(
        &self,
        index: u32,
    ) -> Result<f64, DynArrayGetError> {
        let val = self.get(index)?;
        val.f64().ok_or_else(|| DynArrayGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a string [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    pub fn get_string(
        &self,
        index: u32,
    ) -> Result<&str, DynArrayGetError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.string().ok_or_else(|| DynArrayGetError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`] [`value`] in the [`array`] at `index`.
    ///
    /// [`array`]: struct.DynArray.html
    /// [`value`]: enum.Value.html
    pub fn get_array(
        &self,
        index: u32,
    ) -> Result<DynArrayRef<'_>, DynArrayGetError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.array().ok_or_else(|| DynArrayGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`] [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.DynTable.html
    /// [`array`]: struct.DynArray.html
    pub fn get_table(
        &self,
        index: u32,
    ) -> Result<DynTableRef<'_>, DynArrayGetError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.table().ok_or_else(|| DynArrayGetError::IncorrectValueType(val_type))
    }

    /// Returns an in-order [`iterator`] over [`values`] in the [`array`].
    ///
    /// [`iterator`]: struct.DynArrayIter.html
    /// [`values`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    pub fn iter(&self) -> DynArrayIter<'_> {
        DynArrayIter(self.0.iter())
    }

    /// Tries to get a mutable reference to a [`value`] in the [`array`] at `index`.
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`array`].
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_mut(
        &mut self,
        index: u32,
    ) -> Result<Value<&'_ str, DynArrayMut<'_>, DynTableMut<'_>>, DynArrayGetError> {
        self.get_mut_impl(index)
    }

    /// Changes the [`value`] in the [`array`] at `index` to `value`.
    /// Returns an [`error`] if `index` is out of bounds or if `value` is of invalid type.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: struct.DynArraySetError.html
    pub fn set<'s>(
        &mut self,
        index: u32,
        value: Value<&'s str, DynArray, DynTable>,
    ) -> Result<(), DynArraySetError> {
        self.set_impl(index, value)
    }

    /// Pushes the [`value`] to the back of the [`array`].
    /// Returns an [`error`] if `value` is of invalid type.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: struct.DynArraySetError.html
    pub fn push<'s>(
        &mut self,
        value: Value<&'s str, DynArray, DynTable>,
    ) -> Result<(), DynArraySetError> {
        self.push_impl(value)
    }

    /// Pops the [`value`] off the back of the [`array`].
    /// Returns an [`error`] if the [`array`] is empty.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.DynArray.html
    /// [`error`]: struct.DynArrayGetError.html
    pub fn pop(&mut self) -> Result<Value<String, DynArray, DynTable>, DynArrayGetError> {
        self.pop_impl()
    }

    fn len_impl(&self) -> u32 {
        self.0.len() as u32
    }

    fn get_impl(
        &self,
        index: u32,
    ) -> Result<Value<&'_ str, DynArrayRef<'_>, DynTableRef<'_>>, DynArrayGetError> {
        use DynArrayGetError::*;

        let len = self.len();

        if index >= len {
            Err(IndexOutOfBounds(len))
        } else {
            unsafe {
                let value = match self.0.get_unchecked(index as usize) {
                    Value::Bool(value) => Value::Bool(*value),
                    Value::I64(value) => Value::I64(*value),
                    Value::F64(value) => Value::F64(*value),
                    Value::String(value) => Value::String(value.as_str()),
                    Value::Array(value) => Value::Array(DynArrayRef::new(value)),
                    Value::Table(value) => Value::Table(DynTableRef::new(value)),
                };

                Ok(value)
            }
        }
    }

    fn get_mut_impl(
        &mut self,
        index: u32,
    ) -> Result<Value<&'_ str, DynArrayMut<'_>, DynTableMut<'_>>, DynArrayGetError> {
        use DynArrayGetError::*;

        let len = self.len();

        if index >= len {
            Err(IndexOutOfBounds(len))
        } else {
            unsafe {
                let value = match self.0.get_unchecked_mut(index as usize) {
                    Value::Bool(value) => Value::Bool(*value),
                    Value::I64(value) => Value::I64(*value),
                    Value::F64(value) => Value::F64(*value),
                    Value::String(value) => Value::String(value.as_str()),
                    Value::Array(value) => Value::Array(DynArrayMut::new(value)),
                    Value::Table(value) => Value::Table(DynTableMut::new(value)),
                };

                Ok(value)
            }
        }
    }

    fn validate_value_type<S: Into<String>>(
        &self,
        value: &Value<S, DynArray, DynTable>,
    ) -> Result<(), DynArraySetError> {
        use DynArraySetError::*;

        let value_type = value.get_type();

        // If array is non-empty and has a value type, ensure the provided value type is compatible.
        if self.len() > 0 {
            let array_value_type = unsafe { self.0.get_unchecked(0).get_type() };

            if !array_value_type.is_compatible(value_type) {
                return Err(InvalidValueType(array_value_type));
            }
        }
        // Else the array has no type.

        Ok(())
    }

    fn set_impl<'s>(
        &mut self,
        index: u32,
        value: Value<&'s str, DynArray, DynTable>,
    ) -> Result<(), DynArraySetError> {
        use DynArraySetError::*;

        // Validate the index.
        let len = self.len();

        if index >= len {
            return Err(IndexOutOfBounds(len));
        }

        // Validate the value type.
        self.validate_value_type(&value)?;

        let index = index as usize;

        match value {
            Value::Bool(value) => self.0[index] = Value::Bool(value),
            Value::I64(value) => self.0[index] = Value::I64(value),
            Value::F64(value) => self.0[index] = Value::F64(value),
            Value::String(value) => self.0[index] = Value::String(value.to_owned()),
            Value::Array(value) => self.0[index] = Value::Array(value),
            Value::Table(value) => self.0[index] = Value::Table(value),
        }

        Ok(())
    }

    fn push_impl<'s>(
        &mut self,
        value: Value<&'s str, DynArray, DynTable>,
    ) -> Result<(), DynArraySetError> {
        // Validate the value type.
        self.validate_value_type(&value)?;

        match value {
            Value::Bool(value) => self.0.push(Value::Bool(value)),
            Value::I64(value) => self.0.push(Value::I64(value)),
            Value::F64(value) => self.0.push(Value::F64(value)),
            Value::String(value) => self.0.push(Value::String(value.to_owned())),
            Value::Array(value) => self.0.push(Value::Array(value)),
            Value::Table(value) => self.0.push(Value::Table(value)),
        };

        Ok(())
    }

    fn pop_impl(&mut self) -> Result<Value<String, DynArray, DynTable>, DynArrayGetError> {
        self.0.pop().ok_or(DynArrayGetError::ArrayEmpty)
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

/// Represents an immutable reference to an [`array`].
///
/// [`array`]: struct.DynArray.html
pub struct DynArrayRef<'a>(&'a DynArray);

impl<'a> DynArrayRef<'a> {
    pub(super) fn new(inner: &'a DynArray) -> Self {
        Self(inner)
    }
}

impl<'a> std::ops::Deref for DynArrayRef<'a> {
    type Target = DynArray;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// Represents a mutable reference to an [`array`].
///
/// [`Value`]: struct.Value.html
/// [`array`]: struct.DynArray.html
pub struct DynArrayMut<'a>(&'a mut DynArray);

impl<'a> DynArrayMut<'a> {
    pub(super) fn new(inner: &'a mut DynArray) -> Self {
        Self(inner)
    }
}

impl<'a> Deref for DynArrayMut<'a> {
    type Target = DynArray;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> DerefMut for DynArrayMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

/// In-order iterator over [`values`] in the [`array`].
///
/// [`values`]: enum.Value.html
/// [`array`]: struct.DynArray.html
pub struct DynArrayIter<'a>(VecIter<'a, Value<String, DynArray, DynTable>>);

impl<'a> Iterator for DynArrayIter<'a> {
    type Item = Value<&'a str, DynArrayRef<'a>, DynTableRef<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(value) = self.0.next() {
            let value = match value {
                Value::Bool(value) => Value::Bool(*value),
                Value::I64(value) => Value::I64(*value),
                Value::F64(value) => Value::F64(*value),
                Value::String(value) => Value::String(value.as_str()),
                Value::Array(value) => Value::Array(DynArrayRef::new(value)),
                Value::Table(value) => Value::Table(DynTableRef::new(value)),
            };

            Some(value)
        } else {
            None
        }
    }
}

impl DisplayIndent for DynArray {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        self.fmt_indent_impl(f, indent, comma)
    }
}

impl<'a> DisplayIndent for DynArrayRef<'a> {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        self.fmt_indent_impl(f, indent, comma)
    }
}

impl<'a> DisplayIndent for DynArrayMut<'a> {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        self.fmt_indent_impl(f, indent, comma)
    }
}

impl Display for DynArray {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_indent_impl(f, 0, true)
    }
}
