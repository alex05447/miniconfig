use {
    crate::{
        lua_config::error::config_key_from_lua_value,
        util::unwrap_unchecked,
        value::*,
        *,
    },
    rlua::Value as LuaValue,
    rlua_ext::value_type,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LuaTableKeyType {
    String,
    Integer,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LuaTableType {
    Array,
    Table,
}

fn lua_config_table_type_to_u32(val: LuaTableType) -> u32 {
    use LuaTableType::*;

    match val {
        Array => 0,
        Table => 1,
    }
}

fn lua_config_table_type_from_u32(val: u32) -> Option<LuaTableType> {
    use LuaTableType::*;

    match val {
        0 => Some(Array),
        1 => Some(Table),
        _ => None,
    }
}

pub(super) fn are_lua_types_compatible(l: rlua_ext::ValueType, r: rlua_ext::ValueType) -> bool {
    use rlua_ext::ValueType::*;

    if l == Nil || r == Nil {
        true
    } else {
        match l {
            Nil => true,
            Boolean => r == Boolean,
            LightUserData => r == LightUserData,
            Integer => r == Integer || r == Number,
            Number => r == Integer || r == Number,
            String => r == String,
            Table => r == Table,
            Function => r == Function,
            Thread => r == Thread,
            UserData => r == UserData,
            Error => r == Error,
        }
    }
}

pub(super) fn validate_lua_config_table<'lua, 'a>(
    lua: rlua::Context<'lua>,
    table: &rlua::Table<'lua>,
) -> Result<(), LuaConfigError<'a>> {
    validate_lua_config_table_impl(lua, table)
        .map(|_| ())
        .map_err(LuaConfigError::reverse)
}

fn validate_lua_config_table_impl<'lua, 'a>(
    lua: rlua::Context<'lua>,
    table: &rlua::Table<'lua>,
) -> Result<LuaTableType, LuaConfigError<'a>> {
    use LuaConfigError::*;

    // Needed to ensure all keys are the same type.
    let mut key_type = None;

    // For arrays, needed to ensure all values are the same (Lua) type.
    let mut array_lua_value_type = None;
    let mut array_value_type = None;

    // Keep track of actual table length.
    let mut len = 0;

    // Keep track of max integer key value.
    // For arrays `max_key` == `len - 1`.
    let mut max_key = 0;

    for pair in table.clone().pairs::<LuaValue, LuaValue>() {
        // Must succeed - no conversion from `LuaValue` is performed.
        let (key, value) = unwrap_unchecked(pair);

        // Validate the key and determine if the table might be an array.
        let is_array = match key {
            LuaValue::String(ref key) => {
                // Ensure keys are all strings.
                if let Some(key_type) = key_type {
                    if key_type != LuaTableKeyType::String {
                        return Err(MixedKeys(ConfigPath::new()));
                    }
                } else {
                    key_type.replace(LuaTableKeyType::String);
                }

                // Ensure string keys are non-empty and are valid UTF-8.
                let key = key.to_str().map_err(|error| InvalidKeyUTF8 {
                    path: ConfigPath::new(),
                    error,
                })?;

                if key.is_empty() {
                    return Err(EmptyKey(ConfigPath::new()));
                }

                len += 1;

                // Definitely not an array.
                false
            }
            LuaValue::Integer(key) => {
                // Ensure keys are all integers.
                if let Some(key_type) = key_type {
                    if key_type != LuaTableKeyType::Integer {
                        return Err(MixedKeys(ConfigPath::new()));
                    }
                } else {
                    key_type.replace(LuaTableKeyType::Integer);
                }

                // Ensure keys are in valid range for arrays.
                // NOTE - `1` because of Lua array indexing.
                if key < 1 {
                    return Err(InvalidArrayIndex(ConfigPath::new()));
                }

                if key > std::u32::MAX as rlua::Integer {
                    return Err(InvalidArrayIndex(ConfigPath::new()));
                }

                let key = key as u32;

                max_key = max_key.max(key);

                len += 1;

                // Might be an array.
                true
            }
            // Only string or integer keys allowed.
            key => {
                return Err(InvalidKeyType {
                    path: ConfigPath::new(),
                    invalid_type: value_type(&key),
                })
            }
        };

        // For (potential) arrays, ensure all values are the same Lua type
        // (even if they are invalid config values).
        if is_array {
            let lua_value_type = value_type(&value);

            if let Some(array_lua_value_type) = array_lua_value_type {
                if !are_lua_types_compatible(array_lua_value_type, lua_value_type) {
                    return Err(MixedArray {
                        path: ConfigPath(vec![config_key_from_lua_value(key)]),
                        expected: array_lua_value_type,
                        found: lua_value_type,
                    });
                }
            } else {
                array_lua_value_type.replace(lua_value_type);
            }
        }

        // Validate the value and get its config value type.
        let value_type = match value {
            LuaValue::Boolean(_) => ValueType::Bool,
            LuaValue::Integer(_) => ValueType::I64,
            LuaValue::Number(_) => ValueType::F64,
            LuaValue::String(value) => {
                // Ensure string values are valid UTF-8.
                if let Err(error) = value.to_str() {
                    return Err(InvalidValueUTF8 {
                        path: ConfigPath(vec![config_key_from_lua_value(key)]),
                        error,
                    });
                }

                ValueType::String
            }
            LuaValue::Table(value) => validate_lua_config_table_impl(lua, &value)
                .map(|table_type| match table_type {
                    LuaTableType::Array => ValueType::Array,
                    LuaTableType::Table => ValueType::Table,
                })
                // Push the current table / array key to the end of the path on error.
                // The path will be reversed at the end.
                .map_err(|err| err.push_key(key))?,
            // Only valid Lua value types allowed.
            invalid_value => {
                return Err(InvalidValueType {
                    path: ConfigPath(vec![config_key_from_lua_value(key)]),
                    invalid_type: value_type(&invalid_value),
                });
            }
        };

        // For arrays first value type will determine the array value type.
        if array_value_type.is_none() {
            array_value_type.replace(value_type);
        }
    }

    let table_type = match key_type {
        // Treat empty tables as tablesm not arrays.
        Some(LuaTableKeyType::String) | None => Ok(LuaTableType::Table),
        Some(LuaTableKeyType::Integer) => {
            if max_key != len {
                Err(InvalidArrayIndex(ConfigPath::new()))
            } else {
                Ok(LuaTableType::Array)
            }
        }
    }?;

    set_lua_config_table_metatable(lua, &table, table_type, array_value_type, len);

    Ok(table_type)
}

