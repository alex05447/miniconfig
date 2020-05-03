use std::fmt::{Display, Formatter};
use std::io::Write;
use std::mem::size_of;
use std::ops::DerefMut;

use crate::{
    BinArray, BinConfigError, BinConfigValue, BinConfigWriterError, BinTable, DisplayLua, Value,
    ValueType,
};

use super::array_or_table::BinArrayOrTable;
use super::util::{string_hash_fnv1a, u32_from_bin, u32_to_bin_bytes};
use super::value::BinConfigPackedValue;

#[cfg(feature = "ini")]
use crate::{DisplayIni, ToIniStringError, ToIniStringOptions};

#[cfg(feature = "dyn")]
use crate::{DynArray, DynConfig, DynTable};

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
        use Value::*;

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
        use Value::*;

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

impl<'a> Display for BinConfigValue<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
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
