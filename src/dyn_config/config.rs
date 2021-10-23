use {
    crate::{util::DisplayLua, *},
    std::{
        fmt::{Display, Formatter, Write},
        num::NonZeroU32,
    },
};

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

        self.fmt_lua(&mut result)?;

        result.shrink_to_fit();

        Ok(result)
    }

    /// Tries to serialize this [`config`] to a Lua script string to the writer `w`.
    ///
    /// NOTE: you may also use the [`config`]'s `Display` implementation.
    ///
    /// [`config`]: struct.DynConfig.html
    pub fn fmt_lua<W: Write>(&self, w: &mut W) -> Result<(), std::fmt::Error> {
        self.root().fmt_lua(w, 0)
    }

    /// Tries to serialize this [`config`] to a [`binary config`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`binary config`]: struct.BinConfig.html
    #[cfg(feature = "bin")]
    pub fn to_bin_config(&self) -> Result<Box<[u8]>, BinConfigWriterError> {
        use BinConfigWriterError::*;

        let root = self.root();

        if let Some(root_len) = NonZeroU32::new(root.len()) {
            let mut writer = BinConfigWriter::new(root_len)?;

            table_to_bin_config(root, &mut writer)?;

            writer.finish()

        // The root table is empty - nothing to do.
        } else {
            Err(EmptyRootTable)
        }
    }

    /// Creates a new [`config`] from the [`.ini parser`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`.ini parser`]: struct.IniParser.html
    #[cfg(feature = "ini")]
    pub fn from_ini(parser: IniParser) -> Result<Self, IniError> {
        let mut config = DynConfigIniConfig::new();
        parser.parse(&mut config)?;
        Ok(config.into_inner())
    }

    /// Tries to serialize this [`config`] to an `.ini` string.
    ///
    /// [`config`]: struct.DynConfig.html
    #[cfg(feature = "ini")]
    pub fn to_ini_string(&self) -> Result<String, ToIniStringError> {
        self.to_ini_string_opts(Default::default())
    }

    /// Tries to serialize this [`config`] to an `.ini` string to the writer `w` using default [`options`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`options`]: struct.ToIniStringOptions.html
    #[cfg(feature = "ini")]
    pub fn fmt_ini<W: Write>(&self, w: &mut W) -> Result<(), ToIniStringError> {
        self.fmt_ini_opts(Default::default(), w)
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

        self.fmt_ini_opts(options, &mut result)?;

        result.shrink_to_fit();

        Ok(result)
    }

    /// Tries to serialize this [`config`] to an `.ini` string to the writer `w` using provided [`options`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`options`]: struct.ToIniStringOptions.html
    #[cfg(feature = "ini")]
    pub fn fmt_ini_opts<W: std::fmt::Write>(
        &self,
        options: ToIniStringOptions,
        w: &mut W,
    ) -> Result<(), ToIniStringError> {
        let mut path = IniPath::new();

        self.root().fmt_ini(w, 0, false, &mut path, options)
    }
}

impl Display for DynConfig {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.root().fmt_lua(f, 0)
    }
}

/// Implements the `IniConfig` `.ini` parser event handler for the `DynConfig`.
#[cfg(feature = "ini")]
pub(crate) struct DynConfigIniConfig {
    root: DynTable,
    current_section: Option<DynTable>,
    // Never allocates if we don't support nested sections.
    section_stack: Vec<DynTable>,
    // Always `None` if we don't support arrays.
    current_array: Option<DynArray>,
}

#[cfg(feature = "ini")]
impl DynConfigIniConfig {
    pub fn new() -> Self {
        Self {
            root: DynTable::new(),
            current_section: None,
            section_stack: Vec::new(),
            current_array: None,
        }
    }

    pub fn into_inner(self) -> DynConfig {
        debug_assert!(
            self.current_section.is_none(),
            "missing `end_section()` call"
        );
        debug_assert!(
            self.section_stack.is_empty(),
            "missing `end_section()` call"
        );
        debug_assert!(self.current_array.is_none(), "missing `end_array()` call");

        DynConfig(self.root)
    }
}

#[cfg(feature = "ini")]
impl<'s> IniConfig<'s> for DynConfigIniConfig {
    fn contains_key(&self, key: NonEmptyIniStr<'s, '_>) -> Option<bool> {
        let table = self.current_section.as_ref().unwrap_or(&self.root);
        table
            .get_impl(key.as_ne_str())
            .map(|val| val.table().is_some())
    }

