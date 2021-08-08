use {
    crate::{
        util::{unwrap_unchecked, DisplayLua},
        DynTable,
    },
    std::fmt::{Display, Formatter, Write},
};

#[cfg(feature = "bin")]
use crate::{BinConfigWriter, BinConfigWriterError, DynConfigValueRef};

#[cfg(feature = "ini")]
use crate::{
    ArrayError, ConfigKey, DisplayIni, IniConfig, IniError, IniParser, IniPath, IniValue,
    NonEmptyStr, ToIniStringError, ToIniStringOptions,
};

use crate::debug_unreachable;
#[cfg(any(feature = "bin", feature = "ini"))]
use crate::{DynArray, Value};

/// Represents a mutable config with a root hashmap [`table`].
///
/// [`table`]: struct.DynTable.html
pub struct DynConfig(DynTable);

impl DynConfig {
    /// Creates a new [`config`] with an empty root [`table`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`table`]: struct.DynTable.html
    pub fn new() -> Self {
        Self(DynTable::new())
    }

    /// Returns the immutable reference to the root [`table`] of the [`config`].
    ///
    /// [`table`]: struct.DynTable.html
    /// [`config`]: struct.DynConfig.html
    pub fn root(&self) -> &DynTable {
        &self.0
    }

    /// Returns the mutable reference to the root [`table`] of the [`config`].
    ///
    /// [`table`]: struct.DynTable.html
    /// [`config`]: struct.DynConfig.html
    //pub fn root_mut(&mut self) -> DynTableMut<'_> {
    pub fn root_mut(&mut self) -> &mut DynTable {
        &mut self.0
    }

    /// Tries to serialize this [`config`] to a Lua script string.
    ///
    /// NOTE: you may also call `to_string` via the [`config`]'s `Display` implementation.
    ///
    /// [`config`]: struct.DynConfig.html
    pub fn to_lua_string(&self) -> Result<String, std::fmt::Error> {
        let mut result = String::new();

        write!(&mut result, "{}", self)?;

        result.shrink_to_fit();

        Ok(result)
    }

    /// Tries to serialize this [`config`] to a [`binary config`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`binary config`]: struct.BinConfig.html
    #[cfg(feature = "bin")]
    pub fn to_bin_config(&self) -> Result<Box<[u8]>, BinConfigWriterError> {
        use BinConfigWriterError::*;

        let root = self.root();

        // The root table is empty - nothing to do.
        if root.len() == 0 {
            return Err(EmptyRootTable);
        }

        let mut writer = BinConfigWriter::new(root.len())?;

        table_to_bin_config(root, &mut writer)?;

        writer.finish()
    }

    /// Creates a new [`config`] from the [`.ini parser`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`.ini parser`]: struct.IniParser.html
    #[cfg(feature = "ini")]
    pub fn from_ini(parser: IniParser) -> Result<Self, IniError> {
        let mut config = DynConfig::new();

        parser.parse(&mut config)?;

        Ok(config)
    }

    /// Tries to serialize this [`config`] to an `.ini` string.
    ///
    /// [`config`]: struct.DynConfig.html
    #[cfg(feature = "ini")]
    pub fn to_ini_string(&self) -> Result<String, ToIniStringError> {
        self.to_ini_string_opts(Default::default())
    }

    /// Tries to serialize this [`config`] to an `.ini` string using provided [`options`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`options`]: struct.ToIniStringOptions.html
    #[cfg(feature = "ini")]
    pub fn to_ini_string_opts(
        &self,
        options: ToIniStringOptions,
    ) -> Result<String, ToIniStringError> {
        let mut result = String::new();
        let mut path = IniPath::new();

        self.root()
            .fmt_ini(&mut result, 0, false, &mut path, options)?;

        result.shrink_to_fit();

        Ok(result)
    }
}

impl Display for DynConfig {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.root().fmt_lua(f, 0)
    }
}

