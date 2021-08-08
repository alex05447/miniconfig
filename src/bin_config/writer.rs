use {
    super::{
        array_or_table::InternedString,
        config::BinConfigHeader,
        util::{string_hash_fnv1a, StringHash},
        value::{BinConfigPackedValue, BinTableKey, StringIndex},
    },
    crate::{util::unwrap_unchecked, BinConfigWriterError, ValueType},
    std::{
        collections::{hash_map::Entry, HashMap},
        io::{Cursor, Seek, SeekFrom, Write},
        mem::size_of,
        str::from_utf8_unchecked,
    },
};

/// Represents a UTF-8 string interned by the binary config writer.
/// Looked up by its hash.
#[derive(Clone, Copy, PartialEq, Eq)]
struct BinConfigWriterString {
    /// The string's offset and length in the string section.
    offset_and_len: InternedString,
    /// If the string was interned as a key string at least once, it has an entry in the key table
    /// and this is its index in it.
    /// `None` if the string was never used as a key.
    index: Option<StringIndex>,
}

/// Provides an interface for recording of [`binary configs`].
///
/// [`binary configs`]: struct.BinConfig.html
pub struct BinConfigWriter {
    /// Offset in bytes to the string section of the binary config data blob.
    data_offset: u32,
    /// Binary config data blob writer.
    config_writer: Cursor<Vec<u8>>,
    /// Maps string hashes to their string section entries / indices in the key table.
    strings: HashMap<StringHash, Vec<BinConfigWriterString>>,
    /// Contains key string offsets and lengths in bytes for all interned key strings.
    /// NOTE - offsets are w.r.t. the string section during recording,
    /// then fixed up to full offsets w.r.t. the data blob when the recording is finished.
    key_table: Vec<InternedString>,
    /// Binary config string section writer.
    /// Contains UTF-8 strings.
    /// NOTE - strings are null-terminated just in case.
    string_writer: Vec<u8>,
    /// LIFO stack which contains the root table and any nested arrays/tables during recording.
    stack: Vec<BinConfigArrayOrTable>,
}

impl BinConfigWriter {
    /// Creates a new [`binary config`] [`writer`] with the root [`table`] with `len` elements.
    ///
    /// [`binary config`]: struct.BinConfig.html
    /// [`writer`]: struct.BinConfigWriter.html
    /// [`table`]: struct.BinTable.html
    pub fn new(len: u32) -> Result<Self, BinConfigWriterError> {
        use BinConfigWriterError::*;

        // Empty root tables are not supported.
        if len == 0 {
            return Err(EmptyRootTable);
        }

        let mut writer = Self {
            data_offset: 0,
            config_writer: Cursor::new(Vec::new()),
            strings: HashMap::new(),
            key_table: Vec::new(),
            string_writer: Vec::new(),
            stack: Vec::new(),
        };

        // Write the config header / root table length, prepare to receive root table elements.
        writer.root(len)?;

        Ok(writer)
    }

    /// Writes a `bool` value to the current [`array`] / [`table`] (including the root [`table`]).
    ///
    /// NOTE - a non-empty UTF-8 string `key` is required for a [`table`] element (including the root [`table`]).
    ///
    /// [`array`]: struct.BinArray.html
    /// [`table`]: struct.BinTable.html
    pub fn bool<'k, K: Into<Option<&'k str>>>(
        &mut self,
        key: K,
        value: bool,
    ) -> Result<(), BinConfigWriterError> {
        // Value's key and its offset in bytes.
        let (key, value_offset) = self.key_and_value_offset(key.into(), ValueType::Bool)?;

        // Write the packed value.
        Self::write_value(
            &mut self.config_writer,
            &mut self.stack,
            BinConfigPackedValue::new_bool(key, value),
            value_offset,
        )?;

        Ok(())
    }

    /// Writes an `i64` value to the current [`array`] / [`table`] (including the root [`table`]).
    ///
    /// NOTE - a non-empty string `key` is required for a [`table`] element (including the root [`table`]).
    ///
    /// [`array`]: struct.BinArray.html
    /// [`table`]: struct.BinTable.html
    pub fn i64<'k, K: Into<Option<&'k str>>>(
        &mut self,
        key: K,
        value: i64,
    ) -> Result<(), BinConfigWriterError> {
        // Value's key and its offset in bytes.
        let (key, value_offset) = self.key_and_value_offset(key.into(), ValueType::I64)?;

        // Write the packed value.
        Self::write_value(
            &mut self.config_writer,
            &mut self.stack,
            BinConfigPackedValue::new_i64(key, value),
            value_offset,
        )?;

        Ok(())
    }

