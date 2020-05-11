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

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DynTableGetPathError {
    /// (Part of the) provided path does not exist in the [`table`].
    /// Contains the invalid path.
    /// [`table`]: struct.DynTable.html
    PathDoesNotExist(Vec<String>),
    /// Value at non-terminating path element is not a [`table`].
    /// Contains the invalid path and the value [`type`].
    /// [`table`]: struct.DynTable.html
    /// [`type`]: struct.ValueType.html
    ValueNotATable {
        path: Vec<String>,
        value_type: ValueType,
    },
    /// Value is of incorrect [`type`].
    /// Contains the value [`type`].
    /// [`type`]: struct.ValueType.html
    IncorrectValueType(ValueType),
}

struct Path<'p>(&'p Vec<String>);

impl<'p> Display for Path<'p> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        for (i, key) in self.0.iter().enumerate() {
            let last = i == self.0.len() - 1;

            write!(f, "\"{}\"", key)?;

            if !last {
                write!(f, ".")?;
            }
        }

        Ok(())
    }
}

impl Display for DynTableGetPathError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use DynTableGetPathError::*;

        match self {
            PathDoesNotExist(path) => write!(
                f,
                "(Part of the) provided path ({}) does not exist in the table.",
                Path(&path)
            ),
            ValueNotATable { path, value_type } => write!(
                f,
                "Value at non-terminating path element ({}) is not a table, but a \"{}\".",
                Path(&path),
                value_type
            ),
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
    /// [`table`]: struct.DynTable.html
    KeyDoesNotExist,
    /// Provided [`table`] key is empty.
    /// [`table`]: struct.DynTable.html
    EmptyKey,
}

impl Display for DynTableSetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use DynTableSetError::*;

        match self {
            KeyDoesNotExist => write!(f, "Tried to remove a non-existant value from the table."),
            EmptyKey => write!(f, "Provided table key is empty."),
        }
    }
}