#[cfg(feature = "ini")]
impl IniConfig for DynConfig {
    fn contains_section<'s, P: Iterator<Item = NonEmptyStr<'s>>>(
        &self,
        section: NonEmptyStr<'s>,
        path: P,
    ) -> bool {
        let table = match self.root().get_table_path(path.map(ConfigKey::from)) {
            Ok(table) => table,
            Err(_) => {
                debug_assert!(false, "invalid section path");
                return false;
            }
        };

        table.get_table(section).is_ok()
    }

    fn add_section<'s, P: Iterator<Item = NonEmptyStr<'s>>>(
        &mut self,
        section: NonEmptyStr<'s>,
        path: P,
        overwrite: bool,
    ) {
        let table = match self
            .root_mut()
            .get_table_mut_path(path.map(ConfigKey::from))
        {
            Ok(table) => table,
            Err(_) => {
                debug_assert!(false, "invalid section path");
                return;
            }
        };

        if let Ok(already_existed) = table.set_impl(section, Some(Value::Table(DynTable::new()))) {
            debug_assert!(
                overwrite == already_existed,
                "overwrite flag mismatch when adding a section"
            );
        } else {
            debug_unreachable()
        }
    }

    fn contains_key<'s, P: Iterator<Item = NonEmptyStr<'s>>>(
        &self,
        path: P,
        key: NonEmptyStr<'s>,
    ) -> bool {
        let table = match self.root().get_table_path(path.map(ConfigKey::from)) {
            Ok(table) => table,
            Err(_) => {
                debug_assert!(false, "invalid key path");
                return false;
            }
        };

        table.get_impl(key).is_ok()
    }

    fn add_value<'s, P: Iterator<Item = NonEmptyStr<'s>>>(
        &mut self,
        path: P,
        key: NonEmptyStr<'s>,
        value: IniValue<&str>,
        overwrite: bool,
    ) {
        let table = match self
            .root_mut()
            .get_table_mut_path(path.map(ConfigKey::from))
        {
            Ok(table) => table,
            Err(_) => {
                debug_assert!(false, "invalid value path");
                return;
            }
        };

        if let Ok(already_existed) = match value {
            IniValue::Bool(value) => table.set(key, Value::Bool(value)),
            IniValue::I64(value) => table.set(key, Value::I64(value)),
            IniValue::F64(value) => table.set(key, Value::F64(value)),
            IniValue::String(value) => table.set(key, Value::String(value.into())),
        } {
            debug_assert!(
                overwrite == already_existed,
                "overwrite flag mismatch when adding a value"
            );
        } else {
            debug_unreachable()
        }
    }

    fn add_array<'s, P: Iterator<Item = NonEmptyStr<'s>>>(
        &mut self,
        path: P,
        key: NonEmptyStr<'s>,
        array: Vec<IniValue<String>>,
        overwrite: bool,
    ) {
        let table = match self
            .root_mut()
            .get_table_mut_path(path.map(ConfigKey::from))
        {
            Ok(table) => table,
            Err(_) => {
                debug_assert!(false, "invalid array path");
                return;
            }
        };

        let mut dyn_array = DynArray::new();

        for value in array.into_iter() {
            match dyn_array.push(match value {
                IniValue::Bool(value) => Value::Bool(value),
                IniValue::I64(value) => Value::I64(value),
                IniValue::F64(value) => Value::F64(value),
                IniValue::String(value) => Value::String(value),
            }) {
                Ok(_) => {}
                Err(err) => match err {
                    ArrayError::IncorrectValueType(_) => {
                        debug_assert!(false, "mixed type array values")
                    }
                    ArrayError::IndexOutOfBounds(_) | ArrayError::ArrayEmpty => debug_unreachable(),
                },
            }
        }

        if let Ok(already_existed) = table.set_impl(key, Some(Value::Array(dyn_array))) {
            debug_assert!(
                overwrite == already_existed,
                "overwrite flag mismatch when adding an array"
            );
        } else {
            debug_unreachable()
        }
    }
}

