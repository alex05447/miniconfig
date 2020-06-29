use {
    super::{
        array_or_table::BinArrayOrTable,
        util::{string_hash_fnv1a, u32_from_bin, u32_to_bin_bytes},
        value::BinConfigPackedValue,
    },
    crate::{util::DisplayLua, BinConfigError, BinConfigWriterError, BinTable, ValueType},
    std::{
        fmt::{Display, Formatter},
        io::Write,
        mem::size_of,
    },
};

#[cfg(feature = "dyn")]
use {
    crate::{BinArray, BinConfigValue, DynArray, DynConfig, DynTable},
    std::ops::DerefMut,
};

#[cfg(feature = "ini")]
use crate::{DisplayIni, ToIniStringError, ToIniStringOptions};

/// Represents an immutable config with a root hashmap [`table`].
///
/// [`table`]: struct.BinTable.html
#[derive(Debug)]
pub struct BinConfig(Box<[u8]>);

impl BinConfig {
    const MAX_SIZE: u32 = std::u32::MAX;
    const MAX_ARRAY_OR_TABLE_LEN: u32 = (Self::MAX_SIZE - size_of::<BinConfigHeader>() as u32)
        / size_of::<BinConfigPackedValue>() as u32;

    /// Creates a new [`config`] from the `data` binary blob.
    ///
    /// [`config`]: struct.BinConfig.html
    pub fn new(data: Box<[u8]>) -> Result<Self, BinConfigError> {
        // Try to validate the data.
        Self::validate_data(&data)?;
        // Seems to be fine?

        Ok(Self(data))
    }

