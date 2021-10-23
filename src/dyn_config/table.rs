use {
    crate::{util::*, *},
    std::{
        borrow::Borrow,
        collections::{hash_map::Iter as HashMapIter, HashMap},
        convert::TryInto,
        fmt::{Display, Formatter, Write},
        iter::{IntoIterator, Iterator},
    },
};

/// Represents a mutable hashmap of [`Value`]'s with (non-empty) string keys.
///
/// [`Value`]: enum.Value.html
#[derive(Clone)]
pub struct DynTable(HashMap<NonEmptyString, DynConfigValue>);

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
        self.get_val(key).is_some()
    }

    /// Tries to get an immutable reference to a [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_val<K: AsRef<str>>(&self, key: K) -> Option<DynConfigValueRef<'_>> {
        self.get_impl(key.as_ref().try_into().ok()?)
    }

    /// Tries to get an immutable reference to a [`value`] in the [`table`] with the (non-empty) string `key`,
    /// and convert it to the user-requested type [`convertible`](TryFromValue) from a [`value`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key`,
    /// or if the [`value`] is of incorrect and incompatible type.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get<'t, K: AsRef<str>, V: TryFromValue<&'t str, &'t DynArray, &'t DynTable>>(
        &'t self,
        key: K,
    ) -> Result<V, TableError> {
        V::try_from(
            self.get_val(key)
                .ok_or_else(|| TableError::KeyDoesNotExist)?,
        )
        .map_err(TableError::IncorrectValueType)
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
    pub fn get_val_path<'k, K, P>(&self, path: P) -> Result<DynConfigValueRef<'_>, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        DynConfigValueRef::Table(self)
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get an immutable reference to a [`value`] in the [`table`] at `path`,
    /// and convert it to the user-requested type [`convertible`](TryFromValue) from a [`value`].
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
    pub fn get_path<'t, 'k, K, P, V>(&'t self, path: P) -> Result<V, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
        V: TryFromValue<&'t str, &'t DynArray, &'t DynTable>,
    {
        V::try_from(self.get_val_path(path)?).map_err(GetPathError::IncorrectValueType)
    }

    /// Tries to get a [`bool`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`bool`].
    ///
    /// [`bool`]: enum.Value.html#variant.Bool
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_bool<K: AsRef<str>>(&self, key: K) -> Result<bool, TableError> {
        self.get(key)
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
    pub fn get_bool_path<'k, K, P>(&self, path: P) -> Result<bool, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
    }

    /// Tries to get an [`i64`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`i64`] / [`f64`].
    ///
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    /// [`f64`]: enum.Value.html#variant.F64
    pub fn get_i64<K: AsRef<str>>(&self, key: K) -> Result<i64, TableError> {
        self.get(key)
    }

    /// Tries to get an [`i64`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to an [`i64`] / [`f64`] [`value`].
    ///
    /// [`i64`]: enum.Value.html#variant.I64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: enum.Value.html#variant.Array
    /// [`f64`]: enum.Value.html#variant.F64
    pub fn get_i64_path<'k, K, P>(&self, path: P) -> Result<i64, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
    }

    /// Tries to get an [`f64`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`f64`] / [`i64`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    /// [`i64`]: enum.Value.html#variant.I64
    pub fn get_f64<K: AsRef<str>>(&self, key: K) -> Result<f64, TableError> {
        self.get(key)
    }

    /// Tries to get an [`f64`] [`value`] in the [`table`] at `path`.
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
    /// The last key must correspond to an [`f64`] / [`i64`] [`value`].
    ///
    /// [`f64`]: enum.Value.html#variant.F64
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`config keys`]: enum.ConfigKey.html
    /// [`table keys`]: enum.ConfigKey.html#variant.Table
    /// [`array indices`]: enum.ConfigKey.html#variant.Array
    /// [`array`]: struct.DynArray.html
    /// [`i64`]: enum.Value.html#variant.I64
    pub fn get_f64_path<'k, K, P>(&self, path: P) -> Result<f64, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
    }

    /// Tries to get a [`string`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`string`].
    ///
    /// [`string`]: enum.Value.html#variant.String
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_string<K: AsRef<str>>(&self, key: K) -> Result<&str, TableError> {
        self.get(key)
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
    pub fn get_string_path<'k, K, P>(&self, path: P) -> Result<&str, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
    }

    /// Tries to get an immutable reference to an [`array`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`array`].
    ///
    /// [`array`]: enum.Value.html#variant.Array
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_array<K: AsRef<str>>(&self, key: K) -> Result<&DynArray, TableError> {
        self.get(key)
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
    pub fn get_array_path<'k, K, P>(&self, path: P) -> Result<&DynArray, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
    }

    /// Tries to get an immutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`table`](enum.Value.html#variant.Table).
    ///
    /// [`value`]: type.DynConfigValue.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_table<K: AsRef<str>>(&self, key: K) -> Result<&DynTable, TableError> {
        self.get(key)
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
    pub fn get_table_path<'k, K, P>(&self, path: P) -> Result<&DynTable, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path(path)
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
    /// Returns an [`error`] or if the [`table`] does not contain the `key`.
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
    pub fn get_val_mut<K: AsRef<str>>(&mut self, key: K) -> Option<DynConfigValueMut<'_>> {
        self.get_mut_impl(key.as_ref().try_into().ok()?)
    }

    /// Tries to get a mutable reference to a [`value`] in the [`table`] with the (non-empty) string `key`,
    /// and convert it to the user-requested type [`convertible`](TryFromDynConfigValueMut) from a [`value`].
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key`,
    /// or if the [`value`] is of incorrect and incompatible type.
    ///
    /// [`value`]: type.DynConfigValueRef.html
    /// [`table`]: struct.DynTable.html
    /// [`error`]: enum.TableError.html
    pub fn get_mut<
        'a,
        K: AsRef<str>,
        V: TryFromValue<&'a str, &'a mut DynArray, &'a mut DynTable>,
    >(
        &'a mut self,
        key: K,
    ) -> Result<V, TableError> {
        use TableError::*;
        V::try_from(self.get_val_mut(key).ok_or_else(|| KeyDoesNotExist)?)
            .map_err(IncorrectValueType)
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
    pub fn get_val_path_mut<'k, K, P>(
        &mut self,
        path: P,
    ) -> Result<DynConfigValueMut<'_>, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        DynConfigValueMut::Table(self)
            .get_path(path.into_iter())
            .map_err(GetPathError::reverse)
    }

    /// Tries to get a mutable reference to a [`value`] in the [`table`] at `path`,
    /// and convert it to the user-requested type [`convertible`](TryFromDynConfigValueMut) from a [`value`].
    ///
    /// `path` is an iterator over consecutively nested [`config keys`] - either (non-empty) string [`table keys`],
    /// or (`0`-based) [`array indices`].
    /// All keys except the last one must correspond to a [`table`](enum.Value.html#variant.Table) or an [`array`] value.
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
    /// [`array`]: enum.Value.html#variant.Array
    /// [`type`]: enum.ValueType.html
    /// [`arrays`]: enum.Value.html#variant.Array
    /// [`tables`]: enum.Value.html#variant.Table
    /// [`set`]: #method.set
    pub fn get_path_mut<'a, 'k, K, P, V>(&'a mut self, path: P) -> Result<V, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
        V: TryFromValue<&'a str, &'a mut DynArray, &'a mut DynTable>,
    {
        V::try_from(self.get_val_path_mut(path)?).map_err(GetPathError::IncorrectValueType)
    }

    /// Tries to get a mutable reference to an [`array`] [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not an [`array`].
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
        self.get_mut(key)
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
    pub fn get_array_path_mut<'k, K, P>(&mut self, path: P) -> Result<&mut DynArray, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path_mut(path)
    }

    /// Tries to get a mutable reference to a [`table`](enum.Value.html#variant.Table) [`value`] in the [`table`] with the (non-empty) string `key`.
    ///
    /// Returns an [`error`] if the [`table`] does not contain the `key` or if value is not a [`table`](enum.Value.html#variant.Table).
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
        self.get_mut(key)
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
    pub fn get_table_path_mut<'k, K, P>(&mut self, path: P) -> Result<&mut DynTable, GetPathError>
    where
        K: Borrow<ConfigKey<'k>>,
        P: IntoIterator<Item = K>,
    {
        self.get_path_mut(path)
    }

    /// Inserts or changes the [`value`] at (non-empty) string `key`.
    /// Returns `true` if the [`value`] at `key` already existed and was modified.
    /// Returns `false` if the [`value`] at `key` did not exist and was added.
    ///
    /// [`value`]: type.DynConfigValue.html
    pub fn set<K, V>(&mut self, key: K, value: V) -> bool
    where
        K: AsRef<NonEmptyStr>,
        V: Into<DynConfigValue>,
    {
        self.set_impl(key.as_ref(), value.into())
    }

    /// Tries to remove the [`value`] at (non-empty) string `key`.
    /// Returns the now-removed [`value`] at `key` if it existed,
    /// otherwise returns `None`.
    ///
    /// [`value`]: type.DynConfigValue.html
    pub fn remove<K: AsRef<str>>(&mut self, key: K) -> Option<DynConfigValue> {
        self.remove_impl(key.as_ref().try_into().ok()?)
    }

    fn len_impl(&self) -> u32 {
        self.0.len() as u32
    }

    pub(crate) fn get_impl(&self, key: &NonEmptyStr) -> Option<DynConfigValueRef<'_>> {
        self.0.get(key).map(|val| val.into())
    }

    fn set_impl(&mut self, key: &NonEmptyStr, value: DynConfigValue) -> bool {
        // Modify.
        if let Some(cur_value) = self.0.get_mut(key) {
            *cur_value = value;
            true

        // Add.
        } else {
            self.0.insert(key.into(), value);
            false
        }
    }

    pub(crate) fn remove_impl(&mut self, key: &NonEmptyStr) -> Option<DynConfigValue> {
        self.0.remove(key)
    }

    pub(crate) fn get_mut_impl(&mut self, key: &NonEmptyStr) -> Option<DynConfigValueMut<'_>> {
        self.0.get_mut(key).map(|val| val.into())
    }

    fn fmt_lua_impl<W: Write>(&self, w: &mut W, indent: u32) -> std::fmt::Result {
        writeln!(w, "{{")?;

        // Gather the keys.
        let mut keys: Vec<_> = self.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order.
        keys.sort();

        // Iterate the table using the sorted keys.
        for key in keys.into_iter() {
            <Self as DisplayLua>::do_indent(w, indent + 1)?;

            write_lua_key(w, key)?;
            write!(w, " = ")?;

            // Must succeed - all keys are valid.
            let value = unwrap_unchecked(
                self.get_val(key),
                "failed to get a value from a dyn config table with a valid key",
            );

            let is_array_or_table = matches!(value.get_type(), ValueType::Array | ValueType::Table);

            value.fmt_lua(w, indent + 1)?;

            write!(w, ",")?;

            if is_array_or_table {
                write!(w, " -- {}", key)?;
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
            let l_val = unwrap_unchecked(
                self.get_val(*l),
                "failed to get a value from a dyn config table with a valid key",
            );
            let r_val = unwrap_unchecked(
                self.get_val(*r),
                "failed to get a value from a dyn config table with a valid key",
            );

            let l_is_a_table = l_val.get_type() == ValueType::Table;
            let r_is_a_table = r_val.get_type() == ValueType::Table;

            if !l_is_a_table && r_is_a_table {
                std::cmp::Ordering::Less
            } else if l_is_a_table && !r_is_a_table {
                std::cmp::Ordering::Greater
            } else {
                l.cmp(r)
            }
        });

        let len = self.len() as usize;

        // Iterate the table using the sorted keys.
        for (key_index, key) in keys.into_iter().enumerate() {
            let last = key_index == len - 1;

            // Must succeed - all keys are valid.
            let value = unwrap_unchecked(
                self.get_val(key),
                "failed to get a value from a dyn config table with a valid key",
            );

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
                    let has_non_tables = value
                        .iter()
                        .any(|(_, val)| val.get_type() != ValueType::Table);

                    write_ini_table(
                        w,
                        key,
                        key_index as u32,
                        value,
                        value.len(),
                        has_non_tables,
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
struct DynTableIter<'t>(HashMapIter<'t, NonEmptyString, DynConfigValue>);

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
                unwrap_unchecked(NonEmptyStr::new(key.as_ref()), "empty key"),
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

    use {crate::*, ministr_macro::nestr};

    #[test]
    fn len_empty_clear() {
        let mut table = DynTable::new();

        assert_eq!(table.len(), 0);
        assert!(table.is_empty());

        assert!(!table.set(nestr!("foo"), true));

        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());

        assert!(!table.set(nestr!("bar"), 7));

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
        assert!(!table.contains("bar"));

        assert!(!table.set(nestr!("foo"), true));
        assert!(!table.set(nestr!("bar"), 7));

        assert!(table.contains("foo"));
        assert!(table.contains("bar"));

        assert_eq!(table.remove("bar").unwrap().i64().unwrap(), 7);

        assert!(table.contains("foo"));
        assert!(!table.contains("bar"));

        table.clear();

        assert!(!table.contains("foo"));
        assert!(!table.contains("bar"));
    }

    #[test]
    fn DynTableError_KeyDoesNotExist() {
        let mut table = DynTable::new();

        assert!(table.get_val("").is_none());
        assert!(table.get_val("foo").is_none());
        assert_eq!(
            table.get_bool("").err().unwrap(),
            TableError::KeyDoesNotExist
        );
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

        assert!(table.get_val_mut("foo").is_none(),);
        assert_eq!(
            table.get_table_mut("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );
        assert_eq!(
            table.get_array_mut("foo").err().unwrap(),
            TableError::KeyDoesNotExist
        );

        assert!(table.remove(nestr!("foo")).is_none());

        // But this works.

        assert!(!table.set(nestr!("foo"), true));
        assert_eq!(table.get_val("foo").unwrap().bool().unwrap(), true);
        assert_eq!(table.get_bool("foo").unwrap(), true);
        let val: bool = table.get("foo").unwrap();
        assert_eq!(val, true);
    }

    #[test]
    fn DynTableError_IncorrectValueType() {
        let mut table = DynTable::new();

        assert!(!table.set(nestr!("foo"), true));

        assert_eq!(
            table.get_i64("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        let foo: Result<i64, _> = table.get("foo");
        assert_eq!(
            foo.err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_f64("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        let foo: Result<f64, _> = table.get("foo");
        assert_eq!(
            foo.err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_string("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        let foo: Result<&str, _> = table.get("foo");
        assert_eq!(
            foo.err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        let foo: Result<String, _> = table.get("foo");
        assert_eq!(
            foo.err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_table("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        let foo: Result<&DynTable, _> = table.get("foo");
        assert_eq!(
            foo.err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_array("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        let foo: Result<&DynArray, _> = table.get("foo");
        assert_eq!(
            foo.err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );

        assert_eq!(
            table.get_table_mut("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        let foo: Result<&mut DynTable, _> = table.get_mut("foo");
        assert_eq!(
            foo.err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table.get_array_mut("foo").err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );
        let foo: Result<&mut DynArray, _> = table.get_mut("foo");
        assert_eq!(
            foo.err().unwrap(),
            TableError::IncorrectValueType(ValueType::Bool)
        );

        // But this works.

        assert!(!table.set(nestr!("bar"), 3.14));

        assert_eq!(table.get_i64("bar").unwrap(), 3);
        let bar: i64 = table.get("bar").unwrap();
        assert_eq!(bar, 3);
        assert!(cmp_f64(table.get_f64("bar").unwrap(), 3.14));
        let bar: f64 = table.get("bar").unwrap();
        assert!(cmp_f64(bar, 3.14));

        assert!(!table.set(nestr!("baz"), -7));

        assert_eq!(table.get_i64("baz").unwrap(), -7);
        let baz: i64 = table.get("baz").unwrap();
        assert_eq!(baz, -7);
        assert!(cmp_f64(table.get_f64("baz").unwrap(), -7.0));
        let baz: f64 = table.get("baz").unwrap();
        assert!(cmp_f64(baz, -7.0));

        assert!(!table.set(nestr!("bob"), "bill"));

        assert_eq!(table.get_string("bob").unwrap(), "bill");
        let bob: &str = table.get("bob").unwrap();
        assert_eq!(bob, "bill");
        let bob: String = table.get("bob").unwrap();
        assert_eq!(bob, "bill");

        let mut nested_table = DynTable::new();
        assert!(!nested_table.set(nestr!("anne"), true));
        assert!(!table.set(nestr!("amy"), nested_table));

        let amy = table.get_table("amy").unwrap();
        assert_eq!(amy.len(), 1);
        assert_eq!(amy.get_bool("anne").unwrap(), true);
        let amy: &DynTable = table.get("amy").unwrap();
        assert_eq!(amy.len(), 1);
        assert_eq!(amy.get_bool("anne").unwrap(), true);

        let amy = table.get_table_mut("amy").unwrap();
        assert_eq!(amy.len(), 1);
        assert_eq!(amy.get_bool("anne").unwrap(), true);
        assert!(amy.set(nestr!("anne"), false));
        let amy: &mut DynTable = table.get_mut("amy").unwrap();
        assert_eq!(amy.len(), 1);
        assert_eq!(amy.get_bool("anne").unwrap(), false);

        let mut nested_array = DynArray::new();
        nested_array.push(1.into()).unwrap();
        nested_array.push(2.into()).unwrap();
        nested_array.push(3.into()).unwrap();
        assert!(!table.set(nestr!("arne"), nested_array));

        let arne = table.get_array("arne").unwrap();
        assert_eq!(arne.len(), 3);
        assert_eq!(arne.get_i64(0).unwrap(), 1);
        assert_eq!(arne.get_i64(1).unwrap(), 2);
        assert_eq!(arne.get_i64(2).unwrap(), 3);
        let arne: &DynArray = table.get("arne").unwrap();
        assert_eq!(arne.len(), 3);
        assert_eq!(arne.get_i64(0).unwrap(), 1);
        assert_eq!(arne.get_i64(1).unwrap(), 2);
        assert_eq!(arne.get_i64(2).unwrap(), 3);

        let arne = table.get_array_mut("arne").unwrap();
        assert_eq!(arne.len(), 3);
        assert_eq!(arne.get_i64(0).unwrap(), 1);
        assert_eq!(arne.get_i64(1).unwrap(), 2);
        assert_eq!(arne.get_i64(2).unwrap(), 3);
        arne.push(4.into()).unwrap();
        let arne: &mut DynArray = table.get_mut("arne").unwrap();
        assert_eq!(arne.len(), 4);
        assert_eq!(arne.get_i64(0).unwrap(), 1);
        assert_eq!(arne.get_i64(1).unwrap(), 2);
        assert_eq!(arne.get_i64(2).unwrap(), 3);
        assert_eq!(arne.get_i64(3).unwrap(), 4);
    }

    #[test]
    fn basic() {
        // Create an empty table.
        let mut table = DynTable::new();
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());

        // Add a value.
        assert!(!table.contains("bool"));
        assert!(!table.set(nestr!("bool"), true));
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());
        assert!(table.contains("bool"));
        assert_eq!(table.get_bool("bool").unwrap(), true);

        // Add a couple more.
        assert!(!table.contains("i64"));
        assert!(!table.set(nestr!("i64"), 7));
        assert_eq!(table.len(), 2);
        assert!(!table.is_empty());
        assert!(table.contains("i64"));
        assert_eq!(table.get_i64("i64").unwrap(), 7);

        assert!(!table.contains("string"));
        assert!(!table.set(nestr!("string"), "foo"));
        assert_eq!(table.len(), 3);
        assert!(!table.is_empty());
        assert!(table.contains("string"));
        assert_eq!(table.get_string("string").unwrap(), "foo");

        // Change a value.
        assert!(table.set(nestr!("string"), "bar"));
        assert_eq!(table.len(), 3);
        assert!(!table.is_empty());
        assert!(table.contains("string"));
        assert_eq!(table.get_string("string").unwrap(), "bar");

        // Remove a value.
        assert!(table.remove(nestr!("bool")).is_some());
        assert_eq!(table.len(), 2);
        assert!(!table.is_empty());
        assert!(!table.contains("bool"));

        // Add a nested table with some values.
        let mut nested_table = DynTable::new();
        assert_eq!(nested_table.len(), 0);
        assert!(nested_table.is_empty());

        assert!(!nested_table.contains("nested_bool"));
        assert!(!nested_table.set(nestr!("nested_bool"), false));
        assert!(nested_table.contains("nested_bool"));

        assert!(!nested_table.contains("nested_int"));
        assert!(!nested_table.set(nestr!("nested_int"), -9));
        assert!(nested_table.contains("nested_int"));

        assert_eq!(nested_table.len(), 2);
        assert!(!nested_table.is_empty());

        assert!(!table.contains("table"));
        assert!(!table.set(nestr!("table"), nested_table));
        assert_eq!(table.len(), 3);
        assert!(!table.is_empty());
        assert!(table.contains("table"));

        assert_eq!(
            table
                .get_val_path(&["table".into(), "nested_bool".into()])
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
                .get_val_path(&["table".into(), "nested_int".into()])
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
                .get_val_path(&["table".into(), "nested_int".into()])
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
        assert!(!table.set(nestr!("array"), nested_array));
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
