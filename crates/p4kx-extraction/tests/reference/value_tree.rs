// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The value model both sides of the DataForge comparison are brought to.

use p4kx_extraction::{RecordTree, StructValue, Value, ValueError};

/// A number as the reference JSON writes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum JsonNumber {
    Signed(i64),
    Unsigned(u64),
    Float(f64),
}

/// One value of a Record: its kind and its content. Kinds P4KX knows exactly (`Single`,
/// `Double`, the integers) are kept apart so that a number is compared at its own precision.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Node {
    Null,
    Bool(bool),
    Int(i128),
    Single(f32),
    Double(f64),
    Number(JsonNumber),
    Text(String),
    Guid(String),
    /// A link to a Record; the reference side may state only the path, or embed the content of
    /// the Record it links to.
    Reference {
        guid: String,
        path: String,
        embedded: Option<Box<Embedded>>,
    },
    WeakPointer,
    Struct {
        type_name: String,
        fields: Vec<Field>,
    },
    Array(Vec<Self>),
    /// A Reference to a Record that does not exist.
    Dangling,
    /// A bool byte that is neither 0 nor 1.
    InvalidBool,
    /// Any other value P4KX cannot read, with the reason.
    Error(String),
    /// A kind of value the comparison does not map; never allowed.
    Unmapped(String),
}

pub(crate) type Field = (String, Node);

/// The content of a Record a reference tool wrote where the Record is referenced.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Embedded {
    pub(crate) name: String,
    pub(crate) root: Node,
}

/// A Record: its identity and its root struct.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RecordDoc {
    pub(crate) guid: String,
    pub(crate) name: String,
    pub(crate) tag: Option<String>,
    pub(crate) root: Node,
}

/// Sorts fields by name, ignoring case, keeping declaration order among equal names.
pub(crate) fn sorted(mut fields: Vec<Field>) -> Vec<Field> {
    fields.sort_by_cached_key(|(name, _)| name.to_lowercase());
    fields
}

fn from_struct(structure: &StructValue<'_>) -> Node {
    Node::Struct {
        type_name: structure.type_name().to_owned(),
        fields: sorted(
            structure
                .fields()
                .iter()
                .map(|field| (field.name().to_owned(), from_value(field.value())))
                .collect(),
        ),
    }
}

fn from_value(value: &Value<'_>) -> Node {
    match value {
        Value::Bool(flag) => Node::Bool(*flag),
        Value::Int8(number) => Node::Int(i128::from(*number)),
        Value::Int16(number) => Node::Int(i128::from(*number)),
        Value::Int32(number) => Node::Int(i128::from(*number)),
        Value::Int64(number) => Node::Int(i128::from(*number)),
        Value::UInt8(number) => Node::Int(i128::from(*number)),
        Value::UInt16(number) => Node::Int(i128::from(*number)),
        Value::UInt32(number) => Node::Int(i128::from(*number)),
        Value::UInt64(number) => Node::Int(i128::from(*number)),
        Value::Single(number) => Node::Single(*number),
        Value::Double(number) => Node::Double(*number),
        Value::String(text) | Value::Locale(text) | Value::Enum(text) => {
            Node::Text((*text).to_owned())
        }
        Value::Guid(guid) => Node::Guid(guid.to_string()),
        Value::Struct(structure) => from_struct(structure),
        Value::Array(items) => Node::Array(items.iter().map(from_value).collect()),
        Value::Reference(reference) if reference.location().is_some() => Node::WeakPointer,
        Value::Reference(reference) => Node::Reference {
            guid: reference.guid().to_string(),
            path: reference.record_path().to_owned(),
            embedded: None,
        },
        Value::Null => Node::Null,
        Value::Error(ValueError::DanglingReference { .. }) => Node::Dangling,
        Value::Error(ValueError::InvalidBool { .. }) => Node::InvalidBool,
        Value::Error(error) => Node::Error(error.to_string()),
        other => Node::Unmapped(format!("{other:?}")),
    }
}

impl RecordDoc {
    /// The Record P4KX builds, in the value model.
    #[must_use]
    pub(crate) fn from_tree(tree: &RecordTree<'_>) -> Self {
        Self {
            guid: tree.guid().to_string(),
            name: tree.name().to_owned(),
            tag: tree.tag().map(str::to_owned),
            root: from_struct(tree.root()),
        }
    }
}