    /// Returns the immutable reference to the root [`table`] of the [`config`].
    ///
    /// [`table`]: struct.BinTable.html
    /// [`config`]: struct.BinConfig.html
    pub fn root(&self) -> BinTable<'_> {
        // We ensured the data is validated.
        unsafe { Self::root_impl(&self.0) }
    }

    /// Tries to serialize this [`config`] to a Lua script string.
    ///
    /// NOTE: you may also call `to_string` via the [`config`]'s `Display` implementation.
    ///
    /// [`config`]: struct.BinConfig.html
    pub fn to_lua_string(&self) -> Result<String, std::fmt::Error> {
        use std::fmt::Write;

        let mut result = String::new();

        write!(&mut result, "{}", self)?;

        result.shrink_to_fit();

        Ok(result)
    }

    /// Tries to serialize this [`config`] to an `.ini` string.
    ///
    /// [`config`]: struct.BinConfig.html
    #[cfg(feature = "ini")]
    pub fn to_ini_string(&self) -> Result<String, ToIniStringError> {
        self.to_ini_string_opts(Default::default())
    }

    /// Tries to serialize this [`config`] to an `.ini` string using provided [`options`].
    ///
    /// [`config`]: struct.BinConfig.html
    /// [`options`]: struct.ToIniStringOptions.html
    #[cfg(feature = "ini")]
    pub fn to_ini_string_opts(
        &self,
        options: ToIniStringOptions,
    ) -> Result<String, ToIniStringError> {
        let mut result = String::new();

        self.root().fmt_ini(&mut result, 0, false, options)?;

        result.shrink_to_fit();

        Ok(result)
    }

    /// Serializes this [`config`] to a [`dynamic config`].
    ///
    /// [`config`]: struct.BinConfig.html
    /// [`dynamic config`]: struct.DynConfig.html
    #[cfg(feature = "dyn")]
    pub fn to_dyn_config(&self) -> DynConfig {
        let mut result = DynConfig::new();

        Self::table_to_dyn_table(self.root(), result.root_mut());

        result
    }

    // Returns the pointer to the start of the config data blob w.r.t. which
    // all config inner ofsets are specified.
    fn base(data: &[u8]) -> *const u8 {
        data.as_ptr()
    }

    /// The caller ensures the data is at least large enough for the header.
    unsafe fn header(data: &[u8]) -> &BinConfigHeader {
        &*(Self::base(data) as *const _)
    }

    /// The caller ensures the data is at least large enough for the header.
    unsafe fn root_impl(data: &[u8]) -> BinTable<'_> {
        BinTable::new(Self::root_raw_impl(data))
    }

    /// Constructs the root table from the binary config blob data.
    /// NOTE - the caller ensures that data is a valid binary config header.
    unsafe fn root_raw_impl(data: &[u8]) -> BinArrayOrTable<'_> {
        BinArrayOrTable::new(
            Self::base(data),                    // Base address of the binary config.
            size_of::<BinConfigHeader>() as u32, // Offset to the first value of the root table is the size of the header.
            Self::header(data).len(), // Config root table length as read from the header.
        )
    }

    fn validate_data(data: &[u8]) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Make sure the data is large enough to contain at least the header and one value.
        if data.len() < size_of::<BinConfigHeader>() + size_of::<BinConfigPackedValue>() {
            return Err(InvalidBinaryConfigData);
        }

        // Make sure the data is not too large.
        if data.len() > Self::MAX_SIZE as usize {
            return Err(InvalidBinaryConfigData);
        }

        // Read the header.
        let header = unsafe { BinConfig::header(&data) };

        // Check the header magic.
        if !header.check_magic() {
            return Err(InvalidBinaryConfigData);
        }

        // Check the root table.
        if header.len() > 0 {
            let root = unsafe { Self::root_raw_impl(data) };

            // Make sure the root table slice lies within the config data blob.
            // Offset to the first value of the root table is the size of the header.
            Self::validate_range(
                (size_of::<BinConfigHeader>() as u32, data.len() as u32),
                root.offset_range(),
            )?;

            Self::validate_table(data, &root)

        // Empty binary config root tables are not supported.
        } else {
            Err(InvalidBinaryConfigData)
        }
    }

    fn validate_table(data: &[u8], table: &BinArrayOrTable<'_>) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Empty tables must have no data offset.
        if table.len == 0 && table.offset != 0 {
            return Err(InvalidBinaryConfigData);
        }

        if table.len > Self::MAX_ARRAY_OR_TABLE_LEN {
            return Err(InvalidBinaryConfigData);
        }

        if table.len > 0 {
            // Valid data offset range for table values.
            // Minimum offset to string/array/table values in the table is just past the table's packed values.
            let valid_range = (
                table.offset + table.len * size_of::<BinConfigPackedValue>() as u32,
                data.len() as u32,
            );

            // For each table element.
            for index in 0..table.len {
                let value = unsafe { table.packed_value(index) };

                // All values in the table must have a key.
                let key = value.key();

                // Validate the key.
                //----------------------------------------------------------------------------------
                // Key length must be positive.
                if key.len == 0 {
                    return Err(InvalidBinaryConfigData);
                }

                // Make sure the key string and the null terminator lie within the config data blob (`+ 1`for null terminator).
                Self::validate_range(valid_range, (key.offset, key.offset + key.len + 1))?;

                // Make sure the key string is null-terminated.
                let null_terminator = unsafe { table.slice(key.offset + key.len, 1) };

                if null_terminator[0] != b'\0' {
                    return Err(InvalidBinaryConfigData);
                }

                // Make sure the key string is valid UTF-8.
                let key_slice = unsafe { table.slice(key.offset, key.len) };

                let key_string =
                    std::str::from_utf8(key_slice).map_err(|_| InvalidBinaryConfigData)?;

                // Make sure the key hash matches the string.
                if string_hash_fnv1a(key_string) != key.hash {
                    return Err(InvalidBinaryConfigData);
                }
                //----------------------------------------------------------------------------------
                // The key seems to be OK.

                // Validate the value.
                Self::validate_value(data, table, valid_range, value)?;
                // The value seems to be OK.
            }
        }

        Ok(())
    }

    fn validate_array(data: &[u8], array: &BinArrayOrTable<'_>) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Empty arrays must have no data offset.
        if array.len == 0 && array.offset != 0 {
            return Err(InvalidBinaryConfigData);
        }

        if array.len > Self::MAX_ARRAY_OR_TABLE_LEN {
            return Err(InvalidBinaryConfigData);
        }

        // Valid data offset range for array values.
        // Minimum offset to string/array/table values in the array is just past the array's packed values.
        let valid_range = (
            array.offset + array.len * size_of::<BinConfigPackedValue>() as u32,
            data.len() as u32,
        );

        // For each array element.
        for index in 0..array.len {
            let value = unsafe { array.packed_value(index) };

            // All values in the array must have no keys.
            let key = value.key();

            if key.len != 0 || key.offset != 0 || key.hash != 0 {
                return Err(InvalidBinaryConfigData);
            }

            // Validate the value.
            Self::validate_value(data, array, valid_range, value)?;
            // The value seems to be OK.
        }

        Ok(())
    }

    fn validate_range(valid_range: (u32, u32), range: (u32, u32)) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        if range.0 < valid_range.0 {
            return Err(InvalidBinaryConfigData);
        }

        if range.1 > valid_range.1 {
            return Err(InvalidBinaryConfigData);
        }

        Ok(())
    }

    fn validate_value(
        data: &[u8],
        array_or_table: &BinArrayOrTable<'_>, // Validated value's parent array/table.
        valid_range: (u32, u32), // Valid range of offsets within the binary data blob for this string/array/table value.
        value: &BinConfigPackedValue,
    ) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Make sure the value type is valid.
        let value_type = value.try_value_type().ok_or(InvalidBinaryConfigData)?;

        match value_type {
            // Only `0` and `1` are valid for `bool` values.
            ValueType::Bool => {
                value.try_bool().ok_or(InvalidBinaryConfigData)?;
            }
            ValueType::I64 | ValueType::F64 => {}
            ValueType::String => {
                // Non-empty strings have a positive offset to data.
                if value.len() > 0 {
                    // Make sure the string and the null terminator lie within the config data blob (`+ 1`for null terminator).
                    Self::validate_range(
                        valid_range,
                        (value.offset(), value.offset() + value.len() + 1),
                    )?;

                    // Make sure the value string is null-terminated.
                    let null_terminator =
                        unsafe { array_or_table.slice(value.offset() + value.len(), 1) };

                    if null_terminator[0] != b'\0' {
                        return Err(InvalidBinaryConfigData);
                    }

                    // Make sure the value string is valid UTF-8.
                    let string_slice = unsafe { array_or_table.slice(value.offset(), value.len()) };

                    std::str::from_utf8(string_slice).map_err(|_| InvalidBinaryConfigData)?;

                // Empty value strings must have no offset.
                } else if value.offset() != 0 {
                    return Err(InvalidBinaryConfigData);
                }
            }
            ValueType::Array | ValueType::Table => {
                if value.len() > Self::MAX_ARRAY_OR_TABLE_LEN {
                    return Err(InvalidBinaryConfigData);
                }

                // Non-empty arrays/tables have a positive offset to data.
                if value.len() > 0 {
                    let array_or_table =
                        BinArrayOrTable::new(Self::base(data), value.offset(), value.len());

                    // Make sure the array/table slice lies within the config data blob.
                    Self::validate_range(valid_range, array_or_table.offset_range())?;

                    // Validate the array/table values.
                    match value_type {
                        ValueType::Array => {
                            Self::validate_array(data, &array_or_table)?;
                        }
                        ValueType::Table => {
                            Self::validate_table(data, &array_or_table)?;
                        }
                        _ => unreachable!(),
                    }

                // Empty arrays/tables must have no offset.
                } else if value.offset() != 0 {
                    return Err(InvalidBinaryConfigData);
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "dyn")]
    fn table_to_dyn_table<'t, T: DerefMut<Target = DynTable>>(
        table: BinTable<'_>,
        mut dyn_table: T,
    ) {
        let dyn_table = dyn_table.deref_mut();

        for (key, value) in table.iter() {
            Self::value_to_dyn_table(key, value, dyn_table);
        }
    }

    #[cfg(feature = "dyn")]
    fn array_to_dyn_array<A: DerefMut<Target = DynArray>>(array: BinArray<'_>, mut dyn_array: A) {
        let dyn_array = dyn_array.deref_mut();

        for value in array.iter() {
            Self::value_to_dyn_array(value, dyn_array);
        }
    }

    #[cfg(feature = "dyn")]
    fn value_to_dyn_table(key: &str, value: BinConfigValue<'_>, dyn_table: &mut DynTable) {
        use crate::Value::{self, *};

        match value {
            Bool(value) => {
                dyn_table.set(key, Value::Bool(value)).unwrap();
            }
            I64(value) => {
                dyn_table.set(key, Value::I64(value)).unwrap();
            }
            F64(value) => {
                dyn_table.set(key, Value::F64(value)).unwrap();
            }
            String(value) => {
                dyn_table.set(key, Value::String(value.into())).unwrap();
            }
            Array(value) => {
                let mut array = DynArray::new();
                Self::array_to_dyn_array(value, &mut array);
                dyn_table.set(key, Value::Array(array)).unwrap();
            }
            Table(value) => {
                let mut table = DynTable::new();
                Self::table_to_dyn_table(value, &mut table);
                dyn_table.set(key, Value::Table(table)).unwrap();
            }
        }
    }

    #[cfg(feature = "dyn")]
    fn value_to_dyn_array(value: BinConfigValue<'_>, dyn_array: &mut DynArray) {
        use crate::Value::{self, *};

        match value {
            Bool(value) => {
                dyn_array.push(Value::Bool(value)).unwrap();
            }
            I64(value) => {
                dyn_array.push(Value::I64(value)).unwrap();
            }
            F64(value) => {
                dyn_array.push(Value::F64(value)).unwrap();
            }
            String(value) => {
                dyn_array.push(Value::String(value.to_owned())).unwrap();
            }
            Array(value) => {
                let mut array = DynArray::new();
                Self::array_to_dyn_array(value, &mut array);
                dyn_array.push(Value::Array(array)).unwrap();
            }
            Table(value) => {
                let mut table = DynTable::new();
                Self::table_to_dyn_table(value, &mut table);
                dyn_array.push(Value::Table(table)).unwrap();
            }
        }
    }
}

