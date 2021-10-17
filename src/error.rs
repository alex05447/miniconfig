use {
    crate::*,
    std::{
        error::Error,
        fmt::{Display, Formatter},
    },
};

/// An error returned by [`table`] accessors.
///
/// [`table`]: enum.Value.html#variant.Table
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TableError {
    /// Provided [`table`] key is empty.
    ///
    /// [`table`]: enum.Value.html#variant.Table
    EmptyKey,
    /// Provided key does not exist in the [`table`].
    ///
    /// [`table`]: enum.Value.html#variant.Table
    KeyDoesNotExist,
    /// [`Table`] value is of incorrect and incompatible [`type`].
    /// Contains the actual value [`type`].
    ///
    /// [`Table`]: enum.Value.html#variant.Table
    /// [`type`]: enum.Value.html#variant.Table
    IncorrectValueType(ValueType),
}

impl Error for TableError {}

impl Display for TableError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use TableError::*;

        match self {
            EmptyKey => "provided table key is empty".fmt(f),
            KeyDoesNotExist => "provided key does not exist in the table".fmt(f),
            IncorrectValueType(actual_type) => {
                write!(
                    f,
                    "table value is of incorrect and incompatible type (expected {})",
                    actual_type
                )
            }
        }
    }
}

/// An error returned by [`array`] accessors.
///
/// [`array`]: enum.Value.html#variant.Array
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrayError {
    /// [`Array`] index out of bounds.
    /// Contains the actual [`array`] length.
    ///
    /// [`Array`]: enum.Value.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    IndexOutOfBounds(u32),
    /// Tried to `pop` an empty [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    ArrayEmpty,
    /// [`Array`] value is of incorrect and incompatible [`type`].
    /// Contains the actual value [`type`].
    ///
    /// [`Array`]: enum.Value.html#variant.Array
    /// [`type`]: enum.Value.html#variant.Array
    IncorrectValueType(ValueType),
}

impl Error for ArrayError {}

impl Display for ArrayError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use ArrayError::*;

        match self {
            IndexOutOfBounds(len) => write!(f, "array index out of bounds (length is {})", len),
            ArrayEmpty => "tried to pop an empty array".fmt(f),
            IncorrectValueType(actual_type) => {
                write!(
                    f,
                    "array value is of incorrect and incompatible type (expected {})",
                    actual_type
                )
            }
        }
    }
}

/// An error returned by [`table`] and [`array`] path accessors.
///
/// [`table`]: enum.Value.html#variant.Table
/// [`array`]: enum.Value.html#variant.Array
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum GetPathError {
    /// One of the provided [`table string keys`] is empty.
    /// Contains the path to the empty key, or an empty path if the empty key is in the root [`table`] / [`array`].
    ///
    /// [`table string keys`]: enum.ConfigKey.html#variant.Table
    /// [`table`]: enum.Value.html#variant.Table
    /// [`array`]: enum.Value.html#variant.Array
    EmptyKey(ConfigPath),
    /// One of the [`table string keys`] does not exist in the [`table`].
    /// Contains the path to the invalid key, or an empty path for the root [`table`]
    ///
    /// [`table string keys`]: enum.ConfigKey.html#variant.Table
    /// [`table`]: enum.Value.html#variant.Table
    KeyDoesNotExist(ConfigPath),
    /// One of the [`array index keys`] is out of bounds.
    ///
    /// [`array index keys`]: enum.ConfigKey.html#variant.Array
    IndexOutOfBounds {
        /// Path to the invalid index, or an empty path for the root [`array`].
        ///
        /// [`array`]: enum.Value.html#variant.Array
        path: ConfigPath,
        /// Actual [`array`] length.
        ///
        /// [`array`]: enum.Value.html#variant.Array
        len: u32,
    },
    /// Value with an [`array index key`] is not an [`array`].
    ///
    /// [`array index key`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    ValueNotAnArray {
        /// Path to the value, or an empty path for the root [`table`] / [`array`].
        ///
        /// [`table`]: enum.Value.html#variant.Table
        /// [`array`]: enum.Value.html#variant.Array
        path: ConfigPath,
        /// Actual value [`type`].
        ///
        /// [`type`]: enum.ValueType.html
        value_type: ValueType,
    },
    /// Value with a [`table string key`] is not a [`table`].
    ///
    /// [`table string key`]: enum.ConfigKey.html#variant.Table
    /// [`table`]: enum.Value.html#variant.Table
    ValueNotATable {
        /// Path to the value, or an empty path for the root [`table`] / [`array`].
        ///
        /// [`table`]: enum.Value.html#variant.Table
        /// [`array`]: enum.Value.html#variant.Array
        path: ConfigPath,
        /// Actual value [`type`].
        ///
        /// [`type`]: enum.ValueType.html
        value_type: ValueType,
    },
    /// Value is of incorrect and incompatible [`type`].
    /// Contains the actual value [`type`].
    ///
    /// [`type`]: enum.ValueType.html
    IncorrectValueType(ValueType),
}