    fn add_value(&mut self, key: NonEmptyIniStr<'s, '_>, value: IniValue<'s, '_>, overwrite: bool) {
        let table = self.current_section.as_mut().unwrap_or(&mut self.root);

        let key = key.as_ne_str();

        let already_existed = match value {
            IniValue::Bool(value) => table.set(key, value),
            IniValue::I64(value) => table.set(key, value),
            IniValue::F64(value) => table.set(key, value),
            IniValue::String(value) => table.set(key, value.as_str()),
        };

        debug_assert!(
            overwrite == already_existed,
            "overwrite flag mismatch when adding a value"
        );
    }

    fn start_section(&mut self, section: NonEmptyIniStr<'s, '_>, overwrite: bool) {
        let start_section_in_section =
            |parent: &mut DynTable, current_section: &mut Option<DynTable>| {
                // Overwrite the previous value / section with this key in the parent section.
                if overwrite {
                    let already_existed = parent.remove(section.as_ne_str());
                    debug_assert!(
                        already_existed.is_some(),
                        "overwrite flag mismatch when starting a section"
                    );
                    current_section.replace(DynTable::new());

                // Add a new section or continue the previous section with this key in the parent section.
                } else {
                    // Previous value at this key was a section - continue it.
                    if let Some(previous) = parent
                        .remove_impl(section.as_ne_str())
                        .map(Value::table)
                        .flatten()
                    {
                        current_section.replace(previous);

                    // Else it was a value and we will overwrite it.
                    } else {
                        current_section.replace(DynTable::new());
                    }
                }
            };

        if let Some(mut current_section) = self.current_section.take() {
            start_section_in_section(&mut current_section, &mut self.current_section);

            self.section_stack.push(current_section);
        } else {
            start_section_in_section(&mut self.root, &mut self.current_section);
        }
    }

    fn end_section(&mut self, section: NonEmptyIniStr<'s, '_>) {
        if let Some(current_section) = self.current_section.take() {
            if let Some(mut parent_section) = self.section_stack.pop() {
                let already_existed = parent_section.set(section.as_ne_str(), current_section);
                debug_assert!(!already_existed);
                self.current_section.replace(parent_section);
            } else {
                let already_existed = self.root.set(section.as_ne_str(), current_section);
                debug_assert!(!already_existed);
            }
        } else {
            debug_assert!(
                false,
                "`end_section()` call without a matching `start_section()`"
            );
        }
    }

    fn start_array(&mut self, array: NonEmptyIniStr<'s, '_>, overwrite: bool) {
        let table = self.current_section.as_mut().unwrap_or(&mut self.root);

        if overwrite {
            let previous = table.remove(array.as_ne_str());
            debug_assert!(
                previous.is_some(),
                "overwrite flag mismatch when starting an array"
            );
        }

        debug_assert!(
            self.current_array.is_none(),
            "nested arrays are not supported"
        );
        self.current_array.replace(DynArray::new());
    }

    fn add_array_value(&mut self, value: IniValue<'s, '_>) {
        if let Some(current_array) = self.current_array.as_mut() {
            let result = current_array.push(match value {
                IniValue::Bool(value) => Value::Bool(value),
                IniValue::I64(value) => Value::I64(value),
                IniValue::F64(value) => Value::F64(value),
                IniValue::String(value) => Value::String(value.into()),
            });
            debug_assert!(result.is_ok(), "incorrect array value type");
        } else {
            debug_assert!(
                false,
                "`add_array_value()` call without a matching `start_array()`"
            );
        }
    }

