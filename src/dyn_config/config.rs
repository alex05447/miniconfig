use std::fmt::{Display, Formatter};

use crate::{
    DisplayIndent, DynArray, DynArrayMut, DynArrayRef, DynTable, DynTableMut, DynTableRef, Value,
};

#[cfg(feature = "bin")]
use crate::{BinConfigWriter, BinConfigWriterError};

/// Represents a mutable config with a root hashmap [`table`].
///
/// [`table`]: struct.DynTable.html
pub struct DynConfig(DynTable);

impl DynConfig {
    /// Creates a new [`config`] with an empty root [`table`].
    ///
    /// [`config`]: struct.DynConfig.html
    /// [`table`]: struct.DynTable.html
    pub fn new() -> Self {
        Self(DynTable::new())
    }

    /// Returns the immutable reference to the root [`table`] of the [`config`].
    ///
    /// [`table`]: struct.DynTable.html
    /// [`config`]: struct.DynConfig.html
    pub fn root(&self) -> DynTableRef<'_> {
        DynTableRef::new(&self.0)
    }

    /// Returns the mutable reference to the root [`table`] of the [`config`].
    ///
    /// [`table`]: struct.DynTable.html
    /// [`config`]: struct.DynConfig.html
    pub fn root_mut(&mut self) -> DynTableMut<'_> {
        DynTableMut::new(&mut self.0)
    }

    /// Tries to serialize this [`config`] to a [`binary config`].
    ///
    /// [`config`]: struct.LuaConfig.html
    /// [`binary config`]: struct.BinConfig.html
    #[cfg(feature = "bin")]
    pub fn to_bin_config(&self) -> Result<Box<[u8]>, BinConfigWriterError> {
        use BinConfigWriterError::*;

        let root = self.root();

        // The root table is empty - nothing to do.
        if root.len() == 0 {
            return Err(EmptyRootTable);
        }

        let mut writer = BinConfigWriter::new(root.len())?;

        Self::table_to_bin_config(root, &mut writer)?;

        writer.finish()
    }

    #[cfg(feature = "bin")]
    fn table_to_bin_config(
        table: DynTableRef<'_>,
        writer: &mut BinConfigWriter,
    ) -> Result<(), BinConfigWriterError> {
        // Gather the keys.
        let mut key_strins: Vec<_> = table.iter().map(|(key, _)| key).collect();

        // Sort the keys in alphabetical order.
        key_strins.sort_by(|l, r| l.cmp(r));

        // Iterate the table using the sorted keys.
        for key_string in key_strins.into_iter() {
            // Must succeed.
            let value = table.get(key_string).unwrap();

            Self::value_to_bin_config(Some(key_string), value, writer)?;
        }

        Ok(())
    }

    #[cfg(feature = "bin")]
    fn array_to_bin_config(
        array: DynArrayRef<'_>,
        writer: &mut BinConfigWriter,
    ) -> Result<(), BinConfigWriterError> {
        // Iterate the array in order.
        for value in array.iter() {
            Self::value_to_bin_config(None, value, writer)?;
        }

        Ok(())
    }

    #[cfg(feature = "bin")]
    fn value_to_bin_config(
        key: Option<&str>,
        value: Value<&'_ str, DynArrayRef<'_>, DynTableRef<'_>>,
        writer: &mut BinConfigWriter,
    ) -> Result<(), BinConfigWriterError> {
        use Value::*;

        match value {
            Bool(value) => {
                writer.bool(key, value)?;
            }
            I64(value) => {
                writer.i64(key, value)?;
            }
            F64(value) => {
                writer.f64(key, value)?;
            }
            String(value) => {
                writer.string(key, value.as_ref())?;
            }
            Array(value) => {
                writer.array(key, value.len())?;
                Self::array_to_bin_config(value, writer)?;
                writer.end()?;
            }
            Table(value) => {
                writer.table(key, value.len())?;
                Self::table_to_bin_config(value, writer)?;
                writer.end()?;
            }
        }

        Ok(())
    }
}

impl Display for DynConfig {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.root().fmt_indent(f, 0, false)
    }
}

impl<'a> Display for Value<&'a str, DynArray, DynTable> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_indent(f, 0, true)
    }
}

impl<'a> Display for Value<&'a str, DynArrayRef<'a>, DynTableRef<'a>> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_indent(f, 0, true)
    }
}

impl<'a> Display for Value<&'a str, DynArrayMut<'a>, DynTableMut<'a>> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_indent(f, 0, true)
    }
}