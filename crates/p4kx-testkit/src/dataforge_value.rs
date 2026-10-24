// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

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
    /// A struct value: written inline in a class field, or as a new instance that a strong
    /// pointer or a class array holds.
    Struct(TestInstance),
    /// A strong or weak pointer to the instance or Record of this name.
    Pointer(String),
    /// A null pointer or a null Reference.
    Null,
    /// A Reference to the Record with this GUID, written as in [`TestRecord::new`](crate::TestRecord::new).
    Reference(u128),
    /// The values of an array field, stored in the pool of its type or, for a class array, as
    /// consecutive instances.
    Array(Vec<Self>),
}

impl TestValue {
    /// Returns a text value.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Returns a pointer to the instance or Record named `name`.
    #[must_use]
    pub fn pointer(name: impl Into<String>) -> Self {
        Self::Pointer(name.into())
    }
}

/// The values of one struct instance of a synthetic DataForge; fields not set stay zero, and
/// pointers not set are null.
#[derive(Debug, Clone, PartialEq)]
pub struct TestInstance {
    pub(crate) type_name: String,
    pub(crate) name: Option<String>,
    pub(crate) values: Vec<(String, TestValue)>,
}

impl TestInstance {
    /// Creates an instance of the struct `type_name`.
    #[must_use]
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            name: None,
            values: Vec::new(),
        }
    }

    /// Names the instance a struct value creates, so that a later
    /// [`TestValue::Pointer`] in the order values are written can point to it.
    #[must_use]
    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the field `name` to `value`.
    #[must_use]
    pub fn value(mut self, name: impl Into<String>, value: TestValue) -> Self {
        self.values.push((name.into(), value));
        self
    }
}
