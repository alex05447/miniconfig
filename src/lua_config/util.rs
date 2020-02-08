use std::fmt::{Formatter, Write};

use crate::{
    value_type_from_u32, value_type_to_u32, write_char, LuaArray, LuaConfigError, LuaString,
    LuaTable, Value, ValueType,
};

use rlua;
use rlua_ext::value_type;

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

pub(super) fn validate_lua_config_table<'lua>(
    lua: rlua::Context<'lua>,
    table: &rlua::Table<'lua>,
    path: &mut String,
) -> Result<(), LuaConfigError> {
    validate_lua_config_table_impl(lua, table, path).map(|_| ())
}

fn validate_lua_config_table_impl<'lua>(
    lua: rlua::Context<'lua>,
    table: &rlua::Table<'lua>,
    path: &mut String,
) -> Result<LuaTableType, LuaConfigError> {
    use LuaConfigError::*;

    // Needed to ensure all keys are the same type.
    let mut table_key_type = None;

    // For arrays, needed to ensure all values are the same (Lua) type.
    let mut table_lua_value_type = None;
    let mut table_value_type = None;

    // Keep track of actual table length.
    let mut len = 0;

    // Keep track of max integer key value.
    // For arrays `max_key` == `len - 1`.
    let mut max_key = 0;

    for pair in table.clone().pairs::<rlua::Value, rlua::Value>() {
        let (key, value) = pair.unwrap();

        // Validate the key and determine if the table might be an array.
        let is_array = match key {
            rlua::Value::String(key) => {
                // Ensure keys are all strings.
                if let Some(table_key_type) = table_key_type {
                    if table_key_type != LuaTableKeyType::String {
                        return Err(MixedKeys(path.clone()));
                    }
                } else {
                    table_key_type.replace(LuaTableKeyType::String);
                }

                // Ensure string keys are valid UTF-8.
                let key = key.to_str().map_err(|error| InvalidKeyUTF8 {
                    path: path.clone(),
                    error,
                })?;

                len += 1;

                if !path.is_empty() {
                    path.push('.');
                }

                path.push_str(key);

                // Definitely not an array.
                false
            }
            rlua::Value::Integer(key) => {
                // Ensure keys are all integers.
                if let Some(table_key_type) = table_key_type {
                    if table_key_type != LuaTableKeyType::Integer {
                        return Err(MixedKeys(path.clone()));
                    }
                } else {
                    table_key_type.replace(LuaTableKeyType::Integer);
                }

                // Ensure keys are in valid range for arrays.
                // NOTE - `1` because of Lua array indexing.
                if key < 1 {
                    return Err(InvalidArrayIndex(path.clone()));
                }

                if key > std::u32::MAX as rlua::Integer {
                    return Err(InvalidArrayIndex(path.clone()));
                }

                max_key = max_key.max(key as u32);

                len += 1;

                if !path.is_empty() {
                    path.push('.');
                }

                write!(path, "{}", key - 1).unwrap();

                // Might be an array.
                true
            }
            // Only string or integer keys allowed.
            key => {
                return Err(InvalidKeyType {
                    path: path.clone(),
                    invalid_type: value_type(&key),
                })
            }
        };

        // For (potential) arrays, ensure all values are the same Lua type
        // (even they are invalid config values).
        if is_array {
            let lua_value_type = value_type(&value);

            if let Some(table_lua_value_type) = table_lua_value_type {
                if !are_lua_types_compatible(table_lua_value_type, lua_value_type) {
                    return Err(MixedArray {
                        path: path.clone(),
                        expected: table_lua_value_type,
                        found: lua_value_type,
                    });
                }
            } else {
                table_lua_value_type.replace(lua_value_type);
            }
        }

        // Validate the value and get its config value type.
        let value_type = match value {
            rlua::Value::Boolean(_) => ValueType::Bool,
            rlua::Value::Integer(_) => ValueType::I64,
            rlua::Value::Number(_) => ValueType::F64,
            rlua::Value::String(value) => {
                // Ensure string values are valid UTF-8.
                if let Err(error) = value.to_str() {
                    return Err(InvalidValueUTF8 {
                        path: path.clone(),
                        error,
                    });
                }

                ValueType::String
            }
            rlua::Value::Table(value) => validate_lua_config_table_impl(lua, &value, path).map(
                |table_type| match table_type {
                    LuaTableType::Array => ValueType::Array,
                    LuaTableType::Table => ValueType::Table,
                },
            )?,
            // Only valid Lua value types allowed.
            invalid_value => {
                return Err(InvalidValueType {
                    path: path.clone(),
                    invalid_type: value_type(&invalid_value),
                })
            }
        };

        // For arrays first value type will determine the array value type.
        if table_value_type.is_none() {
            table_value_type.replace(value_type);
        }

        if let Some(pos) = path.rfind('.') {
            path.split_off(pos);
        } else {
            path.clear();
        }
    }

    let table_type = match table_key_type.unwrap() {
        LuaTableKeyType::String => Ok(LuaTableType::Table),
        LuaTableKeyType::Integer => {
            if max_key != len {
                Err(InvalidArrayIndex(path.clone()))
            } else {
                Ok(LuaTableType::Array)
            }
        }
    }?;

    set_lua_config_table_metatable(lua, &table, table_type, table_value_type, len);

    Ok(table_type)
}