    /// Writes an `f64` value to the current [`array`] / [`table`] (including the root [`table`]).
    ///
    /// NOTE - a non-empty string `key` is required for a [`table`] element (including the root [`table`]).
    ///
    /// [`array`]: struct.BinArray.html
    /// [`table`]: struct.BinTable.html
    pub fn f64<'k, K: Into<Option<&'k str>>>(
        &mut self,
        key: K,
        value: f64,
    ) -> Result<(), BinConfigWriterError> {
        // Value's key and its offset in bytes.
        let (key, value_offset) = self.key_and_value_offset(key.into(), ValueType::F64)?;

        // Write the packed value.
        Self::write_value(
            &mut self.config_writer,
            &mut self.stack,
            BinConfigPackedValue::new_f64(key, value),
            value_offset,
        )?;

        Ok(())
    }

    /// Writes a string value to the current [`array`] / [`table`] (including the root [`table`]).
    ///
    /// NOTE - a non-empty string `key` is required for a [`table`] element (including the root [`table`]).
    ///
    /// [`array`]: struct.BinArray.html
    /// [`table`]: struct.BinTable.html
    pub fn string<'k, K: Into<Option<&'k str>>>(
        &mut self,
        key: K,
        value: &str,
    ) -> Result<(), BinConfigWriterError> {
        // Value's key and its offset in bytes.
        let (key, value_offset) = self.key_and_value_offset(key.into(), ValueType::String)?;

        // Lookup or intern the string.
        let (_, string) =
            Self::intern_string(&mut self.strings, None, &mut self.string_writer, value)?;

        // Write the packed value.
        Self::write_value(
            &mut self.config_writer,
            &mut self.stack,
            BinConfigPackedValue::new_string(
                key,
                string.offset_and_len.offset,
                string.offset_and_len.len,
            ),
            value_offset,
        )?;

        Ok(())
    }

    /// Writes an array value with `len` elements to the current [`array`] / [`table`] (including the root [`table`])
    /// and makes it the active array for the next `len` calls to this [`writer`]'s methods.
    ///
    /// NOTE - a non-empty string `key` is required for a [`table`] element (including the root [`table`]).
    /// NOTE - [`end`] must be called after the last value is written to the array.
    ///
    /// [`array`]: struct.BinArray.html
    /// [`table`]: struct.BinTable.html
    /// [`writer`]: struct.BinConfigWriter.html
    /// [`end`]: #method.end
    pub fn array<'k, K: Into<Option<&'k str>>>(
        &mut self,
        key: K,
        len: u32,
    ) -> Result<(), BinConfigWriterError> {
        self.array_or_table(key.into(), len, false)
    }

    /// Writes a table value with `len` elements to the current [`array`] / [`table`] (including the root [`table`])
    /// and makes it the active table for the next `len` calls to this [`writer`]'s methods.
    ///
    /// NOTE - a non-empty string `key` is required for a [`table`] element (including the root [`table`]).
    /// NOTE - [`end`] must be called after the last value is written to the table.
    ///
    /// [`array`]: struct.BinArray.html
    /// [`table`]: struct.BinTable.html
    /// [`writer`]: struct.BinConfigWriter.html
    /// [`end`]: #method.end
    pub fn table<'k, K: Into<Option<&'k str>>>(
        &mut self,
        key: K,
        len: u32,
    ) -> Result<(), BinConfigWriterError> {
        self.array_or_table(key.into(), len, true)
    }

    /// Ends the recording of the previous [`array`] / [`table`].
    ///
    /// [`array`]: #method.array
    /// [`table`]: #method.table
    pub fn end(&mut self) -> Result<(), BinConfigWriterError> {
        use BinConfigWriterError::*;

        // Must have an array/non-root table on the stack (excluding the root table).
        if self.stack.len() < 2 {
            return Err(EndCallMismatch);
        }

        // Must succeed - stack is not empty.
        let parent = unwrap_unchecked(self.stack.pop());
        let (len, cur_len) = (parent.len, parent.current_len);

        // Must have been full.
        if cur_len != len {
            self.stack.push(parent);

            return Err(ArrayOrTableLengthMismatch {
                expected: len,
                found: cur_len,
            });
        }

        Ok(())
    }