#[cfg(feature = "bin")]
/// Writes the dyn table recursively to the binary config writer.
fn table_to_bin_config(
    table: &DynTable,
    writer: &mut BinConfigWriter,
) -> Result<(), BinConfigWriterError> {
    // Gather the keys.
    let mut keys: Vec<_> = table.iter().map(|(key, _)| key).collect();

    // Sort the keys in alphabetical order.
    keys.sort_by(|l, r| l.as_ref().cmp(r.as_ref()));

    // Iterate the table using the sorted keys.
    for key in keys.into_iter() {
        // Must succeed - all keys are valid.
        let value = unwrap_unchecked(table.get(key));

        value_to_bin_config(Some(key.as_ref()), value, writer)?;
    }

    Ok(())
}

#[cfg(feature = "bin")]
/// Writes the dyn array recursively to the binary config writer.
fn array_to_bin_config(
    array: &DynArray,
    writer: &mut BinConfigWriter,
) -> Result<(), BinConfigWriterError> {
    // Iterate the array in order.
    for value in array.iter() {
        value_to_bin_config(None, value, writer)?;
    }

    Ok(())
}

#[cfg(feature = "bin")]
/// Writes the dyn config value with `key` recursively to the binary config writer.
fn value_to_bin_config(
    key: Option<&str>,
    value: DynConfigValueRef<'_>,
    writer: &mut BinConfigWriter,
) -> Result<(), BinConfigWriterError> {
    use Value::*;

    match value {
        Bool(value) => {
            writer.bool(key, value)?;
        }
        I64(value) => {
            writer.i64(key, value)?;
        }
        F64(value) => {
            writer.f64(key, value)?;
        }
        String(value) => {
            writer.string(key, value)?;
        }
        Array(value) => {
            writer.array(key, value.len())?;
            array_to_bin_config(value, writer)?;
            writer.end()?;
        }
        Table(value) => {
            writer.table(key, value.len())?;
            table_to_bin_config(value, writer)?;
            writer.end()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use crate::*;

    #[test]
    fn GetPathError_EmptyKey() {
        let mut table = DynTable::new();

        assert_eq!(
            table.get_path(&["".into()]).err().unwrap(),
            GetPathError::EmptyKey(ConfigPath::new())
        );

        let mut other_table = DynTable::new();
        other_table.set("bar", Some(true.into())).unwrap();

        table.set("foo", Some(other_table.into())).unwrap();

        assert_eq!(
            table.get_path(&["foo".into(), "".into()]).err().unwrap(),
            GetPathError::EmptyKey(ConfigPath(vec!["foo".into()]))
        );

        // But this works.

        assert_eq!(
            table.get_bool_path(&["foo".into(), "bar".into()]).unwrap(),
            true,
        );
    }

    #[test]
    fn GetPathError_PathDoesNotExist() {
        let mut table = DynTable::new();

        let mut foo = DynTable::new();
        let mut bar = DynArray::new();
        let mut baz = DynTable::new();

        baz.set("bob", Some(true.into())).unwrap();
        bar.push(baz.into()).unwrap();
        foo.set("bar", Some(bar.into())).unwrap();
        table.set("foo", Some(foo.into())).unwrap();

        assert_eq!(
            table.get_path(&["foo".into(), "baz".into()]).err().unwrap(),
            GetPathError::KeyDoesNotExist(ConfigPath(vec!["foo".into(), "baz".into()]))
        );

        assert_eq!(
            table
                .get_path(&["foo".into(), "bar".into(), 0.into(), "bill".into()])
                .err()
                .unwrap(),
            GetPathError::KeyDoesNotExist(ConfigPath(vec![
                "foo".into(),
                "bar".into(),
                0.into(),
                "bill".into()
            ]))
        );

        // But this works.

        assert_eq!(
            table
                .get_bool_path(&["foo".into(), "bar".into(), 0.into(), "bob".into()])
                .unwrap(),
            true
        );
    }

    #[test]
    fn GetPathError_IndexOutOfBounds() {
        let mut table = DynTable::new();

        let mut array = DynArray::new();
        array.push(true.into()).unwrap();

        table.set("array", Some(array.into())).unwrap();

        assert_eq!(
            table.get_path(&["array".into(), 1.into()]).err().unwrap(),
            GetPathError::IndexOutOfBounds {
                path: ConfigPath(vec!["array".into(), 1.into()]),
                len: 1
            }
        );

        // But this works.

        assert_eq!(
            table.get_bool_path(&["array".into(), 0.into()]).unwrap(),
            true
        );
    }

    #[test]
    fn GetPathError_ValueNotAnArray() {
        let mut table = DynTable::new();

        let mut other_table = DynTable::new();
        other_table.set("array", Some(true.into())).unwrap();

        table.set("table", Some(other_table.into())).unwrap();

        assert_eq!(
            table
                .get_path(&["table".into(), "array".into(), 1.into()])
                .err()
                .unwrap(),
            GetPathError::ValueNotAnArray {
                path: ConfigPath(vec!["table".into(), "array".into()]),
                value_type: ValueType::Bool
            }
        );

        // But this works.

        assert_eq!(
            table
                .get_bool_path(&["table".into(), "array".into()])
                .unwrap(),
            true,
        );
    }

    #[test]
    fn GetPathError_ValueNotATable() {
        let mut table = DynTable::new();

        let mut array = DynArray::new();
        array.push(true.into()).unwrap();

        table.set("array", Some(array.into())).unwrap();

        assert_eq!(
            table
                .get_path(&["array".into(), 0.into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::ValueNotATable {
                path: ConfigPath(vec!["array".into(), 0.into()]),
                value_type: ValueType::Bool
            }
        );

        // But this works.

        assert_eq!(
            table.get_bool_path(&["array".into(), 0.into()]).unwrap(),
            true,
        );
    }

    #[test]
    fn GetPathError_IncorrectValueType() {
        let mut table = DynTable::new();

        let mut other_table = DynTable::new();
        other_table.set("foo", Some(true.into())).unwrap();
        other_table.set("bar", Some(3.14.into())).unwrap();

        table.set("table", Some(other_table.into())).unwrap();

        assert_eq!(
            table
                .get_i64_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table
                .get_f64_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table
                .get_string_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table
                .get_array_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            table
                .get_table_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );

        // But this works.

        assert_eq!(
            table
                .get_bool_path(&["table".into(), "foo".into()])
                .unwrap(),
            true
        );

        assert_eq!(
            table.get_i64_path(&["table".into(), "bar".into()]).unwrap(),
            3
        );
        assert!(cmp_f64(
            table.get_f64_path(&["table".into(), "bar".into()]).unwrap(),
            3.14
        ));
    }

    // "array_value = { 54, 12, 78.9 } -- array_value
    // bool_value = true
    // float_value = 3.14
    // int_value = 7
    // string_value = \"foo\"
    // table_value = {
    // \tbar = 2020,
    // \tbaz = \"hello\",
    // \tfoo = false,
    // } -- table_value";

    #[cfg(feature = "bin")]
    #[test]
    fn to_bin_config() {
        let mut config = DynConfig::new();

        let root = config.root_mut();

        let mut array_value = DynArray::new();

        array_value.push(Value::I64(54)).unwrap();
        array_value.push(Value::I64(12)).unwrap();
        array_value.push(Value::F64(78.9)).unwrap();

        root.set("array_value", Value::Array(array_value)).unwrap();

        root.set("bool_value", Value::Bool(true)).unwrap();

        root.set("float_value", Value::F64(3.14)).unwrap();

        root.set("int_value", Value::I64(7)).unwrap();

        root.set("string_value", Value::String("foo".into()))
            .unwrap();

        let mut table_value = DynTable::new();

        table_value.set("bar", Value::I64(2020)).unwrap();
        table_value
            .set("baz", Value::String("hello".into()))
            .unwrap();
        table_value.set("foo", Value::Bool(false)).unwrap();

        root.set("table_value", Value::Table(table_value)).unwrap();

        // Serialize to binary config.
        let data = config.to_bin_config().unwrap();

        // Load the binary config.
        let config = BinConfig::new(data).unwrap();

        let array_value = config.root().get_array("array_value".into()).unwrap();

        assert_eq!(array_value.len(), 3);
        assert_eq!(array_value.get_i64(0).unwrap(), 54);
        assert!(cmp_f64(array_value.get_f64(0).unwrap(), 54.0));
        assert_eq!(array_value.get_i64(1).unwrap(), 12);
        assert!(cmp_f64(array_value.get_f64(1).unwrap(), 12.0));
        assert_eq!(array_value.get_i64(2).unwrap(), 78);
        assert!(cmp_f64(array_value.get_f64(2).unwrap(), 78.9));

        assert_eq!(config.root().get_bool("bool_value".into()).unwrap(), true);

        assert!(cmp_f64(
            config.root().get_f64("float_value".into()).unwrap(),
            3.14
        ));

        assert_eq!(config.root().get_i64("int_value".into()).unwrap(), 7);

        assert_eq!(
            config.root().get_string("string_value".into()).unwrap(),
            "foo"
        );

        let table_value = config.root().get_table("table_value".into()).unwrap();

        assert_eq!(table_value.len(), 3);
        assert_eq!(table_value.get_i64("bar".into()).unwrap(), 2020);
        assert!(cmp_f64(table_value.get_f64("bar".into()).unwrap(), 2020.0));
        assert_eq!(table_value.get_string("baz".into()).unwrap(), "hello");
        assert_eq!(table_value.get_bool("foo".into()).unwrap(), false);
    }

    #[cfg(feature = "ini")]
    #[test]
    fn to_ini_string() {
        let ini = r#"array = ["foo", "bar", "baz"]
bool = true
float = 3.14
int = 7
string = "foo"

[other_section]
other_bool = true
other_float = 3.14
other_int = 7
other_string = "foo"

[section]
bool = false
float = 7.62
int = 9
string = "bar""#;

        let mut config = DynConfig::new();

        let mut array = DynArray::new();

        array.push(Value::String("foo".into())).unwrap();
        array.push(Value::String("bar".into())).unwrap();
        array.push(Value::String("baz".into())).unwrap();

        config.root_mut().set("array", Value::Array(array)).unwrap();

        config.root_mut().set("bool", Value::Bool(true)).unwrap();
        config.root_mut().set("float", Value::F64(3.14)).unwrap();
        config.root_mut().set("int", Value::I64(7)).unwrap();
        config
            .root_mut()
            .set("string", Value::String("foo".into()))
            .unwrap();

        let mut other_section = DynTable::new();

        other_section.set("other_bool", Value::Bool(true)).unwrap();
        other_section.set("other_float", Value::F64(3.14)).unwrap();
        other_section.set("other_int", Value::I64(7)).unwrap();
        other_section
            .set("other_string", Value::String("foo".into()))
            .unwrap();

        config
            .root_mut()
            .set("other_section", Value::Table(other_section))
            .unwrap();

        let mut section = DynTable::new();

        section.set("bool", Value::Bool(false)).unwrap();
        section.set("float", Value::F64(7.62)).unwrap();
        section.set("int", Value::I64(9)).unwrap();
        section.set("string", Value::String("bar".into())).unwrap();

        config
            .root_mut()
            .set("section", Value::Table(section))
            .unwrap();

        let string = config
            .to_ini_string_opts(ToIniStringOptions {
                arrays: true,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(string, ini);
    }
}
