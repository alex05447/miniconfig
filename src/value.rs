use std::fmt::{Display, Formatter};

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ValueType {
    Bool,
    I64,
    F64,
    String,
    Array,
    Table,
}

impl ValueType {
    #[cfg(any(all(feature = "dyn", feature = "ini"), feature = "lua"))]
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
            Bool => write!(f, "Bool"),
            I64 => write!(f, "I64"),
            F64 => write!(f, "F64"),
            String => write!(f, "String"),
            Array => write!(f, "Array"),
            Table => write!(f, "Table"),
        }
    }
}

impl<S, A, T> Value<S, A, T> {
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

    pub fn bool(&self) -> Option<bool> {
        match self {
            Value::Bool(val) => Some(*val),
            _ => None,
        }
    }

    pub fn i64(&self) -> Option<i64> {
        match self {
            Value::I64(val) => Some(*val),
            Value::F64(val) => Some(*val as i64),
            _ => None,
        }
    }

    pub fn f64(&self) -> Option<f64> {
        match self {
            Value::I64(val) => Some(*val as f64),
            Value::F64(val) => Some(*val),
            _ => None,
        }
    }

    pub fn string(self) -> Option<S> {
        match self {
            Value::String(val) => Some(val),
            _ => None,
        }
    }

    pub fn array(self) -> Option<A> {
        match self {
            Value::Array(val) => Some(val),
            _ => None,
        }
    }

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

#[cfg(any(all(feature = "dyn", feature = "ini"), feature = "lua"))]
pub(crate) fn value_type_to_u32(val: Option<ValueType>) -> u32 {
    use ValueType::*;

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

#[cfg(any(all(feature = "dyn", feature = "ini"), feature = "lua"))]
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
