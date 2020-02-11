use std::fmt::{Display, Formatter};
use std::io::Write;

use super::array_or_table::BinArrayOrTable;
use super::util::string_hash_fnv1a;
use super::value::BinConfigPackedValue;
use crate::{
    BinArray, BinConfigError, BinConfigWriterError, BinTable, DisplayLua, Value, ValueType,
};

#[cfg(feature = "ini")]
use crate::{DisplayINI, ToINIStringError};

/// Represents an immutable config with a root hashmap [`table`].
///
/// [`table`]: struct.BinTable.html
#[derive(Debug)]
pub struct BinConfig(Box<[u8]>);

impl std::cmp::PartialEq for BinConfig {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl BinConfig {
    const MAX_SIZE: u32 = std::u32::MAX;

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

    /// Tries to serialize this [`config`] to an INI string.
    ///
    /// [`config`]: struct.BinConfig.html
    #[cfg(feature = "ini")]
    pub fn to_ini_string(&self) -> Result<String, ToINIStringError> {
        let mut result = String::new();

        self.root().fmt_ini(&mut result, 0)?;

        result.shrink_to_fit();

        Ok(result)
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
            Self::base(data),                              // Base address of the binary config.
            std::mem::size_of::<BinConfigHeader>() as u32, // Offset to the first value of the root table is the size of the header.
            Self::header(data).len(), // Config root table length as read from the header.
        )
    }

    fn validate_data(data: &[u8]) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Make sure the data is large enough to contain at least the header.
        if data.len() < std::mem::size_of::<BinConfigHeader>() {
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

        if header.len() > 0 {
            let root = unsafe { Self::root_raw_impl(data) };

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

        // Minimum offset to string/array/table values in the table is just past the table packed value.
        let min_offset = table.offset + std::mem::size_of::<BinConfigPackedValue>() as u32;

        // Valid data offset range.
        let valid_range = (min_offset, data.len() as u32 - min_offset);

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
            Self::validate_range(valid_range, (key.offset, key.len + 1))?;

            // Make sure the key string is null-terminated.
            let null_terminator = unsafe { table.slice(key.offset + key.len, 1) };

            if null_terminator[0] != b'\0' {
                return Err(InvalidBinaryConfigData);
            }

            // Make sure the key string is valid UTF-8.
            let key_slice = unsafe { table.slice(key.offset, key.len) };

            let key_string = std::str::from_utf8(key_slice).map_err(|_| InvalidBinaryConfigData)?;

            // Make sure the key hash matches the string.
            if string_hash_fnv1a(key_string) != key.hash {
                return Err(InvalidBinaryConfigData);
            }
            //----------------------------------------------------------------------------------
            // The key seems to be OK.

            // Validate the value.
            let value_offset = unsafe { table.packed_value_offset(index) };
            Self::validate_value(data, table, value_offset, value)?;
            // The value seems to be OK.
        }

        Ok(())
    }

    fn validate_array(data: &[u8], array: &BinArrayOrTable<'_>) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Empty arrays must have no data offset.
        if array.len == 0 && array.offset != 0 {
            return Err(InvalidBinaryConfigData);
        }

        // For each array element.
        for index in 0..array.len {
            let value = unsafe { array.packed_value(index) };

            // All values in the array must have no keys.
            let key = value.key();

            if key.len != 0 || key.offset != 0 || key.hash != 0 {
                return Err(InvalidBinaryConfigData);
            }

            // Validate the value.
            let value_offset = unsafe { array.packed_value_offset(index) };
            Self::validate_value(data, array, value_offset, value)?;
            // The value seems to be OK.
        }

        Ok(())
    }

    fn validate_range(valid_range: (u32, u32), range: (u32, u32)) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        if range.1 > valid_range.1 {
            return Err(InvalidBinaryConfigData);
        }

        if range.0 < valid_range.0 {
            return Err(InvalidBinaryConfigData);
        }

        if range.0 > (valid_range.0 + valid_range.1) {
            return Err(InvalidBinaryConfigData);
        }

        if (range.0 + range.1) > (valid_range.0 + valid_range.1) {
            return Err(InvalidBinaryConfigData);
        }

        Ok(())
    }

    fn validate_value(
        data: &[u8],
        array_or_table: &BinArrayOrTable<'_>, // Validated value's parent array/table.
        value_offset: u32, // Offset to the validated packed value in the data blob.
        value: &BinConfigPackedValue,
    ) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Minimum offset to string/array/table values in the table is just past the packed value itself.
        let min_offset = value_offset + std::mem::size_of::<BinConfigPackedValue>() as u32;

        // Valid data offset range.
        let valid_range = (min_offset, data.len() as u32 - min_offset);

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
                    Self::validate_range(valid_range, (value.offset(), value.len() + 1))?;

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
            ValueType::Array => {
                // Non-empty arrays have a positive offset to data.
                if value.len() > 0 {
                    // Make sure the array slice lies within the config data blob.
                    Self::validate_range(
                        valid_range,
                        (
                            value.offset(),
                            value.len() * std::mem::size_of::<BinConfigPackedValue>() as u32,
                        ),
                    )?;

                    // Validate the array values.
                    Self::validate_array(
                        data,
                        &BinArrayOrTable::new(Self::base(data), value.offset(), value.len()),
                    )?;

                // Empty arrays must have no offset.
                } else if value.offset() != 0 {
                    return Err(InvalidBinaryConfigData);
                }
            }
            ValueType::Table => {
                // Non-empty tables have a positive offset to data.
                if value.len() > 0 {
                    // Make sure the table slice lies within the config data blob.
                    Self::validate_range(
                        valid_range,
                        (
                            value.offset(),
                            value.len() * std::mem::size_of::<BinConfigPackedValue>() as u32,
                        ),
                    )?;

                    // Validate the table values.
                    Self::validate_table(
                        data,
                        &BinArrayOrTable::new(Self::base(data), value.offset(), value.len()),
                    )?;

                // Empty arrays must have no offset.
                } else if value.offset() != 0 {
                    return Err(InvalidBinaryConfigData);
                }
            }
        }

        Ok(())
    }
}

impl Display for BinConfig {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.root().fmt_lua(f, 0)
    }
}

impl<'a> Display for Value<&'a str, BinArray<'a>, BinTable<'a>> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}

/// Binary config data blob header.
/// Fields are little-endian.
#[repr(C, packed)]
pub(super) struct BinConfigHeader {
    // Arbitrary magic value for a quick sanity check.
    magic: [u8; 4],
    // Followed by the root table length.
    len: u32,
}

const BIN_CONFIG_HEADER_MAGIC: [u8; 4] = [b'b', b'c', b'f', b'g'];

impl BinConfigHeader {
    fn check_magic(&self) -> bool {
        u32::from_le_bytes(self.magic) == u32::from_le_bytes(BIN_CONFIG_HEADER_MAGIC)
    }

    pub(super) fn len(&self) -> u32 {
        u32::from_le(self.len)
    }

    pub(super) fn write<W: Write>(writer: &mut W, len: u32) -> Result<(), BinConfigWriterError> {
        use BinConfigWriterError::*;

        // Magic.
        writer
            .write(&BIN_CONFIG_HEADER_MAGIC)
            .map_err(|_| WriteError)?;

        // Root table length.
        writer.write(&len.to_le_bytes()).map_err(|_| WriteError)?;

        Ok(())
    }
}