    /// Consumes this [`writer`] and returns the finished [`binary config`] data blob.
    ///
    /// [`writer`]: struct.BinConfigWriter.html
    /// [`binary config`]: struct.BinConfig.html
    pub fn finish(mut self) -> Result<Box<[u8]>, BinConfigWriterError> {
        use BinConfigWriterError::*;

        std::mem::drop(self.strings);

        debug_assert_ne!(self.stack.len(), 0);

        // Must only have the root table on the stack.
        if self.stack.len() > 1 {
            return Err(UnfinishedArraysOrTables(self.stack.len() as u32 - 1));
        }

        // Must succeed - stack is not empty.
        let root = unwrap_unchecked(self.stack.pop());

        // The root table must have been full.
        if root.current_len < root.len {
            return Err(ArrayOrTableLengthMismatch {
                expected: root.len,
                found: root.current_len,
            });
        };

        // Fixup the header with correct key table offset and length.
        let key_table_offset = self.data_offset;
        let key_table_len = self.key_table.len() as u32;

        self.config_writer
            .seek(SeekFrom::Start(0))
            .map_err(|_| WriteError)?;

        BinConfigHeader::write(
            &mut self.config_writer,
            root.len,
            key_table_offset,
            key_table_len,
        )?;

        // Fixup the key table offsets.
        let key_table_size = key_table_len * size_of::<InternedString>() as u32;

        let string_offset = self.data_offset + key_table_size as u32;

        for entry in self.key_table.iter_mut() {
            entry.offset += string_offset;
        }

        // Append the key table to the end of the buffer.
        let mut config_writer = self.config_writer.into_inner();

        let key_table_bytes = unsafe {
            std::slice::from_raw_parts(self.key_table.as_ptr() as *const u8, key_table_size as _)
        };

        config_writer.extend_from_slice(key_table_bytes);
        std::mem::drop(self.key_table);

        // Append the strings to the end of the buffer.
        config_writer.append(&mut self.string_writer);
        config_writer.shrink_to_fit();
        std::mem::drop(self.string_writer);

        // Fixup the string offsets in all entries using them
        // via incrementing them by the now-known data offset.
        let mut data = config_writer.into_boxed_slice();

        Self::fixup_string_offsets(&mut data, string_offset);

        Ok(data)
    }

    /// Called once on construction.
    /// Writes the binary config data blob header / root table length, initializes the data offset.
    fn root(&mut self, len: u32) -> Result<(), BinConfigWriterError> {
        debug_assert_eq!(self.stack.len(), 0);
        debug_assert_eq!(self.data_offset, 0);

        // Write the header.
        self.data_offset += BinConfigHeader::write(
            &mut self.config_writer,
            len,
            0, // NOTE - fixed up when the recording is finished.
            0, // NOTE - fixed up when the recording is finished.
        )?;

        // Push the root table on the stack.
        self.stack
            .push(BinConfigArrayOrTable::new(true, len, self.data_offset));

        // Bump the data offset by the combined table value length.
        self.data_offset += len * size_of::<BinConfigPackedValue>() as u32;

        Ok(())
    }

    fn array_or_table(
        &mut self,
        key: Option<&str>,
        len: u32,
        table: bool,
    ) -> Result<(), BinConfigWriterError> {
        // Offset to the array's/table's packed value is the parent array's/table's value offset.
        let (key, value_offset) = self.key_and_value_offset(
            key,
            if table {
                ValueType::Table
            } else {
                ValueType::Array
            },
        )?;

        // Write the packed value.
        // Offset to the array's/table's values is the current data offset.
        Self::write_value(
            &mut self.config_writer,
            &mut self.stack,
            BinConfigPackedValue::new_array_or_table(key, self.data_offset, len, table),
            value_offset,
        )?;

        // Push the array/table on the stack.
        self.stack
            .push(BinConfigArrayOrTable::new(table, len, self.data_offset));

        // Bump the data offset by the combined value length.
        self.data_offset += len * size_of::<BinConfigPackedValue>() as u32;

        Ok(())
    }

