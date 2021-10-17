use {
    crate::{util::DisplayLua, *},
    std::{
        borrow::Borrow,
        fmt::{Display, Formatter},
    },
};

/// A [`value`] returned when accessing a dynamic [`array`] or [`table`].
///
/// [`value`]: enum.Value.html
/// [`array`]: struct.DynArray.html
/// [`table`]: struct.DynTable.html
pub type DynConfigValue = Value<String, DynArray, DynTable>;

impl From<String> for DynConfigValue {
    fn from(val: String) -> Self {
        Value::String(val)
    }
}

impl<'a> From<&'a str> for DynConfigValue {
    fn from(val: &'a str) -> Self {
        Value::String(val.into())
    }
}

impl From<DynArray> for DynConfigValue {
    fn from(val: DynArray) -> Self {
        Value::Array(val)
    }
}

impl From<DynTable> for DynConfigValue {
    fn from(val: DynTable) -> Self {
        Value::Table(val)
    }
}

impl Display for DynConfigValue {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}

/// A [`value`] returned when accessing a dynamic [`array`] or [`table`] by reference.
///
/// [`value`]: enum.Value.html
/// [`array`]: struct.DynArray.html
/// [`table`]: struct.DynTable.html
pub type DynConfigValueRef<'at> = Value<&'at str, &'at DynArray, &'at DynTable>;

impl<'at> From<&'at Value<String, DynArray, DynTable>> for DynConfigValueRef<'at> {
    fn from(value: &'at Value<String, DynArray, DynTable>) -> Self {
        match value {
            Value::Bool(value) => Value::Bool(*value),
            Value::I64(value) => Value::I64(*value),
            Value::F64(value) => Value::F64(*value),
            Value::String(value) => Value::String(value.as_str()),
            Value::Array(value) => Value::Array(value),
            Value::Table(value) => Value::Table(value),
        }
    }
}

impl<'at> DynConfigValueRef<'at> {
    pub(crate) fn get_path<'k, K, P>(self, mut path: P) -> Result<Self, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: Iterator<Item = K>,
    {
        if let Some(key) = path.next() {
            let key = key.borrow();
            match key {
                ConfigKey::Array(index) => match self {
                    Value::Array(array) => {
                        let value = array.get_val(*index).map_err(|err| match err {
                            ArrayError::IndexOutOfBounds(len) => GetPathError::IndexOutOfBounds {
                                path: vec![(*index).into()].into(),
                                len,
                            },
                            ArrayError::ArrayEmpty | ArrayError::IncorrectValueType(_) => {
                                debug_unreachable!("`get()` does not return `ArrayEmpty` or `IncorrectValueType(_)`")
                            }
                        })?;

                        value.get_path(path).map_err(|err| err.push_index(*index))
                    }
                    _ => Err(GetPathError::ValueNotAnArray {
                        path: ConfigPath::new(),
                        value_type: self.get_type(),
                    }),
                },
                ConfigKey::Table(ref table_key) => match self {
                    Value::Table(table) => {
                        let key = NonEmptyStr::new(table_key.as_str())
                            .ok_or_else(|| GetPathError::EmptyKey(ConfigPath::new()))?;
                        let value = table.get_impl(key).map_err(|err| match err {
                            TableError::KeyDoesNotExist => {
                                GetPathError::KeyDoesNotExist(vec![key.into()].into())
                            }
                            TableError::IncorrectValueType(_) => debug_unreachable!(
                                "`get_impl()` does not return `IncorrectValueType(_)`"
                            ),
                            TableError::EmptyKey => {
                                debug_unreachable!("`get_impl()` does not return `EmptyKey`")
                            }
                        })?;

                        value.get_path(path).map_err(|err| err.push_key(key))
                    }
                    _ => Err(GetPathError::ValueNotATable {
                        path: ConfigPath::new(),
                        value_type: self.get_type(),
                    }),
                },
            }
        } else {
            Ok(self)
        }
    }
}

impl<'a> Display for DynConfigValueRef<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}

/// A [`value`] returned when accessing a dynamic [`array`] or [`table`] by mutable reference.
///
/// [`value`]: enum.Value.html
/// [`array`]: struct.DynArray.html
/// [`table`]: struct.DynTable.html
pub type DynConfigValueMut<'at> = Value<&'at str, &'at mut DynArray, &'at mut DynTable>;

impl<'at> From<&'at mut Value<String, DynArray, DynTable>> for DynConfigValueMut<'at> {
    fn from(value: &'at mut Value<String, DynArray, DynTable>) -> Self {
        match value {
            Value::Bool(value) => Value::Bool(*value),
            Value::I64(value) => Value::I64(*value),
            Value::F64(value) => Value::F64(*value),
            Value::String(value) => Value::String(value.as_str()),
            Value::Array(value) => Value::Array(value),
            Value::Table(value) => Value::Table(value),
        }
    }
}