impl GetPathError {
    /// Pushes the table key to the back of the path if the error has one.
    pub(crate) fn push_key(mut self, key: &NonEmptyStr) -> Self {
        use GetPathError::*;

        let key = OwnedConfigKey::Table(key.into());

        match &mut self {
            EmptyKey(path) => path.0.push(key),
            KeyDoesNotExist(path) => path.0.push(key),
            ValueNotAnArray { path, .. } => path.0.push(key),
            ValueNotATable { path, .. } => path.0.push(key),
            IndexOutOfBounds { path, .. } => path.0.push(key),
            IncorrectValueType(_) => {}
        }

        self
    }

    /// Pushes the array index to the back of the path if the error has one.
    pub(crate) fn push_index(mut self, index: u32) -> Self {
        use GetPathError::*;

        let index = OwnedConfigKey::Array(index);

        match &mut self {
            EmptyKey(path) => path.0.push(index),
            KeyDoesNotExist(path) => path.0.push(index),
            ValueNotAnArray { path, .. } => path.0.push(index),
            ValueNotATable { path, .. } => path.0.push(index),
            IndexOutOfBounds { path, .. } => path.0.push(index),
            IncorrectValueType(_) => {}
        }

        self
    }

    /// Reverses the path if the error has one.
    /// Must do this because path elements were pushed to the back of the `Vec`
    /// when unwinding the stack on error.
    /// (Alternatively we could always push path elements to the front, but that would constantly shuffle the `Vec`).
    pub(crate) fn reverse(mut self) -> Self {
        use GetPathError::*;

        match &mut self {
            EmptyKey(path) => path.0.reverse(),
            KeyDoesNotExist(path) => path.0.reverse(),
            ValueNotAnArray { path, .. } => path.0.reverse(),
            ValueNotATable { path, .. } => path.0.reverse(),
            IndexOutOfBounds { path, .. } => path.0.reverse(),
            IncorrectValueType(_) => {}
        };

        self
    }
}

impl<'a> Error for GetPathError {}

impl<'a> Display for GetPathError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use GetPathError::*;

        match self {
            EmptyKey(path) => write!(
                f,
                "one of the provided table string keys in {} is empty",
                path
            ),
            KeyDoesNotExist(path) => write!(f, "key {} does not exist in the table", path),
            IndexOutOfBounds { path, len } => write!(
                f,
                "array index in {} out of bounds (length is {})",
                path, len
            ),
            ValueNotAnArray { path, value_type } => write!(
                f,
                "value at {} is not an array (but a \"{}\")",
                path, value_type
            ),
            ValueNotATable { path, value_type } => write!(
                f,
                "value at {} is not an table (but a \"{}\")",
                path, value_type
            ),
            IncorrectValueType(actual_type) => write!(
                f,
                "value is of incorrect and incompatible type (expected {})",
                actual_type
            ),
        }
    }
}