    /// For tables, looks up/interns the required `key` string
    /// and returns its hash / index in the key table.
    /// For arrays returns a default key.
    fn key(
        strings: &mut HashMap<StringHash, Vec<BinConfigWriterString>>,
        key_table: &mut Vec<InternedString>,
        string_writer: &mut Vec<u8>,
        parent_table: Option<&mut BinConfigArrayOrTable>,
        key: Option<&str>,
    ) -> Result<BinTableKey, BinConfigWriterError> {
        use BinConfigWriterError::*;

        // Tables require string keys.
        if let Some(parent_table) = parent_table {
            if let Some(key) = key {
                // Empty key strings are not allowed.
                if key.is_empty() {
                    return Err(TableKeyRequired);
                }

                // Lookup / intern the key string, return its hash and index in the string table.
                let (hash, key) =
                    Self::intern_string(strings, Some(key_table), string_writer, key)?;

                // Must succeed - table keys have a key table index.
                let index = unwrap_unchecked(key.index);

                if index > BinTableKey::max_index() {
                    return Err(TooManyKeys(BinTableKey::max_index()));
                }

                let entry = parent_table.keys.entry(hash);

                match entry {
                    // Non-unique key (error) or hash collision.
                    Entry::Occupied(mut keys) => {
                        // Make sure the key is unique.
                        if keys.get().contains(&index) {
                            return Err(NonUniqueKey);
                        }

                        // Add the new key index with the same hash.
                        keys.get_mut().push(index);
                    }
                    // New unique key - update the table key indices.
                    Entry::Vacant(_) => {
                        entry.or_insert(vec![index]);
                    }
                }

                Ok(BinTableKey::new(hash, index))
            } else {
                Err(TableKeyRequired)
            }

        // Arrays don't use keys.
        } else if key.is_some() {
            Err(ArrayKeyNotRequired)
        } else {
            Ok(BinTableKey::default())
        }
    }

    /// Looks up the `string` in the string section.
    /// If not found, interns the string.
    /// Returns its hash, offset / length w.r.t. the string section and its (optional) index in the key table (if `key_table` is `Some`,
    /// i.e. it is a key string).
    fn intern_string(
        strings: &mut HashMap<StringHash, Vec<BinConfigWriterString>>,
        mut key_table: Option<&mut Vec<InternedString>>,
        string_writer: &mut Vec<u8>,
        string: &str,
    ) -> Result<(StringHash, BinConfigWriterString), BinConfigWriterError> {
        // Hash the string.
        let hash = string_hash_fnv1a(string);

        /// Writes the string to the `string_writer` and adds it to the `key_table`, if `Some`.
        fn intern_string_impl(
            string_writer: &mut Vec<u8>,
            key_table: Option<&mut Vec<InternedString>>,
            string: &str,
        ) -> Result<BinConfigWriterString, BinConfigWriterError> {
            use BinConfigWriterError::*;

            // Offset to the start of the string is the current length of the string writer.
            let offset = string_writer.len() as u32;

            // Write the unique string and the null terminator.
            string_writer
                .write_all(string.as_bytes())
                .map_err(|_| WriteError)?;
            string_writer.write_all(&[b'\0']).map_err(|_| WriteError)?;

            let len = string.len() as u32;

            let offset_and_len = InternedString { offset, len };

            let index = if let Some(key_table) = key_table {
                // The new string's index is the current length of the string table.
                let index = key_table.len() as StringIndex;

                key_table.push(offset_and_len);

                Some(index)
            } else {
                None
            };

            Ok(BinConfigWriterString {
                offset_and_len,
                index,
            })
        }

        // Lookup the string in the hash map.
        if let Some(strings) = strings.get_mut(&hash) {
            // Hashes match - now compare the strings.
            let mut result = None;

            /// Looks up the string in the `string_writer` at `offset_and_len`.
            fn lookup_string<'s>(
                offset_and_len: InternedString,
                string_writer: &'s Vec<u8>,
            ) -> &'s str {
                unsafe {
                    from_utf8_unchecked(string_writer.get_unchecked(
                        offset_and_len.offset as usize
                            ..(offset_and_len.offset + offset_and_len.len) as usize,
                    ))
                }
            }

            for interned_string in strings.iter_mut() {
                // Strings match - return this interned string.
                if lookup_string(interned_string.offset_and_len, string_writer) == string {
                    // If the string was previously interned as a string value, but is now being interned as a key,
                    // add it to the key table and update the interned string's index.
                    if let Some(key_table) = key_table.as_mut() {
                        // The string already has a key table entry - do a debug sanity check,
                        // make sure the hash map's and the key table's offsets/lengths match.
                        if let Some(index) = interned_string.index {
                            debug_assert!((index as usize) < key_table.len());
                            let table_entry = unsafe { key_table.get_unchecked(index as usize) };
                            debug_assert_eq!(*table_entry, interned_string.offset_and_len);
                        // Otherwise add it to the key table and update the index.
                        } else {
                            // The new string's index is the current length of the string table.
                            let index = key_table.len() as StringIndex;

                            key_table.push(interned_string.offset_and_len);

                            interned_string.index.replace(index);
                        }
                    }

                    result.replace(*interned_string);
                    break;
                }
            }

