use {
    crate::{util::*, *},
    std::{
        borrow::Borrow,
        collections::{hash_map::Iter as HashMapIter, HashMap},
        fmt::{Display, Formatter, Write},
        iter::{IntoIterator, Iterator},
    },
};

/// Represents a mutable hashmap of [`Value`]'s with (non-empty) string keys.
///
/// [`Value`]: enum.Value.html
#[derive(Clone)]
pub struct DynTable(HashMap<String, DynConfigValue>);

impl DynTable {
    /// Creates a new empty [`table`].
    ///
    /// [`table`]: struct.DynTable.html
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Returns the number of entries in the [`table`].
    ///
    /// [`table`]: struct.DynTable.html
    pub fn len(&self) -> u32 {
        self.len_impl()
    }

    /// Returns `true` if the [`table`] is empty.
    ///
    /// [`table`]: struct.DynTable.html
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the [`table`].
    ///
    /// [`table`]: struct.DynTable.html
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Returns `true` if the [`table`] contains a [`value`] with the (non-empty) string `key`.
    /// Returns `false` if the `key` is empty.
    ///
    /// [`table`]: struct.DynTable.html
    /// [`value`]: type.DynConfigValueRef.html
    pub fn contains<K: AsRef<str>>(&self, key: K) -> bool {
        use TableError::*;

        match self.get(key) {
            Ok(_) => true,
            Err(err) => match err {
                EmptyKey | KeyDoesNotExist => false,
                // This error is not returned by `get()`.
                IncorrectValueType(_) => debug_unreachable!("unexpected error"),
            },
        }
    }

