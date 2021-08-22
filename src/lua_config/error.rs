use {
    crate::{util::unwrap_unchecked, *},
    rlua::Value as LuaValue,
    rlua_ext,
    std::{
        error::Error,
        fmt::{Display, Formatter},
    },
};

/// An error returned by [`LuaConfig::from_script`], [`LuaConfigKey::from_script`].
///
/// [`LuaConfig::from_script`]: struct.LuaConfig.html#method.from_script
/// [`LuaConfigKey::from_script`]: struct.LuaConfigKey.html#method.from_script
#[derive(Clone, Debug)]
pub enum LuaConfigError<'a> {
    /// Error loading the Lua config script.
    /// Contains the actual Lua error.
    LuaScriptError(rlua::Error),
    /// Mixed string and integer keys are not allowed in Lua config [`tables`].
    /// Contains the path to the [`table`], or an empty path if the mixed keys are in the root [`table`].
    ///
    /// [`tables`]: struct.LuaTable.html
    MixedKeys(ConfigPath<'a>),
    /// Mixed (and incompatible) type values are not allowed in Lua config [`arrays`].
    ///
    /// [`arrays`]: struct.LuaArray.html
    MixedArray {
        /// Path to the invalid array element.
        path: ConfigPath<'a>,
        /// Expected Lua value [`type`] (as determined by the first value in the array).
        ///
        /// [`type`]: enum.ValueType.html
        expected: rlua_ext::ValueType,
        /// Found Lua value [`type`].
        ///
        /// [`type`]: enum.ValueType.html
        found: rlua_ext::ValueType,
    },
    /// Only string and number keys are allowed in Lua config [`tables`].
    ///
    /// [`tables`]: struct.LuaTable.html
    InvalidKeyType {
        /// Path to the [`table`], or an empty path for the root [`table`].
        ///
        /// [`table`]: struct.LuaTable.html
        path: ConfigPath<'a>,
        /// Invalid key Lua value type.
        invalid_type: rlua_ext::ValueType,
    },
    /// Invalid [`table`] string key UTF-8.
    ///
    /// [`table`]: struct.LuaTable.html
    InvalidKeyUTF8 {
        /// Path to the [`table`] containing the invalid key, or an empty path for the root [`table`].
        ///
        /// [`table`]: struct.LuaTable.html
        path: ConfigPath<'a>,
        /// UTF-8 parse error.
        error: rlua::Error,
    },
    /// Empty key strings are not allowed in Lua config [`tables`].
    /// Contains the path to the [`table`] containing the empty key, or an empty path for the root [`table`].
    ///
    /// [`tables`]: struct.LuaTable.html
    EmptyKey(ConfigPath<'a>),
    /// Invalid index in Lua config [`array`].
    /// Contains the path to the invalid [`array`].
    ///
    /// [`array`]: struct.LuaArray.html
    InvalidArrayIndex(ConfigPath<'a>),
    /// Invalid Lua value type for a Lua config [`value`].
    ///
    /// [`value`]: type.LuaConfigValue.html
    InvalidValueType {
        /// Path to the invalid config [`value`].
        ///
        /// [`value`]: type.LuaConfigValue.html
        path: ConfigPath<'a>,
        /// Invalid Lua value type.
        invalid_type: rlua_ext::ValueType,
    },
    /// Invalid string [`value`] UTF-8.
    ///
    /// [`value`]: type.LuaConfigValue.html
    InvalidValueUTF8 {
        /// Path to the invalid string [`value`].
        ///
        /// [`value`]: type.LuaConfigValue.html
        path: ConfigPath<'a>,
        /// UTF-8 parse error.
        error: rlua::Error,
    },
}

impl<'a> LuaConfigError<'a> {
    /// Pushes the table key / array index to the back of the path if the error has one.
    pub(crate) fn push_key(mut self, key: LuaValue<'_>) -> Self {
        use LuaConfigError::*;

        let key = config_key_from_lua_value(key);

        match &mut self {
            MixedKeys(path) => path.0.push(key),
            MixedArray { path, .. } => path.0.push(key),
            InvalidKeyType { path, .. } => path.0.push(key),
            InvalidKeyUTF8 { path, .. } => path.0.push(key),
            EmptyKey(path) => path.0.push(key),
            InvalidArrayIndex(path) => path.0.push(key),
            InvalidValueType { path, .. } => path.0.push(key),
            InvalidValueUTF8 { path, .. } => path.0.push(key),

            LuaScriptError(_) => {}
        };

        self
    }

    /// Reverses the path if the error has one.
    /// Must do this because path elements were pushed to the back of the `Vec`
    /// when unwinding the stack on error.
    /// (Alternatively we could always push path elements to the front, but that would constantly shuffle the `Vec`).
    pub(crate) fn reverse(mut self) -> Self {
        use LuaConfigError::*;

        match &mut self {
            MixedKeys(path) => path.0.reverse(),
            MixedArray { path, .. } => path.0.reverse(),
            InvalidKeyType { path, .. } => path.0.reverse(),
            InvalidKeyUTF8 { path, .. } => path.0.reverse(),
            EmptyKey(path) => path.0.reverse(),
            InvalidArrayIndex(path) => path.0.reverse(),
            InvalidValueType { path, .. } => path.0.reverse(),
            InvalidValueUTF8 { path, .. } => path.0.reverse(),

            LuaScriptError(_) => {}
        };

        self
    }
}

impl<'a> Error for LuaConfigError<'a> {}

impl<'a> Display for LuaConfigError<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use LuaConfigError::*;

        match self {
            LuaScriptError(err) => write!(f, "error loading the Lua config script: {}", err),
            MixedKeys(path) => write!(f, "mixed string and integer keys are not allowed in Lua config table {}", path),
            MixedArray { path, expected, found } =>
                write!(
                    f,
                    "mixed (and incompatible) type values are not allowed in Lua config array {}: expected: \"{}\", found: \"{}\"",
                    path,
                    expected,
                    found,
                ),
            InvalidKeyType{ path, invalid_type } => write!(f, "only string and number keys are allowed in Lua config table {}; found: \"{}\"", path, invalid_type),
            InvalidKeyUTF8{ path, error } => write!(f, "invalid string key UTF-8 in Lua config table {}: {}", path, error),
            EmptyKey(path) => write!(f, "empty key strings are not allowed in Lua config table {}", path),
            InvalidArrayIndex(path) => write!(f, "invalid index in Lua config array {}", path),
            InvalidValueType{ path, invalid_type } => write!(f, "invalid Lua value type (\"{}\") for a Lua config value at {}", invalid_type, path),
            InvalidValueUTF8{ path, error } => write!(f, "invalid string value UTF-8 at {}: {}", path, error),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LuaConfigKeyError {
    /// Lua state mismatch - tried to call [`config()`] / [`root()`] with the [`Lua context`]
    /// the [`config key`] is not associated with.
    ///
    /// [`config()`]: struct.LuaConfigKey.html#method.config
    /// [`root()`]: struct.LuaConfigKey.html#method.root
    /// [`Lua context`]: https://docs.rs/rlua/*/rlua/struct.Context.html
    /// [`config key`]: struct.LuaConfigKey.html
    LuaStateMismatch,
}

impl Error for LuaConfigKeyError {}

impl Display for LuaConfigKeyError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use LuaConfigKeyError::*;

        match self {
            LuaStateMismatch => "lua state mismatch".fmt(f),
        }
    }
}

// NOTE: the caller ensures the `key` is either a (valid UTF-8) string or an integer.
pub(super) fn config_key_from_lua_value<'a>(key: LuaValue<'_>) -> ConfigKey<'a> {
    match key {
        LuaValue::String(key) => ConfigKey::Table(
            // Must succeed - keys are valid strings or integers.
            unwrap_unchecked(key.to_str()).to_owned().into(),
        ),
        LuaValue::Integer(key) => {
            debug_assert!(key > 0);
            ConfigKey::Array((key - 1) as u32)
        }
        _ => panic!("expected a string or integer Lua table / array key"),
    }
}