impl Display for BinConfig {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.root().fmt_lua(f, 0)
    }
}

/// Binary config data blob header.
///
/// Fields are in whatever endianness we use; see `super::util::__to_bin_bytes(), _from_bin()`.
#[repr(C, packed)]
pub(super) struct BinConfigHeader {
    // Arbitrary magic value for a quick sanity check.
    magic: u32,
    // Followed by the root table length.
    len: u32,
}

const BIN_CONFIG_HEADER_MAGIC: u32 = 0x67666362; // `bcfg`, little endian.

impl BinConfigHeader {
    fn check_magic(&self) -> bool {
        u32_from_bin(self.magic) == BIN_CONFIG_HEADER_MAGIC
    }

    pub(super) fn len(&self) -> u32 {
        u32_from_bin(self.len)
    }

    pub(super) fn write<W: Write>(writer: &mut W, len: u32) -> Result<(), BinConfigWriterError> {
        use BinConfigWriterError::*;

        // Magic.
        writer
            .write(&u32_to_bin_bytes(BIN_CONFIG_HEADER_MAGIC))
            .map_err(|_| WriteError)?;

        // Root table length.
        writer
            .write(&u32_to_bin_bytes(len))
            .map_err(|_| WriteError)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use crate::*;

    #[test]
    fn GetPathError_EmptyKey() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.table("foo", 1).unwrap();
        writer.bool("bar", true).unwrap();
        writer.end().unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config
                .root()
                .get_path(&["foo".into(), "".into()])
                .err()
                .unwrap(),
            GetPathError::EmptyKey(ConfigPath(vec!["foo".into()]))
        );