            // If we found a matching string, return it and its hash.
            if let Some(string) = result {
                Ok((hash, string))

            // Else there's a hash collision - write a new string and add it to the hash map and to the key table, if `Some`.
            } else {
                let result = intern_string_impl(string_writer, key_table, string)?;

                strings.push(result);

                Ok((hash, result))
            }

        // Or write a new string and add it to the hash map and to the key table, if `Some`.
        } else {
            let result = intern_string_impl(string_writer, key_table, string)?;

            strings.insert(hash, vec![result]);

            Ok((hash, result))
        }
    }

    /// Returns the packed binary config value key
    /// and the offset in bytes w.r.t. config data blob to the current packed value.
    /// Checks if the current parent array/table is full.
    fn key_and_value_offset(
        &mut self,
        key: Option<&str>,
        value_type: ValueType,
    ) -> Result<(BinTableKey, u32), BinConfigWriterError> {
        use BinConfigWriterError::*;

        // Parent array/table; offset to current value.
        let (parent, value_offset) = Self::parent_and_value_offset(&mut self.stack)?;

        // If it's an array, ensure the value types are not mixed.
        if !parent.table {
            if let Some(array_type) = parent.array_type.as_ref() {
                if !value_type.is_compatible(*array_type) {
                    return Err(MixedArray {
                        expected: *array_type,
                        found: value_type,
                    });
                }
            } else {
                parent.array_type.replace(value_type);
            }
        }

        // If it's a parent table, a string key must be provided.
        let key = Self::key(
            &mut self.strings,
            &mut self.key_table,
            &mut self.string_writer,
            if parent.table { Some(parent) } else { None },
            key,
        )?;

        Ok((key, value_offset))
    }

    /// Returns the current parent array/table.
    /// and the offset in bytes w.r.t. config data blob to its current value.
    /// Checks if the current parent array/table is full.
    fn parent_and_value_offset(
        stack: &mut Vec<BinConfigArrayOrTable>,
    ) -> Result<(&mut BinConfigArrayOrTable, u32), BinConfigWriterError> {
        use BinConfigWriterError::*;

        // Must have a parent array/table.
        debug_assert_ne!(stack.len(), 0);

        let parent = stack.len() - 1;
        let parent = unsafe { stack.get_unchecked_mut(parent) };

        // Must not be full.
        if parent.current_len >= parent.len {
            return Err(ArrayOrTableLengthMismatch {
                expected: parent.len,
                found: parent.current_len + 1,
            });
        }

        let value_offset = parent.value_offset;

        Ok((parent, value_offset))
    }

    /// Increments the currently active array's / table's length;
    /// bumps the value offset for the next value.
    /// NOTE - caller guarantees the parent array / table in not full.
    fn increment_len(stack: &mut Vec<BinConfigArrayOrTable>) {
        // Must have a parent array/table.
        debug_assert_ne!(stack.len(), 0);

        let last = stack.len() - 1;
        let parent = unsafe { stack.get_unchecked_mut(last) };

        // Must not be full.
        debug_assert!(parent.current_len < parent.len);
        parent.current_len += 1;

        // Bump the parent array's/table's value offset for the next value.
        parent.value_offset += size_of::<BinConfigPackedValue>() as u32;
    }

    /// Adds `string_offset` to all string offsets in the binary config `data` blob
    /// to transform the from offset w.r.t. the string section to offsets w.r.t. the data blob.
    fn fixup_string_offsets(data: &mut [u8], string_offset: u32) {
        let header = unsafe { &mut *(data.as_mut_ptr() as *mut BinConfigHeader) };
        let len = header.len();

        let base = data.as_mut_ptr() as *mut u8;

        let begin = unsafe { base.add(size_of::<BinConfigHeader>()) as *mut BinConfigPackedValue };
        let values = unsafe { std::slice::from_raw_parts_mut(begin, len as _) };

        Self::fixup_string_offsets_impl(base, values, string_offset);
    }

    fn fixup_string_offsets_impl(
        base: *mut u8,
        values: &mut [BinConfigPackedValue],
        string_offset: u32,
    ) {
        for value in values.iter_mut() {
            match value.value_type() {
                // If the value is a string, fix it up.
                ValueType::String => {
                    value.set_offset(value.offset() + string_offset);
                }
                // If the value is an array/table, process its elements recursively.
                ValueType::Array | ValueType::Table => {
                    let begin = unsafe {
                        base.offset(value.offset() as isize) as *mut BinConfigPackedValue
                    };
                    let values = unsafe { std::slice::from_raw_parts_mut(begin, value.len() as _) };

                    Self::fixup_string_offsets_impl(base, values, string_offset);
                }
                _ => {}
            }
        }
    }

    fn write_value(
        config_writer: &mut Cursor<Vec<u8>>,
        stack: &mut Vec<BinConfigArrayOrTable>,
        value: BinConfigPackedValue,
        offset: u32,
    ) -> Result<(), BinConfigWriterError> {
        use BinConfigWriterError::*;

        config_writer
            .seek(SeekFrom::Start(offset as u64))
            .map_err(|_| WriteError)?;
        value.write(config_writer).map_err(|_| WriteError)?;

        // Increment the parent array's/table's table length/value offset.
        Self::increment_len(stack);

        Ok(())
    }
}

