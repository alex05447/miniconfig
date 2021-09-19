use {
    super::{array_or_table::BinArrayOrTable, value::BinConfigUnpackedValue},
    crate::{util::*, *},
    std::{
        borrow::Borrow,
        fmt::{Display, Formatter, Write},
        iter::Iterator,
    },
};

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

    /// Returns `true` if the [`array`] is empty.
    ///
    /// [`array`]: struct.BinArray.html
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Tries to get a reference to a [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds.
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: enum.BinArrayError.html
    pub fn get(&self, index: u32) -> Result<BinConfigValue<'a>, BinArrayError> {
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
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    /// [`type`]: enum.Value.html
    pub fn get_path<'b, K, P>(&self, path: P) -> Result<BinConfigValue<'_>, GetPathError<'b>>
    where
        K: Borrow<ConfigKey<'b>>,
        P: IntoIterator<Item = K>,
    {
        BinConfigValue::Array(BinArray(self.0.clone()))
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a [`bool`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: enum.BinArrayError.html
    pub fn get_bool(&self, index: u32) -> Result<bool, BinArrayError> {
        let val = self.get(index)?;
        val.bool()
            .ok_or_else(|| BinArrayError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`bool`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to a [`bool`] [`value`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_bool_path<'b, K, P>(&self, path: P) -> Result<bool, GetPathError<'b>>
    where
        K: Borrow<ConfigKey<'b>>,
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
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: enum.BinArrayError.html
    pub fn get_i64(&self, index: u32) -> Result<i64, BinArrayError> {
        let val = self.get(index)?;
        val.i64()
            .ok_or_else(|| BinArrayError::IncorrectValueType(val.get_type()))
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
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_i64_path<'b, K, P>(&self, path: P) -> Result<i64, GetPathError<'b>>
    where
        K: Borrow<ConfigKey<'b>>,
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
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: enum.BinArrayError.html
    pub fn get_f64(&self, index: u32) -> Result<f64, BinArrayError> {
        let val = self.get(index)?;
        val.f64()
            .ok_or_else(|| BinArrayError::IncorrectValueType(val.get_type()))
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
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_f64_path<'b, K, P>(&self, path: P) -> Result<f64, GetPathError<'b>>
    where
        K: Borrow<ConfigKey<'b>>,
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
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: enum.BinArrayError.html
    pub fn get_string(&self, index: u32) -> Result<&'a str, BinArrayError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| BinArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`string`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to a [`string`] [`value`].
    ///
    /// [`string`]: enum.Value.html#variant.I64
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_string_path<'b, K, P>(&self, path: P) -> Result<&str, GetPathError<'b>>
    where
        K: Borrow<ConfigKey<'b>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an [`array`](enum.Value.html#variant.Array) [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not an [`array`](enum.Value.html#variant.Array).
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: enum.BinArrayError.html
    pub fn get_array(&self, index: u32) -> Result<BinArray<'a>, BinArrayError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| BinArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`](enum.Value.html#variant.Array) [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to an [`array`](enum.Value.html#variant.Array) [`value`].
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`table`]: enum.Value.html#variant.Table
    pub fn get_array_path<'b, K, P>(&self, path: P) -> Result<BinArray<'_>, GetPathError<'b>>
    where
        K: Borrow<ConfigKey<'b>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`table`] [`value`] in the [`array`] at `index`.
    ///
    /// Returns an [`error`] if `index` is out of bounds or if value is not a [`table`].
    ///
    /// [`table`]: enum.Value.html#variant.Table
    /// [`value`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    /// [`error`]: enum.BinArrayError.html
    pub fn get_table(&self, index: u32) -> Result<BinTable<'_>, BinArrayError> {
        let val = self.get(index)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| BinArrayError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to a [`table`] [`value`] in the [`array`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`](enum.Value.html#variant.Array) value.
    /// The last key must correspond to a [`table`] [`value`].
    ///
    /// [`value`]: type.BinConfigValue.html
    /// [`table`]: enum.Value.html#variant.Table
    /// [`array`]: struct.BinArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    pub fn get_table_path<'b, K, P>(&self, path: P) -> Result<BinTable<'_>, GetPathError<'b>>
    where
        K: Borrow<ConfigKey<'b>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Returns an in-order iterator over [`values`] in the [`array`].
    ///
    /// [`values`]: type.BinConfigValue.html
    /// [`array`]: struct.BinArray.html
    pub fn iter(&self) -> impl Iterator<Item = BinConfigValue<'a>> {
        BinArrayIter::new(BinArray(self.0.clone()))
    }

    pub(super) fn new(array: BinArrayOrTable<'a>) -> Self {
        Self(array)
    }

    fn get_impl(&self, index: u32) -> Result<BinConfigValue<'a>, BinArrayError> {
        use BinArrayError::*;

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
                    self.0.key_table,
                    offset,
                    len,
                ))),
                Table { offset, len } => Value::Table(BinTable::new(BinArrayOrTable::new(
                    self.0.base,
                    self.0.key_table,
                    offset,
                    len,
                ))),
            };

            Ok(value)
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
/// [`values`]: type.BinConfigValue.html
/// [`array`]: struct.BinArray.html
struct BinArrayIter<'a> {
    array: BinArray<'a>,
    index: u32,
}

impl<'a> BinArrayIter<'a> {
    fn new(array: BinArray<'a>) -> Self {
        Self { array, index: 0 }
    }
}

impl<'a> Iterator for BinArrayIter<'a> {
    type Item = Value<&'a str, BinArray<'a>, BinTable<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;

        if index < self.array.len() {
            self.index += 1;

            // Must succeed - all indices are valid.
            Some(unwrap_unchecked_msg(
                self.array.get(index),
                "invalid index in array iterator",
            ))
        } else {
            None
        }
    }
}

impl<'a> DisplayLua for BinArray<'a> {
    fn fmt_lua<W: Write>(&self, f: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(f, indent)
    }
}

impl<'a> Display for BinArray<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use {crate::*, ministr_macro::nestr};

    #[test]
    fn BinArrayError_IndexOutOfBounds() {
        let mut writer = BinConfigWriter::new(1).unwrap();

        writer.array(nestr!("array"), 1).unwrap();
        writer.bool(None, true).unwrap();
        writer.end().unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config
                .root()
                .get_array("array".into())
                .unwrap()
                .get(1)
                .err()
                .unwrap(),
            BinArrayError::IndexOutOfBounds(1)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config
                    .root()
                    .get_array(key!("array"))
                    .unwrap()
                    .get(1)
                    .err()
                    .unwrap(),
                BinArrayError::IndexOutOfBounds(1)
            );
        }
        assert_eq!(
            config
                .root()
                .get_array_str("array")
                .unwrap()
                .get(1)
                .err()
                .unwrap(),
            BinArrayError::IndexOutOfBounds(1)
        );

        // But this works.

        assert_eq!(
            config
                .root()
                .get_array("array".into())
                .unwrap()
                .get_bool(0)
                .unwrap(),
            true
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config
                    .root()
                    .get_array(key!("array"))
                    .unwrap()
                    .get_bool(0)
                    .unwrap(),
                true
            );
        }
        assert_eq!(
            config
                .root()
                .get_array_str("array")
                .unwrap()
                .get_bool(0)
                .unwrap(),
            true
        );
    }

    #[test]
    fn BinArrayError_IncorrectValueType() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array(nestr!("array"), 1).unwrap();
        writer.f64(None, 3.14).unwrap();
        writer.end().unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config
                .root()
                .get_array("array".into())
                .unwrap()
                .get_bool(0)
                .err()
                .unwrap(),
            BinArrayError::IncorrectValueType(ValueType::F64)
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config
                    .root()
                    .get_array(key!("array"))
                    .unwrap()
                    .get_bool(0)
                    .err()
                    .unwrap(),
                BinArrayError::IncorrectValueType(ValueType::F64)
            );
        }
        assert_eq!(
            config
                .root()
                .get_array_str("array")
                .unwrap()
                .get_bool(0)
                .err()
                .unwrap(),
            BinArrayError::IncorrectValueType(ValueType::F64)
        );

        // But this works.

        assert_eq!(
            config
                .root()
                .get_array("array".into())
                .unwrap()
                .get_i64(0)
                .unwrap(),
            3
        );
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config
                    .root()
                    .get_array(key!("array"))
                    .unwrap()
                    .get_i64(0)
                    .unwrap(),
                3
            );
        }
        assert_eq!(
            config
                .root()
                .get_array_str("array")
                .unwrap()
                .get_i64(0)
                .unwrap(),
            3
        );
        assert!(cmp_f64(
            config
                .root()
                .get_array("array".into())
                .unwrap()
                .get_f64(0)
                .unwrap(),
            3.14
        ));
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config
                    .root()
                    .get_array(key!("array"))
                    .unwrap()
                    .get_f64(0)
                    .unwrap(),
                3.14
            );
        }
        assert!(cmp_f64(
            config
                .root()
                .get_array_str("array")
                .unwrap()
                .get_f64(0)
                .unwrap(),
            3.14
        ));
    }
}