        // But this works.

        assert_eq!(
            config
                .root()
                .get_bool_path(&["foo".into(), "bar".into()])
                .unwrap(),
            true,
        );
    }

    #[test]
    fn GetPathError_PathDoesNotExist() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.table("foo", 1).unwrap();
        writer.array("bar", 1).unwrap();
        writer.table(None, 1).unwrap();
        writer.bool("bob", true).unwrap();
        writer.end().unwrap();
        writer.end().unwrap();
        writer.end().unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config
                .root()
                .get_path(&["foo".into(), "baz".into()])
                .err()
                .unwrap(),
            GetPathError::KeyDoesNotExist(ConfigPath(vec!["foo".into(), "baz".into()]))
        );

        assert_eq!(
            config
                .root()
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
            config
                .root()
                .get_bool_path(&["foo".into(), "bar".into(), 0.into(), "bob".into()])
                .unwrap(),
            true
        );
    }

    #[test]
    fn GetPathError_IndexOutOfBounds() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array("array", 1).unwrap();
        writer.bool(None, true).unwrap();
        writer.end().unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config
                .root()
                .get_path(&["array".into(), 1.into()])
                .err()
                .unwrap(),
            GetPathError::IndexOutOfBounds {
                path: ConfigPath(vec!["array".into(), 1.into()]),
                len: 1
            }
        );

        // But this works.

        assert_eq!(
            config
                .root()
                .get_bool_path(&["array".into(), 0.into()])
                .unwrap(),
            true
        );
    }

    #[test]
    fn GetPathError_ValueNotAnArray() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.table("table", 1).unwrap();
        writer.bool("array", true).unwrap();
        writer.end().unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config
                .root()
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
            config
                .root()
                .get_bool_path(&["table".into(), "array".into()])
                .unwrap(),
            true,
        );
    }

    #[test]
    fn GetPathError_ValueNotATable() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array("array", 1).unwrap();
        writer.bool(None, true).unwrap();
        writer.end().unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config
                .root()
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
            config
                .root()
                .get_bool_path(&["array".into(), 0.into()])
                .unwrap(),
            true,
        );
    }

    #[test]
    fn GetPathError_IncorrectValueType() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.table("table", 2).unwrap();
        writer.bool("foo", true).unwrap();
        writer.f64("bar", 3.14).unwrap();
        writer.end().unwrap();
        let data = writer.finish().unwrap();
        let config = BinConfig::new(data).unwrap();

        assert_eq!(
            config
                .root()
                .get_i64_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            config
                .root()
                .get_f64_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            config
                .root()
                .get_string_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            config
                .root()
                .get_array_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );
        assert_eq!(
            config
                .root()
                .get_table_path(&["table".into(), "foo".into()])
                .err()
                .unwrap(),
            GetPathError::IncorrectValueType(ValueType::Bool)
        );

        // But this works.

        assert_eq!(
            config
                .root()
                .get_bool_path(&["table".into(), "foo".into()])
                .unwrap(),
            true
        );

        assert_eq!(
            config
                .root()
                .get_i64_path(&["table".into(), "bar".into()])
                .unwrap(),
            3
        );
        assert!(cmp_f64(
            config
                .root()
                .get_f64_path(&["table".into(), "bar".into()])
                .unwrap(),
            3.14
        ));
    }

    #[test]
    fn hash_collisions() {
        // See `fnv1a_hash_collisions()`.

        let mut writer = BinConfigWriter::new(2).unwrap();

        writer.string("costarring", "declinate").unwrap();
        writer.string("liquid", "macallums").unwrap();

        let data = writer.finish().unwrap();

        let config = BinConfig::new(data).unwrap();

        assert_eq!(config.root().get_string("liquid").unwrap(), "macallums");
        assert_eq!(config.root().get_string("costarring").unwrap(), "declinate");
    }

    #[cfg(feature = "dyn")]
    #[test]
    fn to_dyn_config() {
        let mut writer = BinConfigWriter::new(6).unwrap();

        writer.array("array_value", 3).unwrap();
        writer.i64(None, 54).unwrap();
        writer.i64(None, 12).unwrap();
        writer.f64(None, 78.9).unwrap();
        writer.end().unwrap();

        writer.bool("bool_value", true).unwrap();
        writer.f64("float_value", 3.14).unwrap();
        writer.i64("int_value", 7).unwrap();
        writer.string("string_value", "foo").unwrap();

        writer.table("table_value", 3).unwrap();
        writer.i64("bar", 2020).unwrap();
        writer.string("baz", "hello").unwrap();
        writer.bool("foo", false).unwrap();
        writer.end().unwrap();

        let data = writer.finish().unwrap();

        let config = BinConfig::new(data).unwrap();

        // Serialize to dynamic config.
        let dyn_config = config.to_dyn_config();

        let array_value = dyn_config.root().get_array("array_value").unwrap();

        assert_eq!(array_value.len(), 3);
        assert_eq!(array_value.get_i64(0).unwrap(), 54);
        assert!(cmp_f64(array_value.get_f64(0).unwrap(), 54.0));
        assert_eq!(array_value.get_i64(1).unwrap(), 12);
        assert!(cmp_f64(array_value.get_f64(1).unwrap(), 12.0));
        assert_eq!(array_value.get_i64(2).unwrap(), 78);
        assert!(cmp_f64(array_value.get_f64(2).unwrap(), 78.9));

        assert_eq!(dyn_config.root().get_bool("bool_value").unwrap(), true);

        assert!(cmp_f64(
            dyn_config.root().get_f64("float_value").unwrap(),
            3.14
        ));

        assert_eq!(dyn_config.root().get_i64("int_value").unwrap(), 7);

        assert_eq!(dyn_config.root().get_string("string_value").unwrap(), "foo");

        let table_value = dyn_config.root().get_table("table_value").unwrap();

        assert_eq!(table_value.len(), 3);
        assert_eq!(table_value.get_i64("bar").unwrap(), 2020);
        assert!(cmp_f64(table_value.get_f64("bar").unwrap(), 2020.0));
        assert_eq!(table_value.get_string("baz").unwrap(), "hello");
        assert_eq!(table_value.get_bool("foo").unwrap(), false);
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

        let mut writer = BinConfigWriter::new(7).unwrap();

        writer.array("array", 3).unwrap();
        writer.string(None, "foo").unwrap();
        writer.string(None, "bar").unwrap();
        writer.string(None, "baz").unwrap();
        writer.end().unwrap();

        writer.bool("bool", true).unwrap();
        writer.f64("float", 3.14).unwrap();
        writer.i64("int", 7).unwrap();
        writer.string("string", "foo").unwrap();

        writer.table("other_section", 4).unwrap();
        writer.bool("other_bool", true).unwrap();
        writer.f64("other_float", 3.14).unwrap();
        writer.i64("other_int", 7).unwrap();
        writer.string("other_string", "foo").unwrap();
        writer.end().unwrap();

        writer.table("section", 4).unwrap();
        writer.bool("bool", false).unwrap();
        writer.f64("float", 7.62).unwrap();
        writer.i64("int", 9).unwrap();
        writer.string("string", "bar").unwrap();
        writer.end().unwrap();

        let data = writer.finish().unwrap();

        let config = BinConfig::new(data).unwrap();

        let string = config
            .to_ini_string_opts(ToIniStringOptions {
                arrays: true,
                ..Default::default()
            })
            .unwrap();

        assert_eq!(string, ini);
    }
}
