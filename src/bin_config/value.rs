use {
    super::{
        array::BinArray,
        table::BinTable,
        util::{u32_from_bin, u32_to_bin, u64_from_bin, u64_to_bin},
    },
    crate::{
        value::{value_type_from_u32, value_type_to_u32},
        BinArrayError, ConfigKey, ConfigPath, DisplayLua, GetPathError, TableError, Value,
        ValueType,
    },
    static_assertions::const_assert,
    std::{
        borrow::Borrow,
        fmt::{Display, Formatter},
        io::Write,
    },
};

// | value type |    key length, if any   |
// |-- 4 bits --|-------- 28 bits --------|

// Number of bits in `type_and_key_len` we use for table key length.
const KEY_LEN_BITS: u32 = 28;

const KEY_LEN_OFFSET: u32 = 0;

// Read/write mask for key length bits in `type_and_key_len`.
const KEY_LEN_MASK: u32 = ((1 << KEY_LEN_BITS) - 1) << KEY_LEN_OFFSET;

// Maximum table key string length determined by the number of bits in `type_and_key_len` we use for it.
const MAX_KEY_LEN: u32 = KEY_LEN_MASK >> KEY_LEN_OFFSET;

// Number of bits in `type_and_key_len` we use for value type.
const TYPE_BITS: u32 = 4;

const TYPE_OFFSET: u32 = KEY_LEN_OFFSET + KEY_LEN_BITS;

// Read/write mask for type bits in `type_and_key_len`.
const TYPE_MASK: u32 = ((1 << TYPE_BITS) - 1) << TYPE_OFFSET;

const_assert!(KEY_LEN_BITS + TYPE_BITS == (std::mem::size_of::<u32>() as u32) * 8);

// |--   offset   --|--   length   --|
// |--   32 bits  --|--   32 bits  --|

// Number of bits in `value_or_offset_and_len` we use for value length.
const VALUE_LEN_BITS: u64 = 32;

const VALUE_LEN_OFFSET: u64 = 0;

// Read/write mask for value length bits in `value_or_offset_and_len`.
const VALUE_LEN_MASK: u64 = ((1 << VALUE_LEN_BITS) - 1) << VALUE_LEN_OFFSET;

// Number of bits in `value_or_offset_and_len` we use for value offset.
const VALUE_OFFSET_BITS: u64 = 32;

const VALUE_OFFSET_OFFSET: u64 = VALUE_LEN_BITS;

// Read/write mask for value offset bits in `value_or_offset_and_len`.
const VALUE_OFFSET_MASK: u64 = ((1 << VALUE_OFFSET_BITS) - 1) << VALUE_OFFSET_OFFSET;

/// Represents a single config value as stored in the binary config data blob.
///
/// Fields are in whatever endianness we use; see `super::util::__to_bin_bytes(), _from_bin()`.
#[repr(C, packed)]
pub(crate) struct BinConfigPackedValue {
    /// Config value type and, if it's a table element, key string length, packed.
    /// | value type |    key length, if any   |
    /// |-- 4 bits --|-------- 28 bits --------|
    type_and_key_len: u32,

    /// For table elements - key string hash (for quick comparison) and offset.
    key_hash: u32,
    key_offset: u32,

    /// `Bool`, `I64`, `F64` values are stored here directly, using 8 bytes.
    /// |---- bool / i64 / f64 ----|
    /// |----      64 bits     ----|
    /// `String`, `Array` and `Table` values of `u32` length are stored separately at a `u32` offset.
    /// |--   offset   --|--   length   --|
    /// |--   32 bits  --|--   32 bits  --|
    value_or_offset_and_len: u64,
}

impl BinConfigPackedValue {
    /// Create a new packed value representing a `bool`.
    pub(super) fn new_bool(key: BinTableKey, value: bool) -> Self {
        let mut result = Self::default();

        result.set_value_type(ValueType::Bool);
        result.set_key(key);
        result.set_value_or_offset_and_len(if value { 1 } else { 0 });

        result
    }

