use crate::IniStr;

/// Represents an individual leaf-level `.ini` config value,
/// contained in the root of the config, config section or an array.
#[derive(Clone, Copy, Debug)]
pub enum IniValue<'s, 'a> {
    Bool(bool),
    I64(i64),
    F64(f64),
    String(IniStr<'s, 'a>),
}

impl<'s, 'a> IniValue<'s, 'a> {
    pub(crate) fn get_ini_type(&self) -> IniValueType {
        match self {
            IniValue::Bool(_) => IniValueType::Bool,
            IniValue::I64(_) => IniValueType::I64,
            IniValue::F64(_) => IniValueType::F64,
            IniValue::String(_) => IniValueType::String,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum IniValueType {
    Bool,
    I64,
    F64,
    String,
}

impl IniValueType {
    pub(crate) fn is_compatible(self, other: IniValueType) -> bool {
        use IniValueType::*;

        match self {
            Bool => other == Bool,
            I64 => (other == I64) || (other == F64),
            F64 => (other == I64) || (other == F64),
            String => other == String,
        }
    }
}