impl<'at> DynConfigValueMut<'at> {
    pub(crate) fn get_path<'k, K, P>(self, mut path: P) -> Result<Self, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: Iterator<Item = K>,
    {
        if let Some(key) = path.next() {
            let key = key.borrow();
            match key {
                ConfigKey::Array(index) => match self {
                    Value::Array(array) => {
                        let value = array.get_val_mut(*index).map_err(|err| match err {
                            ArrayError::IndexOutOfBounds(len) => GetPathError::IndexOutOfBounds {
                                path: vec![(*index).into()].into(),
                                len,
                            },
                            ArrayError::ArrayEmpty | ArrayError::IncorrectValueType(_) => {
                                debug_unreachable!("`get_mut()` does not return `ArrayEmpty` or `IncorrectValueType(_)`")
                            }
                        })?;

                        value.get_path(path).map_err(|err| err.push_index(*index))
                    }
                    _ => Err(GetPathError::ValueNotAnArray {
                        path: ConfigPath::new(),
                        value_type: self.get_type(),
                    }),
                },
                ConfigKey::Table(ref table_key) => match self {
                    Value::Table(table) => {
                        let key = NonEmptyStr::new(table_key.as_str())
                            .ok_or_else(|| GetPathError::EmptyKey(ConfigPath::new()))?;
                        let value = table.get_mut_impl(key).map_err(|err| match err {
                            TableError::KeyDoesNotExist => {
                                GetPathError::KeyDoesNotExist(vec![key.into()].into())
                            }
                            TableError::IncorrectValueType(_) => debug_unreachable!(
                                "`get_mut_impl()` does not return `IncorrectValueType(_)`"
                            ),
                            TableError::EmptyKey => {
                                debug_unreachable!("`get_mut_impl()` does not return `EmptyKey`")
                            }
                        })?;

                        value.get_path(path).map_err(|err| err.push_key(key))
                    }
                    _ => Err(GetPathError::ValueNotATable {
                        path: ConfigPath::new(),
                        value_type: self.get_type(),
                    }),
                },
            }
        } else {
            Ok(self)
        }
    }
}

impl<'a> Display for DynConfigValueMut<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}

impl<'a> TryFromValue<&'a str, &'a DynArray, &'a DynTable> for &'a str {
    fn try_from(val: Value<&'a str, &'a DynArray, &'a DynTable>) -> Result<Self, ValueType> {
        let val_type = val.get_type();
        val.string().ok_or_else(|| val_type)
    }
}

impl<'a> TryFromValue<&'a str, &'a DynArray, &'a DynTable> for String {
    fn try_from(val: Value<&'a str, &'a DynArray, &'a DynTable>) -> Result<Self, ValueType> {
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| val_type)
            .map(|string| string.into())
    }
}

impl<'a> TryFromValue<&'a str, &'a DynArray, &'a DynTable> for &'a DynArray {
    fn try_from(val: Value<&'a str, &'a DynArray, &'a DynTable>) -> Result<Self, ValueType> {
        let val_type = val.get_type();
        val.array().ok_or_else(|| val_type)
    }
}

impl<'a> TryFromValue<&'a str, &'a DynArray, &'a DynTable> for &'a DynTable {
    fn try_from(val: Value<&'a str, &'a DynArray, &'a DynTable>) -> Result<Self, ValueType> {
        let val_type = val.get_type();
        val.table().ok_or_else(|| val_type)
    }
}

impl<'a> TryFromValue<&'a str, &'a mut DynArray, &'a mut DynTable> for &'a str {
    fn try_from(
        val: Value<&'a str, &'a mut DynArray, &'a mut DynTable>,
    ) -> Result<Self, ValueType> {
        let val_type = val.get_type();
        val.string().ok_or_else(|| val_type)
    }
}

impl<'a> TryFromValue<&'a str, &'a mut DynArray, &'a mut DynTable> for String {
    fn try_from(
        val: Value<&'a str, &'a mut DynArray, &'a mut DynTable>,
    ) -> Result<Self, ValueType> {
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| val_type)
            .map(|string| string.into())
    }
}

impl<'a> TryFromValue<&'a str, &'a mut DynArray, &'a mut DynTable> for &'a mut DynArray {
    fn try_from(
        val: Value<&'a str, &'a mut DynArray, &'a mut DynTable>,
    ) -> Result<Self, ValueType> {
        let val_type = val.get_type();
        val.array().ok_or_else(|| val_type)
    }
}

impl<'a> TryFromValue<&'a str, &'a mut DynArray, &'a mut DynTable> for &'a mut DynTable {
    fn try_from(
        val: Value<&'a str, &'a mut DynArray, &'a mut DynTable>,
    ) -> Result<Self, ValueType> {
        let val_type = val.get_type();
        val.table().ok_or_else(|| val_type)
    }
}