    /// Tries to get an immutable reference to a [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty or if the [`table`] does not contain the `key`.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get<K: AsRef<str>>(&self, key: K) -> Result<DynConfigValueRef<'_>, TableError> {
        let key = NonEmptyStr::new(key.as_ref()).ok_or_else(|| TableError::EmptyKey)?;
        self.get_impl(&key)
    }

    /// Tries to get an immutable reference to a [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns the [`table`] itself if the `path` is empty.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    /// [`type`]: enum.ValueType.html
    pub fn get_path<'a, K, P>(&self, path: P) -> Result<DynConfigValueRef<'_>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        DynConfigValueRef::Table(self)
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_bool<K: AsRef<str>>(&self, key: K) -> Result<bool, TableError> {
        let val = self.get(key)?;
        val.bool()
            .ok_or_else(|| TableError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to a [`bool`] [`value`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_bool_path<'a, K, P>(&self, path: P) -> Result<bool, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        val.bool()
            .ok_or_else(|| GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not an [`i64`] / [`f64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_i64<K: AsRef<str>>(&self, key: K) -> Result<i64, TableError> {
        let val = self.get(key)?;
        val.i64()
            .ok_or_else(|| TableError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`i64`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to an [`i64`] / [`f64`] [`value`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_i64_path<'a, K, P>(&self, path: P) -> Result<i64, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        val.i64()
            .ok_or_else(|| GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not an [`f64`] / [`i64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_f64<K: AsRef<str>>(&self, key: K) -> Result<f64, TableError> {
        let val = self.get(key)?;
        val.f64()
            .ok_or_else(|| TableError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get an [`f64`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to an [`i64`] / [`f64`] [`value`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.DynArray.html
    pub fn get_f64_path<'a, K, P>(&self, path: P) -> Result<f64, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        val.f64()
            .ok_or_else(|| GetPathError::IncorrectValueType(val.get_type()))
    }

    /// Tries to get a [`string`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_string<K: AsRef<str>>(&self, key: K) -> Result<&str, TableError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| TableError::IncorrectValueType(val_type))
    }

    /// Tries to get a [`string`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to a [`string`] [`value`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_string_path<'a, K, P>(&self, path: P) -> Result<&str, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.string()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not an [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_array<K: AsRef<str>>(&self, key: K) -> Result<&DynArray, TableError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| TableError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to an [`array`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to an [`array`] [`value`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    pub fn get_array_path<'a, K, P>(&self, path: P) -> Result<&DynArray, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_table<K: AsRef<str>>(&self, key: K) -> Result<&DynTable, TableError> {
        let val = self.get(key)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| TableError::IncorrectValueType(val_type))
    }

    /// Tries to get an immutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to a [`table`](enum.Value.html#variant.Table) [`value`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    pub fn get_table_path<'a, K, P>(&self, path: P) -> Result<&DynTable, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Returns an iterator over (`key`, [`value`]) pairs of the [`table`], in unspecified order.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    pub fn iter(&self) -> impl Iterator<Item = (&NonEmptyStr, DynConfigValueRef<'_>)> {
        DynTableIter(self.0.iter())
    }

    /// Tries to get a mutable reference to a [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty or if the [`table`] does not contain the `key`.
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`table`].
    ///
    /// [`value`]: type.DynConfigValueMut.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_mut<K: AsRef<str>>(&mut self, key: K) -> Result<DynConfigValueMut<'_>, TableError> {
        let key = NonEmptyStr::new(key.as_ref()).ok_or_else(|| TableError::EmptyKey)?;
        self.get_mut_impl(key)
    }

    /// Tries to get a mutable reference to a [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`] value.
    /// The last key may correspond to a value of any [`type`].
    ///
    /// Returns the [`table`] itself if the `path` is empty.
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`table`].
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.DynArray.html
    /// [`type`]: enum.ValueType.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_mut_path<'a, K, P>(
        &mut self,
        path: P,
    ) -> Result<DynConfigValueMut<'_>, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        DynConfigValueMut::Table(self)
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a mutable reference to an [`array`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not an [`array`].
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`table`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_array_mut<K: AsRef<str>>(&mut self, key: K) -> Result<&mut DynArray, TableError> {
        let val = self.get_mut(key)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| TableError::IncorrectValueType(val_type))
    }

    /// Tries to get a mutable reference to an [`array`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to an [`array`] [`value`].
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`table`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`array`]: enum.Value.html#variant.Array
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_array_mut_path<'a, K, P>(
        &mut self,
        path: P,
    ) -> Result<&mut DynArray, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_mut_path(path)?;
        let val_type = val.get_type();
        val.array()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// Tries to get a mutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the `key` is empty, if the [`table`] does not contain the `key` or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// NOTE: mutable reference extends to [`arrays`] and [`tables`], not other value types.
    /// Use [`set`] to mutate other value types in the [`table`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_table_mut<K: AsRef<str>>(&mut self, key: K) -> Result<&mut DynTable, TableError> {
        let val = self.get_mut(key)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| TableError::IncorrectValueType(val_type))
    }

    /// Tries to get a mutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`] or an [`array`] value.
    /// The last key must correspond to a [`table`](enum.Value.html#variant.Table) [`value`].
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`array`]: struct.DynArray.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    pub fn get_table_mut_path<'a, K, P>(
        &mut self,
        path: P,
    ) -> Result<&mut DynTable, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: IntoIterator<Item = K>,
    {
        let val = self.get_mut_path(path)?;
        let val_type = val.get_type();
        val.table()
            .ok_or_else(|| GetPathError::IncorrectValueType(val_type))
    }

    /// If [`value`] is `Some`, inserts or changes the value at (non-empty) string `key`.
    /// Returns `true` if the value at `key` already existed and was modified.
    /// Returns `false` if the value at `key` did not exist and was added.
    ///
    /// If [`value`] is `None`, tries to remove the value at `key`.
    /// Returns an [`error`] if the value at `key` did not exist,
    /// otherwise returns `true`.
    ///
    /// Returns an [`error`] if the the `key` is empty.
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`error`]: enum.TableError.html
    pub fn set<K, V>(&mut self, key: K, value: V) -> Result<bool, TableError>
    where
        K: AsRef<str>,
        V: Into<Option<DynConfigValue>>,
    {
        let key = NonEmptyStr::new(key.as_ref()).ok_or_else(|| TableError::EmptyKey)?;
        self.set_impl(&key, value.into())
    }

    fn len_impl(&self) -> u32 {
        self.0.len() as u32
    }

    pub(crate) fn get_impl(&self, key: &NonEmptyStr) -> Result<DynConfigValueRef<'_>, TableError> {
        use TableError::*;

        if let Some(value) = self.0.get(key.as_ref()) {
            Ok(value.into())
        } else {
            Err(KeyDoesNotExist)
        }
    }

    /// When adding or modifying a value (`value` is `Some`), returns `true` if the value at `key` already existed and was modified,
    /// `false` if the value did not exist and was added.
    /// When removing an existing value (`value` is `None`), returns `true` if the value existed and was removed,
    /// or a `KeyDoesNotExist` error if it did not exist.
    pub(crate) fn set_impl(
        &mut self,
        key: &NonEmptyStr,
        value: Option<DynConfigValue>,
    ) -> Result<bool, TableError> {
        use TableError::*;

        let key = key.as_ref();

        // Add or modify a value - always succeeds.
        if let Some(value) = value {
            // Modify.
            if let Some(cur_value) = self.0.get_mut(key) {
                *cur_value = value;
                Ok(true)

            // Add.
            } else {
                self.0.insert(key.into(), value);
                Ok(false)
            }

        // (Try to) remove a value.
        // Succeeds if key existed.
        } else {
            match self.0.remove(key) {
                None => Err(KeyDoesNotExist),
                Some(_) => Ok(true),
            }
        }
    }

    #[cfg(feature = "ini")]
    pub(crate) fn remove(&mut self, key: &NonEmptyStr) -> Option<DynConfigValue> {
        self.0.remove(key.as_ref())
    }

    fn get_mut_impl(&mut self, key: &NonEmptyStr) -> Result<DynConfigValueMut<'_>, TableError> {
        use TableError::*;

        if let Some(value) = self.0.get_mut(key.as_ref()) {
            Ok(value.into())
        } else {
            Err(KeyDoesNotExist)
        }
    }

    fn fmt_lua_impl<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        writeln!(w, "{{")?;

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order.
        keys.sort_by(|l, r| l.as_ref().cmp(r.as_ref()));

        // Iterate the table using the sorted keys.
        for key in keys.into_iter() {
            <Self as DisplayLua>::do_indent(w, indent + 1)?;

            write_lua_key(w, key)?;
            write!(w, " = ")?;

            // Must succeed - all keys are valid.
            let value = unwrap_unchecked(self.get(key));

            let is_array_or_table = matches!(value.get_type(), ValueType::Array | ValueType::Table);

            value.fmt_lua(w, indent + 1)?;

            write!(w, ",")?;

            if is_array_or_table {
                write!(w, " -- {}", key.as_ref())?;
            }

            writeln!(w)?;
        }

        <Self as DisplayLua>::do_indent(w, indent)?;
        write!(w, "}}")?;

        Ok(())
    }

    #[cfg(feature = "ini")]
    fn fmt_ini_impl<W: Write>(
        &self,
        w: &mut W,
        level: u32,
        _array: bool,
        path: &mut IniPath,
        options: ToIniStringOptions,
    ) -> Result<(), ToIniStringError> {
        debug_assert!(options.nested_sections() || level < 2);

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order, non-tables first.
        keys.sort_by(|l, r| {
            // Must succeed - all keys are valid.
            let l_val = unwrap_unchecked(self.get(*l));
            let r_val = unwrap_unchecked(self.get(*r));

            let l_is_a_table = l_val.get_type() == ValueType::Table;
            let r_is_a_table = r_val.get_type() == ValueType::Table;

            if !l_is_a_table && r_is_a_table {
                std::cmp::Ordering::Less
            } else if l_is_a_table && !r_is_a_table {
                std::cmp::Ordering::Greater
            } else {
                l.as_ref().cmp(r.as_ref())
            }
        });

        let len = self.len() as usize;

        // Iterate the table using the sorted keys.
        for (key_index, key) in keys.into_iter().enumerate() {
            let last = key_index == len - 1;

            // Must succeed - all keys are valid.
            let value = unwrap_unchecked(self.get(key));

            match value {
                Value::Array(value) => {
                    write_ini_array(
                        w,
                        key,
                        value.iter(),
                        value.len() as usize,
                        last,
                        level,
                        path,
                        options,
                    )?;
                }
                Value::Table(value) => {
                    write_ini_table(
                        w,
                        key,
                        key_index as u32,
                        value,
                        value.len(),
                        last,
                        level,
                        path,
                        options,
                    )?;
                }
                value => {
                    write_ini_value(w, key, &value, last, level, false, path, options)?;
                }
            }
        }

        Ok(())
    }
}

