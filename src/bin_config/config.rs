use {
    super::{
        array_or_table::{BinArrayOrTable, InternedString},
        util::{string_hash_fnv1a, u32_from_bin, u32_to_bin_bytes},
        value::BinConfigPackedValue,
    },
    crate::{util::DisplayLua, BinConfigError, BinConfigWriterError, BinTable, ValueType},
    std::{
        fmt::{Display, Formatter},
        io::Write,
        mem::size_of,
        slice::from_raw_parts,
    },
};

#[cfg(feature = "dyn")]
use {
    crate::{BinArray, BinConfigValue, DynArray, DynConfig, DynTable, NonEmptyStr},
    std::ops::DerefMut,
};

#[cfg(feature = "ini")]
use crate::{DisplayIni, IniPath, ToIniStringError, ToIniStringOptions};

/// Represents an immutable config with a root hashmap [`table`].
///
/// [`table`]: struct.BinTable.html
pub struct BinConfig(Box<[u8]>);

impl BinConfig {
    /// Tries to create a new [`config`] from the `data` binary blob.
    ///
    /// Attempts to validate the binary config `data` blob and returns an [`error`]
    /// if the `data` is not a valid binary config data blob,
    /// e.g. returned by the binary config [`writer`].
    ///
    /// [`config`]: struct.BinConfig.html
    /// [`error`]: enum.BinConfigError.html
    /// [`writer`]: struct.BinConfigWriter.html
    pub fn new(data: Box<[u8]>) -> Result<Self, BinConfigError> {
        // Try to validate the data.
        Self::validate_data(&data)?;
        // Seems to be fine?

        Ok(Self(data))
    }

    /// Attempts to validate the binary config `data` blob and returns an [`error`]
    /// if the `data` is not a valid binary config data blob,
    /// e.g. returned by the binary config [`writer`].
    ///
    /// [`config`]: struct.BinConfig.html
    /// [`error`]: enum.BinConfigError.html
    /// [`writer`]: struct.BinConfigWriter.html
    pub fn validate(data: &Box<[u8]>) -> Result<(), BinConfigError> {
        Self::validate_data(&data)
    }

    /// Like [`new`], but does not validate the `data` at all.
    ///
    /// # Safety
    ///
    /// It's up to the user to ensure that `data` is a valid binary config data blob,
    /// e.g. returned by the binary config [`writer`].
    ///
    /// [`writer`]: struct.BinConfigWriter.html
    pub unsafe fn new_unchecked(data: Box<[u8]>) -> Self {
        Self(data)
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
        let mut path = IniPath::new();

        self.root()
            .fmt_ini(&mut result, 0, false, &mut path, options)?;

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

    /// The caller ensures `key_table_offset` and `key_table_len` are valid and point to
    /// the actual key table in the `data` blob.
    unsafe fn key_table(
        data: &[u8],
        key_table_offset: u32,
        key_table_len: u32,
    ) -> &[InternedString] {
        from_raw_parts(
            data.as_ptr().offset(key_table_offset as _) as *const _,
            key_table_len as _,
        )
    }

    /// The caller ensures the data is at least large enough for the header.
    unsafe fn header(data: &[u8]) -> &BinConfigHeader {
        &*(data.as_ptr() as *const _)
    }

    /// The caller ensures the data is at least large enough for the header.
    unsafe fn root_impl(data: &[u8]) -> BinTable<'_> {
        BinTable::new(Self::root_raw_impl(data))
    }

    /// Constructs the root table from the binary config blob data.
    /// NOTE - the caller ensures that data is a valid binary config header.
    unsafe fn root_raw_impl(data: &[u8]) -> BinArrayOrTable<'_> {
        let header = Self::header(data);