fn set_lua_config_table_metatable<'lua>(
    lua: rlua::Context<'lua>,
    table: &rlua::Table<'lua>,
    table_type: LuaTableType,
    array_value_type: Option<ValueType>,
    len: u32,
) {
    let metatable = lua.create_table().unwrap();

    let table_type: u32 = lua_config_table_type_to_u32(table_type);
    metatable.set("table_type", table_type).unwrap();
    metatable.set("len", len).unwrap();
    metatable
        .set("array_value_type", value_type_to_u32(array_value_type))
        .unwrap();

    table.set_metatable(Some(metatable));
}

pub(super) fn new_table(lua: rlua::Context<'_>) -> rlua::Table<'_> {
    let table = lua.create_table().unwrap();

    set_lua_config_table_metatable(lua, &table, LuaTableType::Table, None, 0);

    table
}

pub(super) fn new_array(lua: rlua::Context<'_>) -> rlua::Table<'_> {
    let table = lua.create_table().unwrap();

    set_lua_config_table_metatable(lua, &table, LuaTableType::Array, None, 0);

    table
}

fn table_type(table: &rlua::Table<'_>) -> LuaTableType {
    lua_config_table_type_from_u32(
        table
            .get_metatable()
            .expect("Lua config table missing a metatable.")
            .get::<_, u32>("table_type")
            .expect("Lua config table metatable missing table type."),
    )
    .expect("Invalid Lua config table type.")
}

pub(super) fn array_value_type(table: &rlua::Table<'_>) -> Option<ValueType> {
    value_type_from_u32(
        table
            .get_metatable()
            .expect("Lua config table missing a metatable.")
            .get::<_, u32>("array_value_type")
            .expect("Lua config table metatable missing array value type."),
    )
}

pub(super) fn set_array_value_type(table: &rlua::Table<'_>, value_type: Option<ValueType>) {
    table
        .get_metatable()
        .expect("Lua config table missing a metatable.")
        .set("array_value_type", value_type_to_u32(value_type))
        .unwrap()
}

pub(super) fn table_len(table: &rlua::Table<'_>) -> u32 {
    table
        .get_metatable()
        .expect("Lua config table missing a metatable.")
        .get::<_, u32>("len")
        .expect("Lua config table metatable missing table length.")
}

pub(super) fn set_table_len(table: &rlua::Table<'_>, len: u32) {
    table
        .get_metatable()
        .expect("Lua config table missing a metatable.")
        .set("len", len)
        .unwrap()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ValueFromLuaValueError {
    KeyDoesNotExist,
    InvalidValueType(rlua_ext::ValueType),
}

pub(super) fn value_from_lua_value(
    value: rlua::Value<'_>,
) -> Result<Value<LuaString<'_>, LuaArray<'_>, LuaTable<'_>>, ValueFromLuaValueError> {
    use ValueFromLuaValueError::*;

    match value {
        rlua::Value::Boolean(value) => Ok(Value::Bool(value)),
        rlua::Value::Number(value) => Ok(Value::F64(value)),
        rlua::Value::Integer(value) => Ok(Value::I64(value)),
        rlua::Value::String(value) => Ok(Value::String(LuaString::new(value))),
        rlua::Value::Table(value) => match table_type(&value) {
            LuaTableType::Array => Ok(Value::Array(LuaArray::from_valid_table(value))),
            LuaTableType::Table => Ok(Value::Table(LuaTable::from_valid_table(value))),
        },
        rlua::Value::Nil => Err(KeyDoesNotExist),
        _ => Err(InvalidValueType(value_type(&value))),
    }
}

pub(crate) trait DisplayLua {
    fn fmt_lua(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result;

    fn do_indent(f: &mut Formatter, indent: u32) -> std::fmt::Result {
        for _ in 0..indent {
            write!(f, "\t")?;
        }

        Ok(())
    }
}

impl<S, A, T> DisplayLua for Value<S, A, T>
where
    S: AsRef<str>,
    A: DisplayLua,
    T: DisplayLua,
{
    fn fmt_lua(&self, f: &mut Formatter, indent: u32) -> std::fmt::Result {
        match self {
            Value::Bool(value) => write!(f, "{}", if *value { "true" } else { "false" }),
            Value::I64(value) => write!(f, "{}", value),
            Value::F64(value) => write!(f, "{}", value),
            Value::String(value) => write_lua_string(f, value.as_ref()),
            Value::Array(value) => value.fmt_lua(f, indent),
            Value::Table(value) => value.fmt_lua(f, indent),
        }
    }
}

/// Writes the `string` to the writer `w`, enclosing it in quotes and escaping special characters
/// ('\\', '\'', '\"', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r').
fn write_lua_string<W: Write>(w: &mut W, string: &str) -> std::fmt::Result {
    write!(w, "\"")?;

    for c in string.chars() {
        write_char(w, c, false)?;
    }

    write!(w, "\"")
}

/// Writes the Lua table `key` to the writer `w`.
/// Writes the string as-si if it's a valid Lua identifier,
/// otherwise encloses it in brackets / quotes, and escapes special characters
/// ('\\', '\'', '\"', '\0', '\a', '\b', '\t', '\n', '\v', '\f', '\r').
pub(crate) fn write_lua_key<W: Write>(w: &mut W, key: &str) -> std::fmt::Result {
    if is_lua_identifier_key(key) {
        write!(w, "{}", key)
    } else {
        write!(w, "[")?;
        write_lua_string(w, key)?;
        write!(w, "]")
    }
}

/// Returns `true` if the char `c` is a valid Lua identifier character.
/// Lua identifiers start with an ASCII letter and may contain ASCII letters, digits and underscores.
fn is_lua_identifier_char(c: char, first: bool) -> bool {
    c.is_ascii_alphabetic() || (!first && ((c == '_') || c.is_ascii_digit()))
}

/// Returns `true` if the non-empty string `key` is a valid Lua identifier.
/// Lua identifiers start with an ASCII letter and may contain ASCII letters, digits and underscores.
fn is_lua_identifier_key(key: &str) -> bool {
    debug_assert!(!key.is_empty());

    let mut chars = key.chars();

    if !is_lua_identifier_char(chars.next().unwrap(), true) {
        return false;
    }

    for c in chars {
        if !is_lua_identifier_char(c, false) {
            return false;
        }
    }

    true
}