/// Iterator over (`key`, [`value`]) tuples of the [`table`], in unspecified order.
///
/// [`value`]: type.DynConfigValue.html
/// [`table`]: struct.DynTable.html
struct DynTableIter<'t>(HashMapIter<'t, String, DynConfigValue>);

impl<'t> Iterator for DynTableIter<'t> {
    type Item = (&'t NonEmptyStr, DynConfigValueRef<'t>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((key, value)) = self.0.next() {
            let value = match value {
                Value::Bool(value) => Value::Bool(*value),
                Value::I64(value) => Value::I64(*value),
                Value::F64(value) => Value::F64(*value),
                Value::String(value) => Value::String(value.as_str()),
                Value::Array(value) => Value::Array(value),
                Value::Table(value) => Value::Table(value),
            };

            // Safe to call - we validated the key.
            Some((
                unwrap_unchecked_msg(NonEmptyStr::new(key.as_ref()), "empty key"),
                value,
            ))
        } else {
            None
        }
    }
}

impl DisplayLua for DynTable {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(w, indent)
    }
}

impl<'t> DisplayLua for &'t DynTable {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(w, indent)
    }
}

impl<'t> DisplayLua for &'t mut DynTable {
    fn fmt_lua<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        self.fmt_lua_impl(w, indent)
    }
}