        BinArrayOrTable::new(
            data.as_ptr(), // Base address of the binary config.
            Self::key_table(data, header.key_table_offset(), header.key_table_len()),
            size_of::<BinConfigHeader>() as u32, // Offset to the first value of the root table is the size of the header.
            header.len(), // Config root table length as read from the header.
        )
    }

    /// Header, one value, one key table entry and the shortest possible key.
    const fn min_size() -> usize {
        size_of::<BinConfigHeader>()
            + size_of::<BinConfigPackedValue>()
            + size_of::<InternedString>()
            + Self::min_string_section_size()
    }

    const fn max_size() -> usize {
        u32::MAX as _
    }

    /// Key table comes after the header and at least one value.
    const fn min_key_table_offset() -> usize {
        size_of::<BinConfigHeader>() + size_of::<BinConfigPackedValue>()
    }

    /// 1 byte + terminating null char.
    const fn min_string_section_size() -> usize {
        2
    }

    fn validate_data(data: &[u8]) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Make sure the data is large enough to contain at least the header, one value, one key table entry and the shortest possible key.
        if data.len() < Self::min_size() {
            return Err(InvalidBinaryConfigData);
        }

        // Make sure the data is not too large.
        if data.len() > Self::max_size() {
            return Err(InvalidBinaryConfigData);
        }

        // Read the header.
        let header = unsafe { BinConfig::header(&data) };

        // Check the header magic.
        if !header.check_magic() {
            return Err(InvalidBinaryConfigData);
        }

        // Check the key table - must contain at least one table key, as we don't allow empty root tables.
        if header.key_table_len == 0 {
            return Err(InvalidBinaryConfigData);
        }

        // |---------- header (16b) --------|-------- root table (16b) ------|- key table 0 (8b) -|2b|

        // Make sure the key table lies within the config data blob.
        Self::validate_range(
            // Minus shortest string section length - one byte and a null terminator.
            Self::min_key_table_offset() as u32
                ..data.len() as u32 - Self::min_string_section_size() as u32,
            header.key_table_range(),
        )?;

        // Check the root table.
        if header.len() > 0 {
            let root = unsafe { Self::root_raw_impl(data) };

            // Make sure the root table values lie within the config data blob.
            // Offset to the first value of the root table is the size of the header.
            // Last value of the root table may be just before the key table and the shortest string section.
            let valid_range = size_of::<BinConfigHeader>() as u32
                ..data.len() as u32
                    - Self::min_string_section_size() as u32
                    - header.key_table_size();

            Self::validate_range(valid_range.clone(), root.offset_range())?;

            Self::validate_table(data, header.key_table_offset, &root)

        // Empty binary config root tables are not supported.
        } else {
            Err(InvalidBinaryConfigData)
        }
    }

    fn validate_table(
        data: &[u8],
        key_table_offset: u32,
        table: &BinArrayOrTable<'_>,
    ) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Empty tables must have no data offset.
        if table.len == 0 && table.offset != 0 {
            return Err(InvalidBinaryConfigData);
        }

        let key_table = unsafe { table.key_table() };
        let key_table_size = table.key_table_size();

        if table.len > 0 {
            // Valid offset range for table values.
            // Minimum offset to string/array/table values in the table is just past the table's packed values.
            // Maximum offset is just before the key table and the shortest string section.
            let mut valid_range = table.offset_range().end
                ..data.len() as u32 - Self::min_string_section_size() as u32 - key_table_size;

            // Valid offset range for strings.
            let valid_string_range = key_table_offset + key_table_size..data.len() as u32;

            // For each table element.
            for index in 0..table.len {
                let value = unsafe { table.packed_value(index) };

                // All values in the table must have a key.
                let key = value.key();

                // Validate the key.
                //----------------------------------------------------------------------------------
                // Key index must be in range.
                if key.index as usize >= key_table.len() {
                    return Err(InvalidBinaryConfigData);
                }

                let key_string = unsafe { key_table.get_unchecked(key.index as usize) };

                // Key length must be positive.
                if key_string.len() == 0 {
                    return Err(InvalidBinaryConfigData);
                }

                // Make sure the key string and the null terminator lie within the config data blob (`+ 1`for null terminator).
                Self::validate_range(
                    valid_string_range.clone(),
                    key_string.offset()..key_string.offset() + key_string.len() + 1,
                )?;

                // Make sure the key string is null-terminated.
                let null_terminator = unsafe { table.slice(key_string.offset() + key_string.len(), 1) };

                if null_terminator[0] != b'\0' {
                    return Err(InvalidBinaryConfigData);
                }

                // Make sure the key string is valid UTF-8.
                let key_slice = unsafe { table.slice(key_string.offset(), key_string.len()) };

                let key_string =
                    std::str::from_utf8(key_slice).map_err(|_| InvalidBinaryConfigData)?;

                // Make sure the key hash matches the string.
                if string_hash_fnv1a(key_string) != key.hash {
                    return Err(InvalidBinaryConfigData);
                }
                //----------------------------------------------------------------------------------
                // The key seems to be OK.

                // Validate the value.
                Self::validate_value(
                    data,
                    key_table_offset,
                    key_table,
                    &mut valid_range,
                    valid_string_range.clone(),
                    table,
                    value,
                )?;
                // The value seems to be OK.
            }
        }

        Ok(())
    }

    fn validate_array(
        data: &[u8],
        key_table_offset: u32,
        valid_range_end: u32,
        array: &BinArrayOrTable<'_>,
    ) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        // Empty arrays must have no data offset.
        if array.len == 0 && array.offset != 0 {
            return Err(InvalidBinaryConfigData);
        }

        let key_table = unsafe { array.key_table() };
        let key_table_size = array.key_table_size();

        // Valid offset range for array values.
        // Minimum offset to string/array/table values in the array is just past the array's packed values.
        // Maximum offset is just before the key table and the shortest string section.
        let mut valid_range = array.offset_range().end..valid_range_end;

        // Valid offset range for strings.
        let valid_string_range = key_table_offset + key_table_size..data.len() as u32;

        let mut array_type: Option<ValueType> = None;

        // For each array element.
        for index in 0..array.len {
            let value = unsafe { array.packed_value(index) };

            let value_type = value.value_type();

            if let Some(current_array_type) = array_type {
                if !current_array_type.is_compatible(value_type) {
                    return Err(InvalidBinaryConfigData);
                } else {
                    array_type.replace(value_type);
                }
            }

            // All values in the array must have no keys.
            let key = value.key();

            if key.hash != 0 || key.index != 0 {
                return Err(InvalidBinaryConfigData);
            }

            // Validate the value.
            Self::validate_value(
                data,
                key_table_offset,
                key_table,
                &mut valid_range,
                valid_string_range.clone(),
                array,
                value,
            )?;
            // The value seems to be OK.
        }

        Ok(())
    }

    fn validate_range(
        valid_range: std::ops::Range<u32>,
        range: std::ops::Range<u32>,
    ) -> Result<(), BinConfigError> {
        use BinConfigError::*;

        if range.start < valid_range.start {
            return Err(InvalidBinaryConfigData);
        }

        if range.end > valid_range.end {
            return Err(InvalidBinaryConfigData);
        }

        Ok(())
    }

    fn validate_value(
        data: &[u8],
        key_table_offset: u32,
        key_table: &[InternedString],
        valid_range: &mut std::ops::Range<u32>, // Valid range of offsets within the binary data blob for this array/table value.
        valid_string_range: std::ops::Range<u32>, // Valid range of offsets within the binary data blob for strings.
        array_or_table: &BinArrayOrTable<'_>,     // Validated value's parent array/table.
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
                        valid_string_range,
                        value.offset()..value.offset() + value.len() + 1,
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
                // Non-empty arrays/tables have a positive offset to data.
                if value.len() > 0 {
                    let array_or_table =
                        BinArrayOrTable::new(data.as_ptr(), key_table, value.offset(), value.len());

                    // Make sure the array/table slice lies within the config data blob.
                    Self::validate_range(valid_range.clone(), array_or_table.offset_range())?;

                    // Validate the array/table values.
                    match value_type {
                        ValueType::Array => {
                            Self::validate_array(
                                data,
                                key_table_offset,
                                valid_range.end,
                                &array_or_table,
                            )?;
                        }
                        ValueType::Table => {
                            Self::validate_table(data, key_table_offset, &array_or_table)?;
                        }
                        _ => unreachable!(),
                    }

                    valid_range.end += size_of::<BinConfigPackedValue>() as u32;

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
    fn value_to_dyn_table<'k>(
        key: NonEmptyStr<'k>,
        value: BinConfigValue<'_>,
        dyn_table: &mut DynTable,
    ) {
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

const BIN_CONFIG_HEADER_MAGIC: u32 = 0x67666362; // `bcfg`, little endian.

/// Binary config data blob header.
///
/// Fields are in whatever endianness we use; see `super::util::__to_bin_bytes(), _from_bin()`.
#[repr(C, packed)]
pub(super) struct BinConfigHeader {
    /// Arbitrary magic value for a quick sanity check.
    magic: u32,
    /// Followed by the root table length.
    len: u32,
    /// Offset in bytes to the start of the key string table.
    /// Each element is an `InternedString` - a key string's offset and length
    /// in the string section.
    key_table_offset: u32,
    /// Length of the key string table in elements.
    key_table_len: u32,
}

impl BinConfigHeader {
    fn check_magic(&self) -> bool {
        u32_from_bin(self.magic) == BIN_CONFIG_HEADER_MAGIC
    }

    pub(super) fn len(&self) -> u32 {
        u32_from_bin(self.len)
    }

    pub(super) fn key_table_offset(&self) -> u32 {
        u32_from_bin(self.key_table_offset)
    }

    pub(super) fn key_table_len(&self) -> u32 {
        u32_from_bin(self.key_table_len)
    }

    pub(super) fn key_table_size(&self) -> u32 {
        self.key_table_len() * size_of::<InternedString>() as u32
    }

    /// Returns the range of bytes within the binary config data blob
    /// occupied by the key table.
    pub(super) fn key_table_range(&self) -> std::ops::Range<u32> {
        let offset = self.key_table_offset();
        offset..offset + self.key_table_size()
    }

    pub(super) fn write<W: Write>(
        writer: &mut W,
        len: u32,
        key_table_offset: u32,
        key_table_len: u32,
    ) -> Result<u32, BinConfigWriterError> {
        use BinConfigWriterError::*;

        let mut written = 0;

        // Magic.
        written += writer
            .write(&u32_to_bin_bytes(BIN_CONFIG_HEADER_MAGIC))
            .map_err(|_| WriteError)?;

        // Root table length.
        written += writer
            .write(&u32_to_bin_bytes(len))
            .map_err(|_| WriteError)?;

        written += writer
            .write(&u32_to_bin_bytes(key_table_offset))
            .map_err(|_| WriteError)?;

        written += writer
            .write(&u32_to_bin_bytes(key_table_len))
            .map_err(|_| WriteError)?;

        Ok(written as u32)
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

        assert_eq!(
            config.root().get_string("liquid".into()).unwrap(),
            "macallums"
        );
        assert_eq!(
            config.root().get_string("costarring".into()).unwrap(),
            "declinate"
        );
        assert_eq!(config.root().get_string_str("liquid").unwrap(), "macallums");
        assert_eq!(
            config.root().get_string_str("costarring").unwrap(),
            "declinate"
        );

        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_string(key!("liquid")).unwrap(),
                "macallums"
            );
            assert_eq!(
                config.root().get_string(key!("costarring")).unwrap(),
                "declinate"
            );
        }
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
