use std::fmt::{Display, Formatter};

use crate::ValueType;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DynArrayGetError {
    /// Index was out of bounds.
    /// Contains the [`array`] length.
    /// [`array`]: struct.DynArray.html
    IndexOutOfBounds(u32),
    /// Tried to pop an empty [`array`].
    /// [`array`]: struct.DynArray.html
    ArrayEmpty,
    /// Value is of incorrect [`type`].
    /// Contains the value [`type`].
    /// [`type`]: struct.ValueType.html
    IncorrectValueType(ValueType),
}

impl Display for DynArrayGetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use DynArrayGetError::*;

        match self {
            IndexOutOfBounds(len) => {
                write!(f, "Array index was out of bounds (length is {}).", len)
            }
            ArrayEmpty => write!(f, "Tried to pop an empty array."),
            IncorrectValueType(invalid_type) => {
                write!(f, "Value is of incorrect type ({}).", invalid_type)
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DynArraySetError {
    /// Index was out of bounds.
    /// Contains the [`array`] length.
    /// [`array`]: struct.DynArray.html
    IndexOutOfBounds(u32),
    /// Incorrect [`value type`] for the [`array`].
    /// Contains the correct [`array`] [`value type`].
    /// [`value type`]: struct.ValueType.html
    /// [`array`]: struct.DynArray.html
    InvalidValueType(ValueType),
}

impl Display for DynArraySetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use DynArraySetError::*;

        match self {
            IndexOutOfBounds(len) => {
                write!(f, "Index was out of bounds (array length is {}).", len)
            }
            InvalidValueType(value_type) => write!(
                f,
                "Incorrect value type for the array (expected \"{}\").",
                value_type
            ),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DynTableGetError {
    /// Provided key does not exist in the [`table`].
    /// [`table`]: struct.DynTable.html
    KeyDoesNotExist,
    /// Value is of incorrect [`type`].
    /// Contains the value [`type`].
    /// [`type`]: struct.ValueType.html
    IncorrectValueType(ValueType),
}

impl Display for DynTableGetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use DynTableGetError::*;

        match self {
            KeyDoesNotExist => write!(f, "Provided key does not exist in the table."),
            IncorrectValueType(invalid_type) => {
                write!(f, "Value is of incorrect type ({}).", invalid_type)
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DynTableSetError {
    /// Tried to remove a non-existant [`value`] from the [`table`].
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    KeyDoesNotExist,
    /// Provided [`table`] key is invalid (contains non-alphanumeric ASCII characters or underscores).
    /// [`table`]: struct.LuaTable.html
    InvalidKey,
}

impl Display for DynTableSetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use DynTableSetError::*;

        match self {
            KeyDoesNotExist => write!(f, "Tried to remove a non-existant value from the table."),
            InvalidKey => write!(f, "Provided table key is invalid (contains non-alphanumeric ASCII characters or underscores)."),
        }
    }
}