    /// Create a new packed value representing an `i64`.
    pub(super) fn new_i64(key: BinTableKey, value: i64) -> Self {
        let mut result = Self::default();

        result.set_value_type(ValueType::I64);
        result.set_key(key);
        result.set_value_or_offset_and_len(unsafe { std::mem::transmute(value) });

        result
    }

    /// Create a new packed value representing an `f64`.
    pub(super) fn new_f64(key: BinTableKey, value: f64) -> Self {
        let mut result = Self::default();

        result.set_value_type(ValueType::F64);
        result.set_key(key);
        result.set_value_or_offset_and_len(value.to_bits());

        result
    }

    /// Create a new packed value representing a string.
    pub(super) fn new_string(key: BinTableKey, offset: u32, len: u32) -> Self {
        let mut result = Self::default();

        result.set_value_type(ValueType::String);
        result.set_key(key);
        result.set_offset(offset);
        result.set_len(len);

        result
    }

    /// Create a new packed value representing an array / table.
    pub(super) fn new_array_or_table(key: BinTableKey, offset: u32, len: u32, table: bool) -> Self {
        let mut result = Self::default();

        result.set_value_type(if table {
            ValueType::Table
        } else {
            ValueType::Array
        });
        result.set_key(key);
        result.set_offset(if len == 0 { 0 } else { offset });
        result.set_len(len);

        result
    }

    /// Creates a default (invalid) packed value.
    fn default() -> Self {
        Self {
            type_and_key_len: 0,
            key_hash: 0,
            key_offset: 0,
            value_or_offset_and_len: 0,
        }
    }

    /// Serialize the packed value to the writer.
    pub(crate) fn write<W: Write>(&self, writer: &mut W) -> std::io::Result<u32> {
        // NOTE - all fields are already packed in correct endianness, so use `to_ne_bytes()`.
        let mut written = writer.write(&self.type_and_key_len.to_ne_bytes())? as u32;
        written += writer.write(&self.key_hash.to_ne_bytes())? as u32;
        written += writer.write(&self.key_offset.to_ne_bytes())? as u32;
        written += writer.write(&self.value_or_offset_and_len.to_ne_bytes())? as u32;

        Ok(written)
    }

    /// Unpacks and returns the table element's key hash, offset and length.
    /// NOTE - the caller ensures this value is a table element.
    pub(super) fn key(&self) -> BinTableKey {
        BinTableKey {
            hash: self.key_hash(),
            offset: self.key_offset(),
            len: self.key_len(),
        }
    }

    /// Packs the value's key hash, offset and length.
    /// NOTE - the caller ensures this value is a table element.
    pub(super) fn set_key(&mut self, key: BinTableKey) {
        debug_assert!(key.len <= MAX_KEY_LEN);

        let mut type_and_key_len = self.type_and_key_len();

        // Keep the type, overwrite the key length.
        type_and_key_len = (type_and_key_len & TYPE_MASK) | (key.len & KEY_LEN_MASK);

        self.set_type_and_key_len(type_and_key_len);

        self.key_hash = u32_to_bin(key.hash);
        self.key_offset = u32_to_bin(key.offset);
    }

    /// Unpacks this value to an intermediate representation for native endianness and a concrete value type.
    /// NOTE - the caller ensures the value is valid.
    pub(super) fn unpack(&self) -> BinConfigUnpackedValue {
        use BinConfigUnpackedValue::*;

        match self.value_type() {
            ValueType::Bool => Bool(self.bool()),
            ValueType::I64 => I64(self.i64()),
            ValueType::F64 => F64(self.f64()),
            ValueType::String => BinConfigUnpackedValue::String {
                offset: self.offset(),
                len: self.len(),
            },
            ValueType::Array => Array {
                offset: self.offset(),
                len: self.len(),
            },
            ValueType::Table => Table {
                offset: self.offset(),
                len: self.len(),
            },
        }
    }

    /// Tries to unpack and load this value's type.
    /// Fails if it's not a valid value type.
    pub(super) fn try_value_type(&self) -> Option<ValueType> {
        let value_type = (self.type_and_key_len() & TYPE_MASK) >> TYPE_OFFSET;

        value_type_from_u32(value_type)
    }

