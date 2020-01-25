use std::fmt::Formatter;

use crate::Value;

pub(crate) trait DisplayIndent {
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, comma: bool) -> std::fmt::Result;

    fn do_indent(f: &mut Formatter, indent: u32) -> std::fmt::Result {
        for _ in 0..indent {
            write!(f, "\t")?;
        }

        Ok(())
    }
}

impl<'s, S, A, T> DisplayIndent for Value<S, A, T>
where
    S: AsRef<str>,
    A: DisplayIndent,
    T: DisplayIndent,
{
    fn fmt_indent(&self, f: &mut Formatter, indent: u32, _comma: bool) -> std::fmt::Result {
        match self {
            Value::Bool(value) => write!(f, "{}", if *value { "true" } else { "false" }),
            Value::I64(value) => write!(f, "{}", value),
            Value::F64(value) => write!(f, "{}", value),
            Value::String(value) => write!(f, "\"{}\"", value.as_ref()),
            Value::Array(value) => value.fmt_indent(f, indent + 1, true),
            Value::Table(value) => value.fmt_indent(f, indent + 1, true),
        }
    }
}
