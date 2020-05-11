use std::fmt::{Display, Formatter};

use crate::ValueType;

use rlua_ext;

#[derive(Clone, Debug)]
pub enum LuaConfigError {
    /// Error loading the Lua config script.
    /// Contains the actual error.
    LuaScriptError(rlua::Error),
    /// Mixed string and integer keys in the Lua table.
    /// Contains the path to the table, or an empty string for the root table.
    MixedKeys(String),
    /// Mixed (and non-convertible) type values in the Lua array.
    MixedArray {
        /// Path to the table, or an empty string for the root table.
        path: String,
        /// Expected Lua value type (as determined by the first value in the array).
        expected: rlua_ext::ValueType,
        /// Found Lua value type.
        found: rlua_ext::ValueType,
    },
    /// Invalid key type in the Lua table - only strings and numbers are allowed.
    InvalidKeyType {
        /// Path to the table, or an empty string for the root table.
        path: String,
        /// Invalid key Lua value type.
        invalid_type: rlua_ext::ValueType,
    },
    /// Invalid string key UTF-8.
    InvalidKeyUTF8 {
        /// Path to the table, or an empty string for the root table.
        path: String,
        /// UTF-8 parse error.
        error: rlua::Error,
    },
    /// Empty key string.
    EmptyKey {
        /// Path to the table, or an empty string for the root table.
        path: String,
    },
    /// Invalid integer array index.
    /// Contains the path to the array, or an empty string for the root table.
    InvalidArrayIndex(String),
    /// Invalid Lua value type for any config value.
    InvalidValueType {
        /// Path to the table element.
        path: String,
        /// Invalid Lua value type.
        invalid_type: rlua_ext::ValueType,
    },
    /// Invalid string value UTF-8.
    InvalidValueUTF8 {
        /// Path to the table element.
        path: String,
        /// UTF-8 parse error.
        error: rlua::Error,
    },
}

impl Display for LuaConfigError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use LuaConfigError::*;

        match self {
            LuaScriptError(_) => write!(f, "Error loading a Lua config script."),
            MixedKeys(path) => write!(f, "Mixed string and integer keys in the Lua table \"{}\".", if path.is_empty() { "<root>" } else { path }),
            MixedArray { path, expected, found } =>
                write!(
                    f,
                    "Mixed (and non-convertible) type values in the Lua array \"{}\". Expected \"{}\", found \"{}\".",
                    if path.is_empty() { "<root>" } else { path },
                    expected,
                    found,
                ),
            InvalidKeyType{ path, invalid_type } => write!(f, "Invalid key type ({}) in the Lua table \"{}\" - only strings and numbers are allowed.", invalid_type, if path.is_empty() { "<root>" } else { path }),
            InvalidKeyUTF8{ path, .. } => write!(f, "Invalid string key UTF-8 in Lua table \"{}\".", if path.is_empty() { "<root>" } else { path }),
            EmptyKey{ path } => write!(f, "Empty key string in Lua table \"{}\".", if path.is_empty() { "<root>" } else { path }),
            InvalidArrayIndex(path) => write!(f, "Invalid integer array index in Lua table \"{}\".", if path.is_empty() { "<root>" } else { path }),
            InvalidValueType{ path, invalid_type } => write!(f, "Invalid Lua value type ({}) for any config value at \"{}\".", invalid_type, path),
            InvalidValueUTF8{ path, .. } => write!(f, "Invalid string value UTF-8 at \"{}\".", path),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LuaConfigKeyError {
    /// Lua state mismatch - tried to call [`config`] / [`root`] with the [`Lua context`]
    /// the [`config key`] is not associated with.
    ///
    /// [`config`]: struct.LuaConfigKey.html#method.config
    /// [`root`]: struct.LuaConfigKey.html#method.root
    /// [`Lua context`]: https://docs.rs/rlua/*/rlua/struct.Context.html
    /// [`config key`]: struct.LuaConfigKey.html
    LuaStateMismatch,
}

impl Display for LuaConfigKeyError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use LuaConfigKeyError::*;

        match self {
            LuaStateMismatch => write!(f, "Lua state mismatch."),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LuaArrayGetError {
    /// Index was out of bounds.
    /// Contains the [`array`] length.
    /// [`array`]: struct.LuaArray.html
    IndexOutOfBounds(u32),
    /// Tried to pop an empty [`array`].
    /// [`array`]: struct.LuaArray.html
    ArrayEmpty,
    /// Value is of incorrect [`type`].
    /// Contains the value [`type`].
    /// [`type`]: struct.ValueType.html
    IncorrectValueType(ValueType),
}

impl Display for LuaArrayGetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use LuaArrayGetError::*;

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
pub enum LuaArraySetError {
    /// Index was out of bounds.
    /// Contains the [`array`] length.
    /// [`array`]: struct.LuaArray.html
    IndexOutOfBounds(u32),
    /// Incorrect [`value type`] for the [`array`].
    /// Contains the correct [`array`] [`value type`].
    /// [`value type`]: struct.ValueType.html
    /// [`array`]: struct.LuaArray.html
    InvalidValueType(ValueType),
}

impl Display for LuaArraySetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use LuaArraySetError::*;

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
pub enum LuaTableGetError {
    /// Provided key does not exist in the [`table`].
    /// [`table`]: struct.LuaTable.html
    KeyDoesNotExist,
    /// Value is of incorrect [`type`].
    /// Contains the value [`type`].
    /// [`type`]: struct.ValueType.html
    IncorrectValueType(ValueType),
}

impl Display for LuaTableGetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use LuaTableGetError::*;

        match self {
            KeyDoesNotExist => write!(f, "Provided key does not exist in the table."),
            IncorrectValueType(invalid_type) => {
                write!(f, "Value is of incorrect type ({}).", invalid_type)
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LuaTableGetPathError {
    /// (Part of the) provided path does not exist in the [`table`].
    /// Contains the invalid path.
    /// [`table`]: struct.LuaTable.html
    PathDoesNotExist(Vec<String>),
    /// Value at non-terminating path element is not a [`table`].
    /// Contains the invalid path and the value [`type`].
    /// [`table`]: struct.LuaTable.html
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

impl Display for LuaTableGetPathError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use LuaTableGetPathError::*;

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
pub enum LuaTableSetError {
    /// Tried to remove a non-existant [`value`] from the [`table`].
    /// [`value`]: enum.Value.html
    /// [`table`]: struct.LuaTable.html
    KeyDoesNotExist,
    /// Provided [`table`] key is empty.
    /// [`table`]: struct.LuaTable.html
    EmptyKey,
}

impl Display for LuaTableSetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use LuaTableSetError::*;

        match self {
            KeyDoesNotExist => write!(f, "Tried to remove a non-existant value from the table."),
            EmptyKey => write!(f, "Provided table key is empty."),
        }
    }
}
