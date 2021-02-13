use {
    super::{value::{BinConfigPackedValue, BinConfigUnpackedValue, BinTableKey}, util::u32_from_bin},
    std::{mem::size_of, slice::from_raw_parts},
};

/// Represents an interned UTF-8 string in the string section of the binary config.
///
/// Fields are in whatever endianness we use; see `super::util::__to_bin_bytes(), _from_bin()`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(C, packed)]
pub(super) struct InternedString {
    /// Offset in bytes to the string w.r.t. the binary config data blob.
    /// `0` if the string is empty (`len` is `0`).
    pub(super) offset: u32,
    /// String length in bytes.
    pub(super) len: u32,
}

impl InternedString {
    pub(super) fn offset(&self) -> u32 {
        u32_from_bin(self.offset)
    }

    pub(super) fn len(&self) -> u32 {
        u32_from_bin(self.len)
    }
}

/// Represents a binary array/table, as unpacked from the binary config data blob.
#[derive(Clone)]
pub(super) struct BinArrayOrTable<'at> {
    /// Base address of the binary config data blob on the heap w.r.t. which all values specify their offsets.
    pub(super) base: *const u8,
    /// Reference to the key table slice in the config data blob.
    /// Used for table key string lookups.
    pub(super) key_table: &'at [InternedString],
    /// Offset in bytes to the first packed value of this array/table.
    /// Must be `0` if `len == 0`.
    pub(super) offset: u32,
    /// Number of elements in this array/table.
    pub(super) len: u32,
}

impl<'at> BinArrayOrTable<'at> {
    pub(super) fn new(
        base: *const u8,
        key_table: &'at [InternedString],
        offset: u32,
        len: u32,
    ) -> Self {
        Self {
            base,
            key_table,
            offset,
            len,
        }
    }

    /// Returns the range of bytes within the binary config data blob
    /// occupied by the packed values of this array / table.
    pub(super) fn offset_range(&self) -> std::ops::Range<u32> {
        self.offset..self.offset + self.len * size_of::<BinConfigPackedValue>() as u32
    }

    pub(super) unsafe fn key_table(&self) -> &'at [InternedString] {
        self.key_table
    }

    pub(super) fn key_table_size(&self) -> u32 {
        (self.key_table.len() * size_of::<InternedString>()) as u32
    }

    /// Reads and returns an unpacked value at `index` of this array/table.
    /// NOTE - the caller ensures the array/table is not empty and `index` is in range.
    pub(super) unsafe fn value(&self, index: u32) -> BinConfigUnpackedValue {
        self.packed_value(index).unpack()
    }

    /// Returns a reference to the packed value at `index` of this array/table
    /// in the binary config data blob.
    /// NOTE - the caller ensures the array/table is not empty and `index` is in range.
    pub(super) unsafe fn packed_value(&self, index: u32) -> &'at BinConfigPackedValue {
        &*(self.base.offset(self.packed_value_offset(index) as isize) as *const _)
    }

    /// Returns an offset in bytes to the packed value at `index` of this array/table
    /// in the binary config data blob.
    /// NOTE - the caller ensures the array/table is not empty and `index` is in range.
    pub(super) unsafe fn packed_value_offset(&self, index: u32) -> u32 {
        debug_assert!(index < self.len, "`index` must be in range.");

        self.offset + index * size_of::<BinConfigPackedValue>() as u32
    }

    /// Returns the byte slice in the binary config data blob at `offset` with length `len`.
    /// NOTE - the caller ensures `offset` and `len` are valid.
    pub(super) unsafe fn slice(&self, offset: u32, len: u32) -> &'at [u8] {
        from_raw_parts(self.base.offset(offset as isize), len as usize)
    }

    /// Returns the UTF-8 string slice in the binary config data blob at `offset` with length `len`.
    /// NOTE - the caller ensures `offset` and `len` are valid and that the string contains valid UTF-8.
    pub(super) unsafe fn string(&self, offset: u32, len: u32) -> &'at str {
        std::str::from_utf8_unchecked(self.slice(offset, len))
    }

    /// Looks up the key table with `index`.
    /// NOTE - the caller ensures `index` is valid.
    pub(super) unsafe fn key_ofset_and_len(&self, index: u32) -> &InternedString {
        let key_table = self.key_table();
        let index = index as usize;

        debug_assert!(index < self.key_table().len(), "`index` must be in range.");

        key_table.get_unchecked(index)
    }

    /// Reads and returns a table key and an unpacked value at `index` of this array/table.
    /// NOTE - the caller ensures it's a table, it's not empty and `index` is in range.
    pub(super) unsafe fn key_and_value(&self, index: u32) -> (BinTableKey, BinConfigUnpackedValue) {
        let packed = self.packed_value(index);
        (packed.key(), packed.unpack())
    }
}