    fn end_array(&mut self, array: NonEmptyIniStr<'s, '_>) {
        if let Some(current_array) = self.current_array.take() {
            let root = &mut self.root;
            let table = self.current_section.as_mut().unwrap_or(root);
            let existed = table.set(array.as_ne_str(), current_array);
            debug_assert!(!existed);
        } else {
            debug_assert!(
                false,
                "`end_array()` call without a matching `start_array()`"
            );
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
    keys.sort();

    // Iterate the table using the sorted keys.
    for key in keys.into_iter() {
        // Must succeed - all keys are valid.
        let value = unwrap_unchecked(
            table.get_val(key),
            "failed to get a value from a dyn config table with a valid key",
        );

        value_to_bin_config(Some(key), value, writer)?;
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
    key: Option<&NonEmptyStr>,
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

    use {crate::*, ministr_macro::nestr};

    #[test]
    fn GetPathError_PathDoesNotExist() {
        let mut table = DynTable::new();

        let mut foo = DynTable::new();
        let mut bar = DynArray::new();
        let mut baz = DynTable::new();

        assert!(!baz.set(nestr!("bob"), true));
        bar.push(baz.into()).unwrap();
        assert!(!foo.set(nestr!("bar"), bar));
        assert!(!table.set(nestr!("foo"), foo));

        assert_eq!(
            table.get_val_path(&["".into()]).err().unwrap(),
            GetPathError::KeyDoesNotExist(ConfigPath::new())
        );
        assert_eq!(
            table
                .get_val_path(&["foo".into(), "".into()])
                .err()
                .unwrap(),
            GetPathError::KeyDoesNotExist(vec![nestr!("foo").into()].into())
        );
        assert_eq!(
            table
                .get_val_path(&["foo".into(), "baz".into()])
                .err()
                .unwrap(),
            GetPathError::KeyDoesNotExist(vec![nestr!("foo").into(), nestr!("baz").into()].into())
        );

        assert_eq!(
            table
                .get_val_path(&["foo".into(), "bar".into(), 0.into(), "bill".into()])
                .err()
                .unwrap(),
            GetPathError::KeyDoesNotExist(
                vec![
                    nestr!("foo").into(),
                    nestr!("bar").into(),
                    0.into(),
                    nestr!("bill").into()
                ]
                .into()
            )
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

        assert!(!table.set(nestr!("array"), array));

        assert_eq!(
            table
                .get_val_path(&["array".into(), 1.into()])
                .err()
                .unwrap(),
            GetPathError::IndexOutOfBounds {
                path: vec![nestr!("array").into(), 1.into()].into(),
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
        assert!(!other_table.set(nestr!("array"), true));

        assert!(!table.set(nestr!("table"), other_table));

        assert_eq!(
            table
                .get_val_path(&["table".into(), "array".into(), 1.into()])
                .err()
                .unwrap(),
            GetPathError::ValueNotAnArray {
                path: vec![nestr!("table").into(), nestr!("array").into()].into(),
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

        assert!(!table.set(nestr!("array"), array));

        assert_eq!(
            table
                .get_val_path(&["array".into(), 0.into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::ValueNotATable {
                path: vec![nestr!("array").into(), 0.into()].into(),
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
        assert!(!other_table.set(nestr!("foo"), true));
        assert!(!other_table.set(nestr!("bar"), 3.14));

        assert!(!table.set(nestr!("table"), other_table));

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

        assert!(!root.set(nestr!("array_value"), array_value));
        assert!(!root.set(nestr!("bool_value"), true));
        assert!(!root.set(nestr!("float_value"), 3.14));
        assert!(!root.set(nestr!("int_value"), 7));
        assert!(!root.set(nestr!("string_value"), "foo"));

        let mut table_value = DynTable::new();

        assert!(!table_value.set(nestr!("bar"), 2020));
        assert!(!table_value.set(nestr!("baz"), "hello"));
        assert!(!table_value.set(nestr!("foo"), false));
        assert!(!root.set(nestr!("table_value"), table_value));

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

        assert!(!config.root_mut().set(nestr!("array"), array));

        assert!(!config.root_mut().set(nestr!("bool"), true));
        assert!(!config.root_mut().set(nestr!("float"), 3.14));
        assert!(!config.root_mut().set(nestr!("int"), 7));
        assert!(!config.root_mut().set(nestr!("string"), "foo"));

        let mut other_section = DynTable::new();

        assert!(!other_section.set(nestr!("other_bool"), true));
        assert!(!other_section.set(nestr!("other_float"), 3.14));
        assert!(!other_section.set(nestr!("other_int"), 7));
        assert!(!other_section.set(nestr!("other_string"), "foo"));

        assert!(!config
            .root_mut()
            .set(nestr!("other_section"), other_section));

        let mut section = DynTable::new();

        assert!(!section.set(nestr!("bool"), false));
        assert!(!section.set(nestr!("float"), 7.62));
        assert!(!section.set(nestr!("int"), 9));
        assert!(!section.set(nestr!("string"), "bar"));

        assert!(!config.root_mut().set(nestr!("section"), section));

        let string = config
            .to_ini_string_opts(ToIniStringOptions {
                arrays: true,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(string, ini);
    }
}