/// Represents a binary array/table, currently recorded by the binary config writer.
struct BinConfigArrayOrTable {
    // Is this an array or a table?
    // Table elements require string keys.
    table: bool,
    // Declared array/table length.
    len: u32,
    // Current array/table length.
    current_len: u32,
    // Offset in bytes to the current array/table element w.r.t. config data blob.
    value_offset: u32,
    // Must keep track of table keys to ensure key uniqueness.
    keys: HashMap<StringHash, Vec<StringIndex>>,
    // For arrays must keep track of value type to ensure no mixed arrays.
    array_type: Option<ValueType>,
}

impl BinConfigArrayOrTable {
    fn new(table: bool, len: u32, value_offset: u32) -> Self {
        Self {
            table,
            len,
            current_len: 0,
            value_offset,
            keys: HashMap::new(),
            array_type: None,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]

    use crate::*;

    #[test]
    fn EmptyRootTable() {
        assert_eq!(
            BinConfigWriter::new(0).err().unwrap(),
            BinConfigWriterError::EmptyRootTable
        );

        // But this works.

        BinConfigWriter::new(1).unwrap();
    }

    #[test]
    fn TableKeyRequired() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        assert_eq!(
            writer.bool(None, true).err().unwrap(),
            BinConfigWriterError::TableKeyRequired
        );

        // But this works.

        writer.bool("bool", true).unwrap();
    }

    #[test]
    fn ArrayKeyNotRequired() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array("array", 1).unwrap();
        assert_eq!(
            writer.bool("bool", true).err().unwrap(),
            BinConfigWriterError::ArrayKeyNotRequired
        );

        // But this works.

