use std::fmt::{Display, Formatter};

use crate::ValueType;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinConfigError {
    /// Binary config blob data is invalid.
    InvalidBinaryConfigData,
}

impl Display for BinConfigError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use BinConfigError::*;

        match self {
            InvalidBinaryConfigData => write!(f, "Binary config blob data is invalid."),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinArrayGetError {
    /// Index was out of bounds.
    /// Contains the [`array`] length.
    /// [`array`]: struct.BinArray.html
    IndexOutOfBounds(u32),
    /// Tried to pop an empty [`array`].
    /// [`array`]: struct.BinArray.html
    ArrayEmpty,
}

impl Display for BinArrayGetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use BinArrayGetError::*;

        match self {
            IndexOutOfBounds(len) => {
                write!(f, "Array index was out of bounds (length is {}).", len)
            }
            ArrayEmpty => write!(f, "Tried to pop an empty array."),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinTableGetError {
    /// Provided key does not exist in the [`table`].
    /// [`table`]: struct.BinTable.html
    KeyDoesNotExist,
}

impl Display for BinTableGetError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use BinTableGetError::*;

        match self {
            KeyDoesNotExist => write!(f, "Provided key does not exist in the table."),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinConfigWriterError {
    /// Empty [`binary config`] root tables are not supported.
    /// [`binary config`]: struct.BinConfig.html
    EmptyRootTable,
    /// A non-empty string key is required for a [`table`] element.
    /// [`table`]: struct.BinTable.html
    TableKeyRequired,
    /// A string key is not required for an [`array`] element.
    /// [`array`]: struct.BinArray.html
    ArrayKeyNotRequired,
    /// Mixed (and non-convertible) type values in the [`array`].
    /// [`array`]: struct.BinArray.html
    MixedArray {
        /// Expected [`array`] value type (as determined by the first value in the [`array`]).
        /// [`array`]: struct.BinArray.html
        expected: ValueType,
        /// Found [`array`] value type.
        /// [`array`]: struct.BinArray.html
        found: ValueType,
    },
    /// A non-unique string key was provided for a [`table`] element.
    /// [`table`]: struct.BinTable.html
    NonUniqueKey,
    /// Mismatch between decalred [`array`]/[`table`] length and actual number of elements provided.
    /// [`array`]: struct.BinArray.html
    /// [`table`]: struct.BinTable.html
    ArrayOrTableLengthMismatch { expected: u32, found: u32 },
    /// Mismatched call to [`end`] (expected a previous call to [`array`]/[`table`]).
    /// [`finish`]: struct.BinConfigWriter.html#method.end
    /// [`array`]: struct.BinConfigWriter.html#method.array
    /// [`table`]: struct.BinConfigWriter.html#method.table
    EndCallMismatch,
    /// One or more unfinished [`arrays`]/[`tables`] remain in the call to [`finish`].
    /// Contains the number of unfinished [`arrays`]/[`tables`].
    /// [`arrays`]: struct.BinArray.html
    /// [`tables`]: struct.BinTable.html
    /// [`finish`]: struct.BinConfigWriter.html#method.finish
    UnfinishedArraysOrTables(u32),
    /// General write error (out of memory?).
    WriteError,
}

impl Display for BinConfigWriterError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use BinConfigWriterError::*;

        match self {
            EmptyRootTable => write!(f, "Empty binary config root tables are not supported."),
            TableKeyRequired => write!(f, "A non-empty string key is required for a table element."),
            ArrayKeyNotRequired => write!(f, "A string key is not required for an array element."),
            MixedArray { expected, found } => write!(f, "Mixed (and non-convertible) type values in the array. Expected \"{}\", found \"{}\".", expected, found),
            NonUniqueKey => write!(f, "A non-unique string key was provided for a table element."),
            ArrayOrTableLengthMismatch { expected, found } => write!(
                f,
                "Mismatch between decalred array/table length ({}) and actual number of elements provided ({}).",
                expected,
                found,
            ),
            EndCallMismatch => write!(f, "Mismatched call to `end` (expected a previous call to `array`/`table`)."),
            UnfinishedArraysOrTables(num) => write!(f, "{} unfinished array(s)/table(s) remain in the call to `finish`.", num),
            WriteError => write!(f, "General write error (out of memory?)."),
        }
    }
}
