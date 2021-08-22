use std::{
    convert::From,
    fmt::{Display, Formatter},
};

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
use {crate::util::*, std::fmt::Write};

/// Represents a config value.
///
/// Different config implementations may represent strings, arrays and tables differently.
#[derive(Clone)]
pub enum Value<S, A, T> {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(S),
    Array(A),
    Table(T),
}

impl<S, A, T> From<bool> for Value<S, A, T> {
    fn from(val: bool) -> Self {
        Value::Bool(val)
    }
}

impl<S, A, T> From<i64> for Value<S, A, T> {
    fn from(val: i64) -> Self {
        Value::I64(val)
    }
}

impl<S, A, T> From<f64> for Value<S, A, T> {
    fn from(val: f64) -> Self {
        Value::F64(val)
    }
}

#[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
impl<S, A, T> DisplayLua for Value<S, A, T>
where
    S: AsRef<str>,
    A: DisplayLua,
    T: DisplayLua,
{
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        match self {
            Value::Bool(value) => write!(w, "{}", if *value { "true" } else { "false" }),
            Value::I64(value) => write!(w, "{}", value),
            Value::F64(value) => write!(w, "{}", value),
            Value::String(value) => write_lua_string(w, value.as_ref()),
            Value::Array(value) => value.fmt_lua(w, indent),
            Value::Table(value) => value.fmt_lua(w, indent),
        }
    }
}

/// Represents the type of the [`config value`].
///
/// [`config value`]: enum.Value.html
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValueType {
    Bool,
    I64,
    F64,
    String,
    Array,
    Table,
}

impl<S, A, T> Value<S, A, T> {
    /// Returns the config value type.
    pub fn get_type(&self) -> ValueType {
        use ValueType::*;

        match self {
            Value::Bool(_) => Bool,
            Value::I64(_) => I64,
            Value::F64(_) => F64,
            Value::String(_) => String,
            Value::Array(_) => Array,
            Value::Table(_) => Table,
        }
    }

    /// Extracts the [`bool`] value from the config value.
    /// Returns `None` if the value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    pub fn bool(&self) -> Option<bool> {
        match self {
            Value::Bool(val) => Some(*val),
            _ => None,
        }
    }

    /// Extracts the [`i64`] value from the config value.
    /// Returns `None` if the value is not an [`i64`] / [`f64`].
    ///
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`f64`]: enum.Value.html#variant.F64
    pub fn i64(&self) -> Option<i64> {
        match self {
            Value::I64(val) => Some(*val),
            Value::F64(val) => Some(*val as i64),
            _ => None,
        }
    }

    /// Extracts the [`f64`] value from the config value.
    /// Returns `None` if the value is not an [`f64`] / [`i64`].
    ///
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`f64`]: enum.Value.html#variant.F64
    pub fn f64(&self) -> Option<f64> {
        match self {
            Value::I64(val) => Some(*val as f64),
            Value::F64(val) => Some(*val),
            _ => None,
        }
    }

    /// Extracts the [`string`] value from the config value.
    /// Returns `None` if the value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    pub fn string(self) -> Option<S> {
        match self {
            Value::String(val) => Some(val),
            _ => None,
        }
    }

    /// Extracts the [`array`] value from the config value.
    /// Returns `None` if the value is not an [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    pub fn array(self) -> Option<A> {
        match self {
            Value::Array(val) => Some(val),
            _ => None,
        }
    }

    /// Extracts the [`table`] value from the config value.
    /// Returns `None` if the value is not a [`table`].
    ///
    /// [`table`]: enum.Value.html#variant.Table
    pub fn table(self) -> Option<T> {
        match self {
            Value::Table(val) => Some(val),
            _ => None,
        }
    }
}

impl<S: AsRef<str>, A, T> Value<S, A, T> {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(val) => Some(val.as_ref()),
            _ => None,
        }
    }
}

impl ValueType {
    #[cfg(any(feature = "bin", feature = "dyn", feature = "lua"))]
    pub(crate) fn is_compatible(self, other: ValueType) -> bool {
        use ValueType::*;

        match self {
            Bool => other == Bool,
            I64 => (other == I64) || (other == F64),
            F64 => (other == I64) || (other == F64),
            String => other == String,
            Array => other == Array,
            Table => other == Table,
        }
    }
}

impl Display for ValueType {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use ValueType::*;

        match self {
            Bool => "Bool".fmt(f),
            I64 => "I64".fmt(f),
            F64 => "F64".fmt(f),
            String => "String".fmt(f),
            Array => "Array".fmt(f),
            Table => "Table".fmt(f),
        }
    }
}

#[cfg(any(feature = "bin", feature = "lua"))]
pub(crate) fn value_type_to_u32<V: Into<Option<ValueType>>>(val: V) -> u32 {
    use ValueType::*;

    let val = val.into();

    if let Some(val) = val {
        match val {
            Bool => 1,
            I64 => 2,
            F64 => 3,
            String => 4,
            Array => 5,
            Table => 6,
        }
    } else {
        0
    }
}

#[cfg(any(feature = "bin", feature = "lua"))]
pub(crate) fn value_type_from_u32(val: u32) -> Option<ValueType> {
    use ValueType::*;

    match val {
        0 => None,
        1 => Some(Bool),
        2 => Some(I64),
        3 => Some(F64),
        4 => Some(String),
        5 => Some(Array),
        6 => Some(Table),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(feature = "bin", feature = "lua"))]
    use super::*;

    #[cfg(any(feature = "bin", feature = "lua"))]
    #[test]
    fn value_type_to_u32_and_back() {
        assert_eq!(value_type_from_u32(value_type_to_u32(None)), None);
        assert_eq!(
            value_type_from_u32(value_type_to_u32(ValueType::Bool)),
            Some(ValueType::Bool)
        );
        assert_eq!(
            value_type_from_u32(value_type_to_u32(ValueType::I64)),
            Some(ValueType::I64)
        );
        assert_eq!(
            value_type_from_u32(value_type_to_u32(ValueType::F64)),
            Some(ValueType::F64)
        );
        assert_eq!(
            value_type_from_u32(value_type_to_u32(ValueType::String)),
            Some(ValueType::String)
        );
        assert_eq!(
            value_type_from_u32(value_type_to_u32(ValueType::Array)),
            Some(ValueType::Array)
        );
        assert_eq!(
            value_type_from_u32(value_type_to_u32(ValueType::Table)),
            Some(ValueType::Table)
        );
    }
}