const TABLE_TYPE_METATABLE_KEY: &str = "table_type";
const ARRAY_OR_TABLE_LEN_METATABLE_KEY: &str = "len";
const ARRAY_VALUE_TYPE_METATABLE_KEY: &str = "array_value_type";

/// Assigns a metatable to a valid Lua config table,
/// which contains info about its type, length and array value type (for arrays).
fn set_lua_config_table_metatable<'lua>(
    lua: rlua::Context<'lua>,
    table: &rlua::Table<'lua>,
    table_type: LuaTableType,
    array_value_type: Option<ValueType>,
    len: u32,
) {
    let metatable = lua.create_table().expect("failed to create a Lua table");

    let table_type: u32 = lua_config_table_type_to_u32(table_type);
    metatable
        .set(TABLE_TYPE_METATABLE_KEY, table_type)
        .expect("failed to set a metatable value");
    metatable
        .set(ARRAY_OR_TABLE_LEN_METATABLE_KEY, len)
        .expect("failed to set a metatable value");
    metatable
        .set(
            ARRAY_VALUE_TYPE_METATABLE_KEY,
            value_type_to_u32(array_value_type),
        )
        .expect("failed to set a metatable value");

    table.set_metatable(Some(metatable));
}

/// Creates a new valid table for a Lua config table,
/// with a valid metatable containing info about its type and length.
pub(super) fn new_table(lua: rlua::Context<'_>) -> rlua::Table<'_> {
    let table = lua.create_table().expect("failed to create a Lua table");

    set_lua_config_table_metatable(lua, &table, LuaTableType::Table, None, 0);

    table
}

/// Creates a new valid table for a Lua config array,
/// with a valid metatable containing info about its type, length and value type.
pub(super) fn new_array(lua: rlua::Context<'_>) -> rlua::Table<'_> {
    let table = lua.create_table().expect("failed to create a Lua table");

    set_lua_config_table_metatable(lua, &table, LuaTableType::Array, None, 0);

    table
}

/// Reads the config table type from the Lua table's metatable.
/// NOTE - caller guarantees `table` is a valid Lua config table.
fn get_table_type(table: &rlua::Table<'_>) -> LuaTableType {
    // Must succeed - `table` is a valid Lua config table.
    unwrap_unchecked(lua_config_table_type_from_u32(unwrap_unchecked(
        unwrap_unchecked(table.get_metatable()).get::<_, u32>(TABLE_TYPE_METATABLE_KEY),
    )))
}

/// Reads the array value type from the array's Lua table metatable.
/// NOTE - caller guarantees `table` is a valid Lua array table.
pub(super) fn get_array_value_type(table: &rlua::Table<'_>) -> Option<ValueType> {
    // Must succeed - `table` is a valid Lua array table.
    value_type_from_u32(unwrap_unchecked(
        unwrap_unchecked(table.get_metatable()).get::<_, u32>(ARRAY_VALUE_TYPE_METATABLE_KEY),
    ))
}

/// Sets the array value type in the array's Lua table metatable.
/// NOTE - caller guarantees `table` is a valid Lua array table.
pub(super) fn set_array_value_type(table: &rlua::Table<'_>, value_type: Option<ValueType>) {
    // Must succeed - `table` is a valid Lua array table.
    unwrap_unchecked(unwrap_unchecked(table.get_metatable()).set(
        ARRAY_VALUE_TYPE_METATABLE_KEY,
        value_type_to_u32(value_type),
    ))
}

/// Reads the config table length from the Lua table's metatable.
/// NOTE - caller guarantees `table` is a valid Lua config table.
pub(super) fn get_table_len(table: &rlua::Table<'_>) -> u32 {
    // Must succeed - `table` is a valid Lua config table.
    unwrap_unchecked(
        unwrap_unchecked(table.get_metatable()).get::<_, u32>(ARRAY_OR_TABLE_LEN_METATABLE_KEY),
    )
}

/// Sets the config table length in the Lua table's metatable.
/// NOTE - caller guarantees `table` is a valid Lua config table.
pub(super) fn set_table_len(table: &rlua::Table<'_>, len: u32) {
    // Must succeed - `table` is a valid Lua config table.
    unwrap_unchecked(
        unwrap_unchecked(table.get_metatable()).set(ARRAY_OR_TABLE_LEN_METATABLE_KEY, len),
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ValueFromLuaValueError {
    KeyDoesNotExist,
    InvalidValueType(rlua_ext::ValueType),
}

pub(super) fn value_from_lua_value(
    value: LuaValue<'_>,
) -> Result<LuaConfigValue<'_>, ValueFromLuaValueError> {
    use ValueFromLuaValueError::*;

    match value {
        LuaValue::Boolean(value) => Ok(Value::Bool(value)),
        LuaValue::Number(value) => Ok(Value::F64(value)),
        LuaValue::Integer(value) => Ok(Value::I64(value)),
        LuaValue::String(value) => Ok(Value::String(LuaString::new(value))),
        LuaValue::Table(value) => match get_table_type(&value) {
            LuaTableType::Array => Ok(Value::Array(LuaArray::from_valid_table(value))),
            LuaTableType::Table => Ok(Value::Table(LuaTable::from_valid_table(value))),
        },
        LuaValue::Nil => Err(KeyDoesNotExist),
        _ => Err(InvalidValueType(value_type(&value))),
    }
}

pub(super) fn clear_table(table: &rlua::Table<'_>) {
    let pairs: rlua::TablePairs<rlua::Value, rlua::Value> = table.clone().pairs();

    for pair in pairs {
        if let Ok((key, _)) = pair {
            let _ = table.set(key, rlua::Value::Nil);
        }
    }
}

pub(super) fn clear_array(table: &rlua::Table<'_>) {
    let values: rlua::TableSequence<rlua::Value> = table.clone().sequence_values();

    for (index, _) in values.enumerate() {
        let _ = table.set(index + 1, rlua::Value::Nil);
    }
}