#[cfg(feature = "ini")]
impl DisplayIni for DynTable {
    fn fmt_ini<W: Write>(
        &self,
        w: &mut W,
        level: u32,
        array: bool,
        path: &mut IniPath,
        options: ToIniStringOptions,
    ) -> Result<(), ToIniStringError> {
        self.fmt_ini_impl(w, level, array, path, options)
    }
}

#[cfg(feature = "ini")]
impl<'t> DisplayIni for &'t DynTable {
    fn fmt_ini<W: Write>(
        &self,
        w: &mut W,
        level: u32,
        array: bool,
        path: &mut IniPath,
        options: ToIniStringOptions,
    ) -> Result<(), ToIniStringError> {
        self.fmt_ini_impl(w, level, array, path, options)
    }
}

impl Display for DynTable {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua_impl(f, 0)
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use crate::*;

    #[test]
    fn len_empty_clear() {
        let mut table = DynTable::new();

        assert_eq!(table.len(), 0);
        assert!(table.is_empty());

        table.set("foo", Some(true.into())).unwrap();

        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());

        table.set("bar", Some(7.into())).unwrap();

        assert_eq!(table.len(), 2);
        assert!(!table.is_empty());

        table.clear();

        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
    }

    #[test]
    fn contains() {
        let mut table = DynTable::new();

        assert!(!table.contains("foo"));

        table.set("foo", Some(true.into())).unwrap();

        assert!(table.contains("foo"));

        table.clear();

        assert!(!table.contains("foo"));
    }

    #[test]
    fn DynTableError_EmptyKey() {
        let mut table = DynTable::new();

        assert_eq!(table.get("").err().unwrap(), TableError::EmptyKey);
        assert_eq!(table.get_bool("").err().unwrap(), TableError::EmptyKey);
        assert_eq!(table.get_i64("").err().unwrap(), TableError::EmptyKey);
        assert_eq!(table.get_f64("").err().unwrap(), TableError::EmptyKey);
        assert_eq!(table.get_string("").err().unwrap(), TableError::EmptyKey);
        assert_eq!(table.get_table("").err().unwrap(), TableError::EmptyKey);
        assert_eq!(table.get_array("").err().unwrap(), TableError::EmptyKey);

        assert_eq!(table.get_mut("").err().unwrap(), TableError::EmptyKey);
        assert_eq!(table.get_table_mut("").err().unwrap(), TableError::EmptyKey);
        assert_eq!(table.get_array_mut("").err().unwrap(), TableError::EmptyKey);

        assert_eq!(
            table.set("", Some(true.into())).err().unwrap(),
            TableError::EmptyKey
        );
    }

    #[test]
    fn DynTableError_KeyDoesNotExist() {
        let mut table = DynTable::new();

        assert_eq!(table.get("foo").err().unwrap(), TableError::KeyDoesNotExist);
        assert_eq!(
            table.get_bool("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );
        assert_eq!(
            table.get_i64("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );
        assert_eq!(
            table.get_f64("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );
        assert_eq!(
            table.get_string("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );
        assert_eq!(
            table.get_table("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );
        assert_eq!(
            table.get_array("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );

        assert_eq!(
            table.get_mut("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );
        assert_eq!(
            table.get_table_mut("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );
        assert_eq!(
            table.get_array_mut("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );

        assert_eq!(
            table.set("foo", None).err().unwrap(),
            TableError::KeyDoesNotExist
        );

        // But this works.

        table.set("foo", Some(true.into())).unwrap();

        assert_eq!(table.get("foo").unwrap().bool().unwrap(), true);
    }

    #[test]
    fn DynTableError_IncorrectValueType() {
        let mut table = DynTable::new();

        table.set("foo", Some(true.into())).unwrap();

        assert_eq!(
            table.get_i64("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_f64("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_string("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_table("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_array("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );

        assert_eq!(
            table.get_table_mut("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_array_mut("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );

        // But this works.

        table.set("bar", Some(3.14.into())).unwrap();

        assert_eq!(table.get_i64("bar").unwrap(), 3);
        assert!(cmp_f64(table.get_f64("bar").unwrap(), 3.14));

        table.set("baz", Some((-7).into())).unwrap();

        assert_eq!(table.get_i64("baz").unwrap(), -7);
        assert!(cmp_f64(table.get_f64("baz").unwrap(), -7.0));
    }

    #[test]
    fn basic() {
        // Create an empty table.
        let mut table = DynTable::new();
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());

        // Add a value.
        assert!(!table.contains("bool"));
        table.set("bool", Some(true.into())).unwrap();
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());
        assert!(table.contains("bool"));
        assert_eq!(table.get_bool("bool").unwrap(), true);

        // Add a couple more.
        assert!(!table.contains("i64"));
        table.set("i64", Some(7.into())).unwrap();
        assert_eq!(table.len(), 2);
        assert!(!table.is_empty());
        assert!(table.contains("i64"));
        assert_eq!(table.get_i64("i64").unwrap(), 7);

        assert!(!table.contains("string"));
        table.set("string", Some("foo".into())).unwrap();
        assert_eq!(table.len(), 3);
        assert!(!table.is_empty());
        assert!(table.contains("string"));
        assert_eq!(table.get_string("string").unwrap(), "foo");

        // Change a value.
        table.set("string", Some("bar".into())).unwrap();
        assert_eq!(table.len(), 3);
        assert!(!table.is_empty());
        assert!(table.contains("string"));
        assert_eq!(table.get_string("string").unwrap(), "bar");

        // Remove a value.
        table.set("bool", None).unwrap();
        assert_eq!(table.len(), 2);
        assert!(!table.is_empty());
        assert!(!table.contains("bool"));

        // Add a nested table with some values.
        let mut nested_table = DynTable::new();
        assert_eq!(nested_table.len(), 0);
        assert!(nested_table.is_empty());

        assert!(!nested_table.contains("nested_bool"));
        nested_table.set("nested_bool", Some(false.into())).unwrap();
        assert!(nested_table.contains("nested_bool"));

        assert!(!nested_table.contains("nested_int"));
        nested_table.set("nested_int", Some((-9).into())).unwrap();
        assert!(nested_table.contains("nested_int"));

        assert_eq!(nested_table.len(), 2);
        assert!(!nested_table.is_empty());

        assert!(!table.contains("table"));
        table.set("table", Some(nested_table.into())).unwrap();
        assert_eq!(table.len(), 3);
        assert!(!table.is_empty());
        assert!(table.contains("table"));

        assert_eq!(
            table
                .get_path(&["table".into(), "nested_bool".into()])
                .unwrap()
                .bool()
                .unwrap(),
            false
        );
        assert_eq!(
            table
                .get_bool_path(&["table".into(), "nested_bool".into()])
                .unwrap(),
            false
        );
        assert_eq!(
            table
                .get_path(&["table".into(), "nested_int".into()])
                .unwrap()
                .i64()
                .unwrap(),
            -9
        );
        assert_eq!(
            table
                .get_i64_path(&["table".into(), "nested_int".into()])
                .unwrap(),
            -9
        );
        assert!(cmp_f64(
            table
                .get_path(&["table".into(), "nested_int".into()])
                .unwrap()
                .f64()
                .unwrap(),
            -9.0
        ));
        assert!(cmp_f64(
            table
                .get_f64_path(&["table".into(), "nested_int".into()])
                .unwrap(),
            -9.0
        ));

        // Add a nested array with some values.
        let mut nested_array = DynArray::new();
        assert_eq!(nested_array.len(), 0);
        assert!(nested_array.is_empty());

        nested_array.push(3.14.into()).unwrap();
        nested_array.push(42.0.into()).unwrap();
        nested_array.push((-17.235).into()).unwrap();
        assert_eq!(nested_array.len(), 3);
        assert!(!nested_array.is_empty());

        assert!(!table.contains("array"));
        table.set("array", Value::Array(nested_array)).unwrap();
        assert_eq!(table.len(), 4);
        assert!(!table.is_empty());
        assert!(table.contains("array"));

        assert_eq!(table.get_i64_path(&["array".into(), 0.into()]).unwrap(), 3);
        assert!(cmp_f64(
            table.get_f64_path(&["array".into(), 0.into()]).unwrap(),
            3.14
        ));

        assert_eq!(table.get_i64_path(&["array".into(), 1.into()]).unwrap(), 42);
        assert!(cmp_f64(
            table.get_f64_path(&["array".into(), 1.into()]).unwrap(),
            42.0
        ));

        assert_eq!(
            table.get_i64_path(&["array".into(), 2.into()]).unwrap(),
            -17
        );
        assert!(cmp_f64(
            table.get_f64_path(&["array".into(), 2.into()]).unwrap(),
            -17.235
        ));

        // Iterate the table.
        for (key, value) in table.iter() {
            match key.as_ref() {
                "i64" => assert_eq!(value.i64().unwrap(), 7),
                "string" => assert_eq!(value.string().unwrap(), "bar"),
                "table" => {
                    // Iterate the nested table.
                    let nested_table = value.table().unwrap();

                    for (key, value) in nested_table.iter() {
                        match key.as_ref() {
                            "nested_bool" => assert_eq!(value.bool().unwrap(), false),
                            "nested_int" => assert_eq!(value.i64().unwrap(), -9),
                            _ => panic!("Invalid key."),
                        }
                    }
                }
                "array" => {
                    // Iterate the nested array.
                    let nested_array = value.array().unwrap();

                    for (index, value) in nested_array.iter().enumerate() {
                        match index {
                            0 => assert!(cmp_f64(value.f64().unwrap(), 3.14)),
                            1 => assert!(cmp_f64(value.f64().unwrap(), 42.0)),
                            2 => assert!(cmp_f64(value.f64().unwrap(), -17.235)),
                            _ => panic!("Invalid index."),
                        }
                    }
                }
                _ => panic!("Invalid key."),
            }
        }
    }
}
