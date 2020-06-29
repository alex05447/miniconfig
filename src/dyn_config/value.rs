use {
    crate::{
        util::DisplayLua, ArrayError, ConfigKey, ConfigPath, DynArray, DynTable, GetPathError,
        TableError, Value,
    },
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

impl<'at> DynConfigValueRef<'at> {
    pub(crate) fn get_path<'a, K, P>(self, mut path: P) -> Result<Self, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: Iterator<Item = K>,
    {
        if let Some(key) = path.next() {
            let key = key.borrow();
            match key {
                ConfigKey::Array(index) => match self {
                    Value::Array(array) => {
                        let value = array.get(*index).map_err(|err| match err {
                            ArrayError::IndexOutOfBounds(len) => GetPathError::IndexOutOfBounds {
                                path: ConfigPath::from_key(key.clone()),
                                len,
                            },
                            ArrayError::ArrayEmpty | ArrayError::IncorrectValueType(_) => {
                                unreachable!()
                            }
                        })?;

                        value.get_path(path).map_err(|err| err.push_key(key))
                    }
                    _ => Err(GetPathError::ValueNotAnArray {
                        path: ConfigPath::new(),
                        value_type: self.get_type(),
                    }),
                },
                ConfigKey::Table(ref table_key) => match self {
                    Value::Table(table) => {
                        let value = table.get(table_key).map_err(|err| match err {
                            TableError::EmptyKey => GetPathError::EmptyKey(ConfigPath::new()),
                            TableError::KeyDoesNotExist => {
                                GetPathError::KeyDoesNotExist(ConfigPath::from_key(key.clone()))
                            }
                            TableError::IncorrectValueType(_) => unreachable!(),
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

impl<'at> DynConfigValueMut<'at> {
    pub(crate) fn get_path<'a, K, P>(self, mut path: P) -> Result<Self, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: Iterator<Item = K>,
    {
        if let Some(key) = path.next() {
            let key = key.borrow();
            match key {
                ConfigKey::Array(index) => match self {
                    Value::Array(array) => {
                        let value = array.get_mut(*index).map_err(|err| match err {
                            ArrayError::IndexOutOfBounds(len) => GetPathError::IndexOutOfBounds {
                                path: ConfigPath::from_key(key.clone()),
                                len,
                            },
                            ArrayError::ArrayEmpty | ArrayError::IncorrectValueType(_) => {
                                unreachable!()
                            }
                        })?;

                        value.get_path(path).map_err(|err| err.push_key(key))
                    }
                    _ => Err(GetPathError::ValueNotAnArray {
                        path: ConfigPath::new(),
                        value_type: self.get_type(),
                    }),
                },
                ConfigKey::Table(ref table_key) => match self {
                    Value::Table(table) => {
                        let value = table.get_mut(table_key).map_err(|err| match err {
                            TableError::EmptyKey => GetPathError::EmptyKey(ConfigPath::new()),
                            TableError::KeyDoesNotExist => {
                                GetPathError::KeyDoesNotExist(ConfigPath::from_key(key.clone()))
                            }
                            TableError::IncorrectValueType(_) => unreachable!(),
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