        writer.bool(None, true).unwrap();
    }

    #[test]
    fn MixedArray() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array("array", 2).unwrap();
        writer.bool(None, true).unwrap();
        assert_eq!(
            writer.i64(None, 7).err().unwrap(),
            BinConfigWriterError::MixedArray {
                expected: ValueType::Bool,
                found: ValueType::I64
            }
        );

        // But this works.

        writer.bool(None, false).unwrap();
    }

    #[test]
    fn NonUniqueKey() {
        let mut writer = BinConfigWriter::new(2).unwrap();
        writer.bool("bool", true).unwrap();
        assert_eq!(
            writer.bool("bool", true).err().unwrap(),
            BinConfigWriterError::NonUniqueKey
        );

        // But this works.

        writer.bool("other_bool", false).unwrap();
    }

    #[test]
    fn ArrayOrTableLengthMismatch() {
        // Underflow, root table.
        {
            let writer = BinConfigWriter::new(1).unwrap();
            assert_eq!(
                writer.finish().err().unwrap(),
                BinConfigWriterError::ArrayOrTableLengthMismatch {
                    expected: 1,
                    found: 0
                }
            );

            // But this works.

            let mut writer = BinConfigWriter::new(1).unwrap();
            writer.bool("bool", true).unwrap();
            writer.finish().unwrap();
        }

        // Overflow, root table.
        {
            let mut writer = BinConfigWriter::new(1).unwrap();
            writer.bool(Some("bool_0"), true).unwrap();
            assert_eq!(
                writer.bool(Some("bool_1"), true).err().unwrap(),
                BinConfigWriterError::ArrayOrTableLengthMismatch {
                    expected: 1,
                    found: 2
                }
            );

            // But this works.

            writer.finish().unwrap();
        }

        // Overflow, nested table.
        {
            let mut writer = BinConfigWriter::new(1).unwrap();
            writer.table(Some("table"), 1).unwrap();
            writer.bool(Some("bool_0"), true).unwrap();
            assert_eq!(
                writer.bool(Some("bool_1"), true).err().unwrap(),
                BinConfigWriterError::ArrayOrTableLengthMismatch {
                    expected: 1,
                    found: 2
                }
            );

            // But this works.

            writer.end().unwrap();
            writer.finish().unwrap();
        }

        // Underflow, nested table.
        {
            let mut writer = BinConfigWriter::new(1).unwrap();
            writer.table("table", 2).unwrap();
            writer.bool("bool_0", true).unwrap();
            assert_eq!(
                writer.end().err().unwrap(),
                BinConfigWriterError::ArrayOrTableLengthMismatch {
                    expected: 2,
                    found: 1
                }
            );

            // But this works.

            writer.bool("bool_1", false).unwrap();
            writer.end().unwrap();
            writer.finish().unwrap();
        }

        // Overflow, nested array.
        {
            let mut writer = BinConfigWriter::new(1).unwrap();
            writer.array("array", 1).unwrap();
            writer.bool(None, true).unwrap();
            assert_eq!(
                writer.bool(None, true).err().unwrap(),
                BinConfigWriterError::ArrayOrTableLengthMismatch {
                    expected: 1,
                    found: 2
                }
            );

            // But this works.

            writer.end().unwrap();
            writer.finish().unwrap();
        }

        // Underflow, nested array.
        {
            let mut writer = BinConfigWriter::new(1).unwrap();
            writer.array(Some("array"), 2).unwrap();
            writer.bool(None, true).unwrap();
            assert_eq!(
                writer.end().err().unwrap(),
                BinConfigWriterError::ArrayOrTableLengthMismatch {
                    expected: 2,
                    found: 1
                }
            );

            // But this works.

            writer.bool(None, false).unwrap();
            writer.end().unwrap();
            writer.finish().unwrap();
        }
    }

    #[test]
    fn EndCallMismatch() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.bool(Some("bool"), true).unwrap();
        assert_eq!(
            writer.end().err().unwrap(),
            BinConfigWriterError::EndCallMismatch
        );

        // But this works.

        writer.finish().unwrap();

        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array("array", 0).unwrap();
        writer.end().unwrap();
        writer.finish().unwrap();
    }

    #[test]
    fn UnfinishedArraysOrTables() {
        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array(Some("array"), 1).unwrap();
        assert_eq!(
            writer.finish().err().unwrap(),
            BinConfigWriterError::UnfinishedArraysOrTables(1)
        );

        // But this succeeds.

        let mut writer = BinConfigWriter::new(1).unwrap();
        writer.array(Some("array"), 1).unwrap();
        writer.bool(None, false).unwrap();
        writer.end().unwrap();
        writer.finish().unwrap();
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

    #[test]
    fn writer() {
        // Write the config.

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

        // Parse the blob and use the config.

        let config = BinConfig::new(data).unwrap();

        assert!(!config.root().contains("missing_value".into()));
        assert!(!config.root().contains_str("missing_value"));
        #[cfg(feature = "str_hash")]
        {
            assert!(!config.root().contains(key!("missing_value")));
        }

        assert!(config.root().contains("array_value".into()));
        assert!(config.root().contains_str("array_value"));
        #[cfg(feature = "str_hash")]
        {
            assert!(config.root().contains(key!("array_value")));
        }

        let array_value = config.root().get_array("array_value".into()).unwrap();

        assert_eq!(array_value.len(), 3);
        assert_eq!(array_value.get_i64(0).unwrap(), 54);
        assert!(cmp_f64(array_value.get_f64(0).unwrap(), 54.0));
        assert_eq!(array_value.get_i64(1).unwrap(), 12);
        assert!(cmp_f64(array_value.get_f64(1).unwrap(), 12.0));
        assert_eq!(array_value.get_i64(2).unwrap(), 78);
        assert!(cmp_f64(array_value.get_f64(2).unwrap(), 78.9));

        assert!(config.root().contains("bool_value".into()));
        assert!(config.root().contains_str("bool_value"));
        #[cfg(feature = "str_hash")]
        {
            assert!(config.root().contains(key!("bool_value")));
        }
        assert_eq!(config.root().get_bool("bool_value".into()).unwrap(), true);
        assert_eq!(config.root().get_bool_str("bool_value").unwrap(), true);
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_bool_str(key!("bool_value")).unwrap(),
                true
            );
        }

        assert!(config.root().contains("float_value".into()));
        assert!(config.root().contains_str("float_value"));
        #[cfg(feature = "str_hash")]
        {
            assert!(config.root().contains(key!("float_value")));
        }
        assert!(cmp_f64(
            config.root().get_f64("float_value".into()).unwrap(),
            3.14
        ));
        assert!(cmp_f64(
            config.root().get_f64_str("float_value").unwrap(),
            3.14
        ));
        #[cfg(feature = "str_hash")]
        {
            assert!(cmp_f64(
                config.root().get_f64(key!("float_value")).unwrap(),
                3.14
            ));
        }

        assert!(config.root().contains("int_value".into()));
        assert!(config.root().contains_str("int_value"));
        #[cfg(feature = "str_hash")]
        {
            assert!(config.root().contains_str(key!("int_value")));
        }

        assert_eq!(config.root().get_i64("int_value".into()).unwrap(), 7);
        assert_eq!(config.root().get_i64_str("int_value").unwrap(), 7);
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(config.root().get_i64(key!("int_value")).unwrap(), 7);
        }

        assert!(config.root().contains("string_value".into()));
        assert!(config.root().contains_str("string_value"));
        #[cfg(feature = "str_hash")]
        {
            assert!(config.root().contains(key!("string_value")));
        }

        assert_eq!(
            config.root().get_string("string_value".into()).unwrap(),
            "foo"
        );
        assert_eq!(config.root().get_string_str("string_value").unwrap(), "foo");
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(
                config.root().get_string(key!("string_value")).unwrap(),
                "foo"
            );
        }

        assert!(config.root().contains("table_value".into()));
        assert!(config.root().contains_str("table_value"));
        #[cfg(feature = "str_hash")]
        {
            assert!(config.root().contains(key!("table_value")));
        }

        let table_value = config.root().get_table("table_value".into()).unwrap();

        assert_eq!(table_value.len(), 3);
        assert!(table_value.contains("bar".into()));
        assert!(table_value.contains_str("bar"));
        #[cfg(feature = "str_hash")]
        {
            assert!(table_value.contains(key!("bar")));
        }
        assert_eq!(table_value.get_i64("bar".into()).unwrap(), 2020);
        assert_eq!(table_value.get_i64_str("bar").unwrap(), 2020);
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(table_value.get_i64(key!("bar")).unwrap(), 2020);
        }
        assert!(cmp_f64(table_value.get_f64("bar".into()).unwrap(), 2020.0));
        assert!(cmp_f64(table_value.get_f64_str("bar").unwrap(), 2020.0));
        #[cfg(feature = "str_hash")]
        {
            assert!(cmp_f64(table_value.get_f64(key!("bar")).unwrap(), 2020.0));
        }
        assert!(table_value.contains("baz".into()));
        assert!(table_value.contains_str("baz"));
        #[cfg(feature = "str_hash")]
        {
            assert!(table_value.contains(key!("baz")));
        }
        assert_eq!(table_value.get_string("baz".into()).unwrap(), "hello");
        assert_eq!(table_value.get_string_str("baz").unwrap(), "hello");
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(table_value.get_string(key!("baz")).unwrap(), "hello");
        }
        assert!(table_value.contains("foo".into()));
        assert!(table_value.contains_str("foo"));
        #[cfg(feature = "str_hash")]
        {
            assert!(table_value.contains(key!("foo")));
        }
        assert_eq!(table_value.get_bool("foo".into()).unwrap(), false);
        assert_eq!(table_value.get_bool_str("foo").unwrap(), false);
        #[cfg(feature = "str_hash")]
        {
            assert_eq!(table_value.get_bool(key!("foo")).unwrap(), false);
        }
        assert!(!table_value.contains("bob".into()));
        assert!(!table_value.contains_str("bob"));
        #[cfg(feature = "str_hash")]
        {
            assert!(!table_value.contains(key!("bob")));
        }
    }
}
