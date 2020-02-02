use std::fmt::{Display, Formatter};
use std::iter::Iterator;

use super::array_or_table::BinArrayOrTable;
use super::value::BinConfigValue;
use crate::{BinArrayGetError, BinTable, DisplayIndent, Value};

/// Represents an immutable array of [`Value`]'s with integer 0-based indices.
///
/// [`Value`]: struct.Value.html
pub struct BinArray<'a>(pub(super) BinArrayOrTable<'a>);

impl<'a> BinArray<'a> {
    /// Returns the length of the [`array`].
    ///
    /// [`array`]: struct.BinArray.html
    pub fn len(&self) -> u32 {
        self.0.len
    }

    /// Tries to get a reference to a [`value`] in the [`array`] at `index`.
    /// Returns an [`error`] if `index` is out of bounds.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: struct.BinArrayGetError.html
    pub fn get(
        &self,
        index: u32,
    ) -> Result<Value<&'a str, BinArray<'a>, BinTable<'a>>, BinArrayGetError> {
        self.get_impl(index)
    }

    /// Tries to get a `bool` [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.BinArray.html
    pub fn get_bool(&self, index: u32) -> Result<bool, BinArrayGetError> {
        let val = self.get(index)?;
        val.bool()
            .ok_or_else(|| BinArrayGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `i64` [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.BinArray.html
    pub fn get_i64(&self, index: u32) -> Result<i64, BinArrayGetError> {
        let val = self.get(index)?;
        val.i64()
            .ok_or_else(|| BinArrayGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an `f64` [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.BinArray.html
    pub fn get_f64(&self, index: u32) -> Result<f64, BinArrayGetError> {
        let val = self.get(index)?;
        val.f64()
            .ok_or_else(|| BinArrayGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a string [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`array`]: struct.BinArray.html
    pub fn get_string(&self, index: u32) -> Result<&str, BinArrayGetError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| BinArrayGetError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`] [`value`] in the [`array`] at `index`.
    ///
    /// [`array`]: struct.BinArray.html
    /// [`value`]: enum.Value.html
    pub fn get_array(&self, index: u32) -> Result<BinArray<'_>, BinArrayGetError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| BinArrayGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`] [`value`] in the [`array`] at `index`.
    ///
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.BinTable.html
    /// [`array`]: struct.BinArray.html
    pub fn get_table(&self, index: u32) -> Result<BinTable<'_>, BinArrayGetError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| BinArrayGetError::IncorrectValueType(val_type))
    }

    /// Returns an in-order [`iterator`] over [`values`] in the [`array`].
    ///
    /// [`iterator`]: struct.BinArrayIter.html
    /// [`values`]: enum.Value.html
    /// [`array`]: struct.BinArray.html
    pub fn iter(&self) -> BinArrayIter<'_, 'a> {
        BinArrayIter::new(self)
    }

    pub(super) fn new(array: BinArrayOrTable<'a>) -> Self {
        Self(array)
    }

    fn get_impl(
        &self,
        index: u32,
    ) -> Result<Value<&'a str, BinArray<'a>, BinTable<'a>>, BinArrayGetError> {
        use BinArrayGetError::*;

        // Index out of bounds.
        if index >= self.len() {
            Err(IndexOutOfBounds(self.len()))
        } else {
            use BinConfigValue::*;

            // Safe to call - the config was validated.
            let value = match unsafe { self.0.value(index) } {
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
            };

            Ok(value)
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
/// [`array`]: struct.BinArray.html
pub struct BinArrayIter<'i, 'a> {
    array: &'i BinArray<'a>,
    index: u32,
}

impl<'i, 'a> BinArrayIter<'i, 'a> {
    fn new(array: &'i BinArray<'a>) -> Self {
        Self { array, index: 0 }
    }
}

impl<'i, 'a> Iterator for BinArrayIter<'i, 'a> {
    type Item = Value<&'a str, BinArray<'a>, BinTable<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;

        if index < self.array.len() {
            self.index += 1;

            // Must succeed.
            Some(self.array.get(index).unwrap())
        } else {
            None
        }
    }
}

impl<'a> DisplayIndent for BinArray<'a> {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result {
        self.fmt_indent_impl(f, indent, comma)
    }
}

impl<'a> Display for BinArray<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_indent_impl(f, 0, true)
    }
}
