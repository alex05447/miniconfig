use std::fmt::{Display, Formatter};
use std::iter::Iterator;

use crate::{BinArrayGetError, BinConfigValue, BinTable, DisplayLua, Value};

use super::array_or_table::BinArrayOrTable;
use super::value::BinConfigUnpackedValue;

/// Represents an immutable array of [`Value`]'s with integer `0`-based indices.
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
    ///
    /// Returns an [`error`] if `index` is out of bounds.
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: struct.BinArrayGetError.html
    pub fn get(&self, index: u32) -> Result<BinConfigValue<'a>, BinArrayGetError> {
        self.get_impl(index)
    }

    /// Tries to get a [`bool`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: struct.BinArrayGetError.html
    pub fn get_bool(&self, index: u32) -> Result<bool, BinArrayGetError> {
        let val = self.get(index)?;
        val.bool()
            .ok_or(BinArrayGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`i64`].
    ///
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: struct.BinArrayGetError.html
    pub fn get_i64(&self, index: u32) -> Result<i64, BinArrayGetError> {
        let val = self.get(index)?;
        val.i64()
            .ok_or(BinArrayGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`f64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: struct.BinArrayGetError.html
    pub fn get_f64(&self, index: u32) -> Result<f64, BinArrayGetError> {
        let val = self.get(index)?;
        val.f64()
            .ok_or(BinArrayGetError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`string`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: struct.BinArrayGetError.html
    pub fn get_string(&self, index: u32) -> Result<&'a str, BinArrayGetError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.string()
            .ok_or(BinArrayGetError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`](enum.Value.html#variant.Array) [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`array`](enum.Value.html#variant.Array).
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: struct.BinArrayGetError.html
    pub fn get_array(&self, index: u32) -> Result<BinArray<'a>, BinArrayGetError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.array()
            .ok_or(BinArrayGetError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`table`].
    ///
    /// [`table`]: enum.Value.html#variant.Table
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: struct.BinArrayGetError.html
    pub fn get_table(&self, index: u32) -> Result<BinTable<'_>, BinArrayGetError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.table()
            .ok_or(BinArrayGetError::IncorrectValueType(val_type))
    }

    /// Returns an in-order iterator over [`values`] in the [`array`].
    ///
    /// [`values`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    pub fn iter<'i>(&'i self) -> impl Iterator<Item = BinConfigValue<'a>> + 'i {
        BinArrayIter::new(self)
    }

    pub(super) fn new(array: BinArrayOrTable<'a>) -> Self {
        Self(array)
    }

    fn get_impl(&self, index: u32) -> Result<BinConfigValue<'a>, BinArrayGetError> {
        use BinArrayGetError::*;

        // Index out of bounds.
        if index >= self.len() {
            Err(IndexOutOfBounds(self.len()))
        } else {
            use BinConfigUnpackedValue::*;

            // Safe to call - the config was validated.
            let value = match unsafe { self.0.value(index) } {
                Bool(val) => Value::Bool(val),
                I64(val) => Value::I64(val),
                F64(val) => Value::F64(val),
                BinConfigUnpackedValue::String { offset, len } => {
                    Value::String(unsafe { self.0.string(offset, len) })
                } // Safe to call - the string was validated.
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

    fn fmt_lua_impl(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        writeln!(f, "{{")?;

        // Iterate the array.
        for (index, value) in self.iter().enumerate() {
            <Self as DisplayLua>::do_indent(f, indent + 1)?;

            value.fmt_lua(f, indent + 1)?;

            write!(f, ",")?;

            let is_array_or_table = match value {
                Value::Table(_) | Value::Array(_) => true,
                _ => false,
            };

            if is_array_or_table {
                write!(f, " -- [{}]", index)?;
            }

            writeln!(f)?;
        }

        <Self as DisplayLua>::do_indent(f, indent)?;
        write!(f, "}}")?;

        Ok(())
    }
}

/// In-order iterator over [`values`] in the [`array`].
///
/// [`values`]: type.BinConfigValue.html
/// [`array`]: struct.BinArray.html
struct BinArrayIter<'i, 'a> {
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

impl<'a> DisplayLua for BinArray<'a> {
    fn fmt_lua(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(f, indent)
    }
}

impl<'a> Display for BinArray<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
    }
}
