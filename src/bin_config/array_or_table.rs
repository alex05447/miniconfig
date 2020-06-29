use {
    super::value::{BinConfigPackedValue, BinConfigUnpackedValue, BinTableKey},
    std::{marker::PhantomData, mem::size_of},
};

/// Represents a binary array/table, as unpacked from the binary config data blob.
#[derive(Clone)]
pub(super) struct BinArrayOrTable<'at> {
    /// Base address of the binary config data blob on the heap w.r.t. which all values specify their offsets.
    pub(super) base: *const u8,
    /// Offset in bytes to the first packed value of this array/table.
    /// Must be `0` if `len == 0`.
    pub(super) offset: u32,
    /// Number of elements in this array/table.
    pub(super) len: u32,
    /// Represents an immutable borrow into the binary config data blob.
    _marker: PhantomData<&'at ()>,
}

impl<'at> BinArrayOrTable<'at> {
    pub(super) fn new(base: *const u8, offset: u32, len: u32) -> Self {
        Self {
            base,
            offset,
            len,
            _marker: PhantomData,
        }
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

    /// Reads and returns an unpacked value at `index` of this array/table.
    /// NOTE - the caller ensures the array/table is not empty and `index` is in range.
    pub(super) unsafe fn value(&self, index: u32) -> BinConfigUnpackedValue {
        self.packed_value(index).unpack()
    }

    /// Reads and returns a table key and an unpacked value at `index` of this array/table.
    /// NOTE - the caller ensures it's a table, it's not empty and `index` is in range.
    pub(super) unsafe fn key_and_value(&self, index: u32) -> (BinTableKey, BinConfigUnpackedValue) {
        let packed = self.packed_value(index);
        (packed.key(), packed.unpack())
    }

    /// Returns the byte slice in the binary config data blob at `offset` with length `len`.
    /// NOTE - the caller ensures `offset` and `len` are valid.
    pub(super) unsafe fn slice(&self, offset: u32, len: u32) -> &'at [u8] {
        std::slice::from_raw_parts(self.base.offset(offset as isize), len as usize)
    }

    /// Returns the UTF-8 string slice in the binary config data blob at `offset` with length `len`.
    /// NOTE - the caller ensures `offset` and `len` are valid and that the string contains valid UTF-8.
    pub(super) unsafe fn string(&self, offset: u32, len: u32) -> &'at str {
        std::str::from_utf8_unchecked(self.slice(offset, len))
    }

    pub(super) fn offset_range(&self) -> (u32, u32) {
        (
            self.offset,
            self.offset + self.len * size_of::<BinConfigPackedValue>() as u32,
        )
    }
}
