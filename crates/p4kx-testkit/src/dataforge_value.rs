/// A value of one field in the instance of a synthetic DataForge Record.
#[derive(Debug, Clone, PartialEq)]
pub enum TestValue {
    /// A boolean, stored as one byte.
    Bool(bool),
    /// A signed 8-bit integer.
    Int8(i8),
    /// A signed 16-bit integer.
    Int16(i16),
    /// A signed 32-bit integer.
    Int32(i32),
    /// A signed 64-bit integer.
    Int64(i64),
    /// An unsigned 8-bit integer.
    UInt8(u8),
    /// An unsigned 16-bit integer.
    UInt16(u16),
    /// An unsigned 32-bit integer.
    UInt32(u32),
    /// An unsigned 64-bit integer.
    UInt64(u64),
    /// A 32-bit float.
    Single(f32),
    /// A 64-bit float.
    Double(f64),
    /// Text stored in the text table: a string, a localisation key or an enum option name.
    Text(String),
    /// A GUID written in text as the hexadecimal number, as in [`TestRecord::new`](crate::TestRecord::new).
    Guid(u128),
    /// Bytes written as they are at the start of the field, e.g. an invalid text offset.
    Raw(Vec<u8>),
}

impl TestValue {
    /// Returns a text value.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }
}
