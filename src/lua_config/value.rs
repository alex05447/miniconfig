use {
    crate::*,
    std::{
        borrow::Borrow,
        fmt::{Display, Formatter},
    },
};

/// Represents a Lua-interned string.
pub struct LuaString<'lua>(rlua::String<'lua>);

impl<'lua> LuaString<'lua> {
    pub(super) fn new(string: rlua::String<'lua>) -> Self {
        Self(string)
    }
}

impl<'lua> AsRef<str> for LuaString<'lua> {
    fn as_ref(&self) -> &str {
        // Guaranteed to be a valid UTF-8 string because 1) we validate the config on construction
        // and 2) only accept valid UTF-8 strings when modifying the config.
        unsafe { std::str::from_utf8_unchecked(self.0.as_bytes()) }
    }
}

/// A [`value`] returned when accessing a Lua [`array`] or [`table`].
///
/// [`value`]: enum.Value.html
/// [`array`]: struct.LuaArray.html
/// [`table`]: struct.LuaTable.html
pub type LuaConfigValue<'lua> = Value<LuaString<'lua>, LuaArray<'lua>, LuaTable<'lua>>;

impl<'s, 'lua> From<&'s str> for Value<&'s str, LuaArray<'lua>, LuaTable<'lua>> {
    fn from(val: &'s str) -> Self {
        Value::String(val)
    }
}

impl<'lua, S> From<LuaTable<'lua>> for Value<S, LuaArray<'lua>, LuaTable<'lua>> {
    fn from(val: LuaTable<'lua>) -> Self {
        Value::Table(val)
    }
}

impl<'lua, S> From<LuaArray<'lua>> for Value<S, LuaArray<'lua>, LuaTable<'lua>> {
    fn from(val: LuaArray<'lua>) -> Self {
        Value::Array(val)
    }
}

impl<'lua> LuaConfigValue<'lua> {
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
                                path: ConfigPath(vec![key.clone()]),
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
                                GetPathError::KeyDoesNotExist(ConfigPath(vec![key.clone()]))
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

impl<'lua> Display for LuaConfigValue<'lua> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}
