use {
    crate::ValueType,
    std::{
        error::Error,
        fmt::{Display, Formatter},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinConfigError {
    /// Binary config data blob is invalid.
    InvalidBinaryConfigData,
}

impl Error for BinConfigError {}

impl Display for BinConfigError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use BinConfigError::*;

        match self {
            InvalidBinaryConfigData => "binary config data blob is invalid".fmt(f),
        }
    }
}

/// An error returned by [`bin array`] accessors.
///
/// [`bin array`]: struct.BinArray.html
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinArrayError {
    /// [`Array`] index out of bounds.
    /// Contains the actual [`array`] length.
    ///
    /// [`Array`]: struct.BinArray.html
    /// [`array`]: struct.BinArray.html
    IndexOutOfBounds(u32),
    /// [`Array`] value is of incorrect [`type`].
    /// Contains the value [`type`].
    ///
    /// [`Array`]: struct.BinArray.html
    /// [`type`]: enum.ValueType.html
    IncorrectValueType(ValueType),
}

impl Error for BinArrayError {}

impl Display for BinArrayError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use BinArrayError::*;

        match self {
            IndexOutOfBounds(len) => write!(f, "array index out of bounds (length is {})", len),
            IncorrectValueType(invalid_type) => {
                write!(f, "array value is of incorrect type: \"{}\"", invalid_type)
            }
        }
    }
}

/// An error returned by the [`bin config writer`] when recording a binary config data blob.
///
/// [`bin config writer`]: struct.BinConfigWriter.html
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinConfigWriterError {
    /// Empty [`binary config`] root tables are not supported.
    ///
    /// [`binary config`]: struct.BinConfig.html
    EmptyRootTable,
    /// A non-empty string key is required for a [`table`] element.
    ///
    /// [`table`]: struct.BinTable.html
    TableKeyRequired,
    /// A string key is not required for an [`array`] element.
    ///
    /// [`array`]: struct.BinArray.html
    ArrayKeyNotRequired,
    /// Mixed (and non-convertible) type values in the [`array`].
    ///
    /// [`array`]: struct.BinArray.html
    MixedArray {
        /// Expected [`array`] value type (as determined by the first value in the [`array`]).
        ///
        /// [`array`]: struct.BinArray.html
        expected: ValueType,
        /// Found [`array`] value type.
        ///
        /// [`array`]: struct.BinArray.html
        found: ValueType,
    },
    /// A non-unique string key was provided for a [`table`] element.
    ///
    /// [`table`]: struct.BinTable.html
    NonUniqueKey,
    /// Mismatch between decalred [`array`]/[`table`] length and actual number of elements provided.
    ///
    /// [`array`]: struct.BinArray.html
    /// [`table`]: struct.BinTable.html
    ArrayOrTableLengthMismatch { expected: u32, found: u32 },
    /// Mismatched call to [`end`] (expected a previous call to [`array`]/[`table`]).
    ///
    /// [`end`]: struct.BinConfigWriter.html#method.end
    /// [`array`]: struct.BinConfigWriter.html#method.array
    /// [`table`]: struct.BinConfigWriter.html#method.table
    EndCallMismatch,
    /// One or more unfinished [`arrays`]/[`tables`] remain in the call to [`finish`].
    /// Contains the number of unfinished [`arrays`]/[`tables`].
    ///
    /// [`arrays`]: struct.BinArray.html
    /// [`tables`]: struct.BinTable.html
    /// [`finish`]: struct.BinConfigWriter.html#method.finish
    UnfinishedArraysOrTables(u32),
    /// General write error (out of memory?).
    WriteError,
}

impl Error for BinConfigWriterError {}

impl Display for BinConfigWriterError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        use BinConfigWriterError::*;

        match self {
            EmptyRootTable => "empty binary config root tables are not supported".fmt(f),
            TableKeyRequired => "a non-empty string key is required for a table element".fmt(f),
            ArrayKeyNotRequired => "a string key is not required for an array element".fmt(f),
            MixedArray { expected, found } => write!(f, "mixed (and non-convertible) type values in the array: expected \"{}\", found \"{}\"", expected, found),
            NonUniqueKey => "a non-unique string key was provided for a table element".fmt(f),
            ArrayOrTableLengthMismatch { expected, found } => write!(
                f,
                "mismatch between decalred array/table length ({}) and actual number of elements provided ({})",
                expected,
                found,
            ),
            EndCallMismatch => "mismatched call to `end` (expected a previous call to `array`/`table`)".fmt(f),
            UnfinishedArraysOrTables(num) => write!(f, "{} unfinished array(s)/table(s) remain in the call to `finish`", num),
            WriteError => "general write error (out of memory?)".fmt(f),
        }
    }
}