    /// Unpacks this value's type.
    /// NOTE - panics if it's not a valid value type.
    pub(super) fn value_type(&self) -> ValueType {
        self.try_value_type()
            .expect("invalid binary config value type")
    }

    /// Packs the value type.
    fn set_value_type(&mut self, value_type: ValueType) {
        let mut type_and_key_len = self.type_and_key_len();

        // Keep the key length, overwrite the value type.
        type_and_key_len = ((value_type_to_u32(Some(value_type)) << TYPE_OFFSET) & TYPE_MASK)
            | (type_and_key_len & KEY_LEN_MASK);

        self.set_type_and_key_len(type_and_key_len);
    }

    /// Tries to unpack and interpret this value as a `bool`.
    /// Fails if it's not `0` or `1`.
    pub(super) fn try_bool(&self) -> Option<bool> {
        let value = self.value_or_offset_and_len();

        match value {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    /// Unpacks and interprets this value as a `bool`.
    /// NOTE - panics if value is not `0` or `1`.
    fn bool(&self) -> bool {
        self.try_bool()
            .expect("invalid binary config boolean value")
    }

    /// Unpacks and interprets this value as an `i64`.
    /// NOTE - the caller ensures the value is actually an `i64`.
    fn i64(&self) -> i64 {
        unsafe { std::mem::transmute(self.value_or_offset_and_len()) }
    }

    /// Unpacks and interprets this value as an `f64`.
    /// NOTE - the caller ensures the value is actually an `f64`.
    fn f64(&self) -> f64 {
        f64::from_bits(self.value_or_offset_and_len())
    }

    /// Unpacks and returns the string/array/table length.
    /// String length is in bytes; array/table length is in elements.
    /// NOTE - the caller ensures this value is a string/array/table.
    pub(super) fn len(&self) -> u32 {
        ((self.value_or_offset_and_len() & VALUE_LEN_MASK) >> VALUE_LEN_OFFSET) as u32
    }

    /// Packs the string/array/table value's length.
    /// String length is in bytes; array/table length is in elements.
    /// NOTE - the caller ensures this value is a string/array/table.
    fn set_len(&mut self, len: u32) {
        let mut value_or_offset_and_len = self.value_or_offset_and_len();

        // Keep the offset, overwrite the length.
        value_or_offset_and_len =
            (value_or_offset_and_len & VALUE_OFFSET_MASK) | (len as u64 & VALUE_LEN_MASK);

        self.set_value_or_offset_and_len(value_or_offset_and_len);
    }

    /// Unpacks and interprets this value as an offset to string / array / table data
    /// and returns the offset.
    /// NOTE - the caller ensures the value is a string / array / table.
    pub(super) fn offset(&self) -> u32 {
        ((self.value_or_offset_and_len() & VALUE_OFFSET_MASK) >> VALUE_OFFSET_OFFSET) as u32
    }

    /// Packs the offset to string / array / table data.
    /// NOTE - the caller ensures the value is a string / array / table.
    pub(super) fn set_offset(&mut self, offset: u32) {
        let mut value_or_offset_and_len = self.value_or_offset_and_len();

        // Keep the length, overwrite the offset.
        value_or_offset_and_len = (((offset as u64) << VALUE_OFFSET_OFFSET) & VALUE_OFFSET_MASK)
            | (value_or_offset_and_len & VALUE_LEN_MASK);

        self.set_value_or_offset_and_len(value_or_offset_and_len);
    }

    /// Unpacks this value's type/key length to `u32`.
    fn type_and_key_len(&self) -> u32 {
        u32_from_bin(self.type_and_key_len)
    }

    /// Packs the value's type/key length.
    fn set_type_and_key_len(&mut self, type_and_key_len: u32) {
        self.type_and_key_len = u32_to_bin(type_and_key_len);
    }

    /// Unpacks this value's value/offset and length to `u64`.
    fn value_or_offset_and_len(&self) -> u64 {
        u64_from_bin(self.value_or_offset_and_len)
    }

    /// Packs the value's value/offset and length
    fn set_value_or_offset_and_len(&mut self, value_or_offset_and_len: u64) {
        self.value_or_offset_and_len = u64_to_bin(value_or_offset_and_len);
    }

    /// Unpacks the key hash to `u32`.
    fn key_hash(&self) -> u32 {
        u32_from_bin(self.key_hash)
    }

    /// Unpacks the key offset to `u32`.
    fn key_offset(&self) -> u32 {
        u32_from_bin(self.key_offset)
    }

    /// Unpacks the key length to `u32`.
    fn key_len(&self) -> u32 {
        (self.type_and_key_len() & KEY_LEN_MASK) >> KEY_LEN_OFFSET
    }
}

/// Each value in the table has a hashed UTF-8 string key,
/// described by this struct.
/// NOTE - only table elements have string keys.
pub(super) struct BinTableKey {
    /// FNV-1a hash of the key.
    pub(super) hash: u32,
    /// Offset to the key string in the config data blob.
    pub(super) offset: u32,
    /// Length of the key string in bytes.
    pub(super) len: u32,
}

impl BinTableKey {
    pub(crate) fn max_len() -> u32 {
        MAX_KEY_LEN
    }

    pub(crate) fn new(hash: u32, offset: u32, len: u32) -> Self {
        debug_assert!(len <= Self::max_len(),);

        Self { hash, offset, len }
    }
}

impl Default for BinTableKey {
    fn default() -> Self {
        Self {
            hash: 0,
            offset: 0,
            len: 0,
        }
    }
}

/// Represents a single config value as unpacked for native endianness and a concrete value type.
/// It's a final representation for bools / ints / floats;
/// an intermediate representation for strings / arrays / tables,
/// which reference other data in the binary config data blob.
pub(super) enum BinConfigUnpackedValue {
    Bool(bool),
    I64(i64),
    F64(f64),
    String { offset: u32, len: u32 },
    Array { offset: u32, len: u32 },
    Table { offset: u32, len: u32 },
}

/// A [`value`] returned when accessing a binary [`array`] or [`table`].
///
/// [`value`]: enum.Value.html
/// [`array`]: struct.BinArray.html
/// [`table`]: struct.BinTable.html
pub type BinConfigValue<'at> = Value<&'at str, BinArray<'at>, BinTable<'at>>;

impl<'at> BinConfigValue<'at> {
    pub(crate) fn get_path<'a, K, P>(self, mut path: P) -> Result<Self, GetPathError<'a>>
    where
        K: Borrow<ConfigKey<'a>>,
        P: Iterator<Item = K>,
    {
        if let Some(key) = path.next() {
            let key = key.borrow();
            match key {
                ConfigKey::Array(index) => match self {
                    Value::Array(array) => {
                        let value = array.get(*index).map_err(|err| match err {
                            BinArrayError::IndexOutOfBounds(len) => {
                                GetPathError::IndexOutOfBounds {
                                    path: ConfigPath::from_key(key.clone()),
                                    len,
                                }
                            }
                            BinArrayError::IncorrectValueType(_) => unreachable!(),
                        })?;

                        value.get_path(path).map_err(|err| err.push_key(key))
                    }
                    _ => Err(GetPathError::ValueNotAnArray {
                        path: ConfigPath::new(),
                        value_type: self.get_type(),
                    }),
                },
                ConfigKey::Table(ref table_key) => match self {
                    Value::Table(table) => {
                        let value = table.get(table_key).map_err(|err| match err {
                            TableError::EmptyKey => GetPathError::EmptyKey(ConfigPath::new()),
                            TableError::KeyDoesNotExist => {
                                GetPathError::KeyDoesNotExist(ConfigPath::from_key(key.clone()))
                            }
                            TableError::IncorrectValueType(_) => unreachable!(),
                        })?;

                        value.get_path(path).map_err(|err| err.push_key(key))
                    }
                    _ => Err(GetPathError::ValueNotATable {
                        path: ConfigPath::new(),
                        value_type: self.get_type(),
                    }),
                },
            }
        } else {
            Ok(self)
        }
    }
}

impl<'a> Display for BinConfigValue<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_lua(f, 0)
    }
}
