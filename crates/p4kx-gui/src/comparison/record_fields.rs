use p4kx_comparison::{ChangedValue, FieldChange, RecordChange, RecordSide, Reference, Value};

/// One side's value of a changed field as the field list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    /// A scalar in the text form of Record XML, e.g. `1.5`, `true` or an enum option.
    Scalar(Box<str>),
    /// A link to a Record: the target's record path and, for a weak pointer, where inside it.
    Reference {
        /// The record path of the target Record, without `.xml`.
        record_path: Box<str>,
        /// For a weak pointer, where inside the target it points, e.g. `wheels[0]`.
        at: Option<Box<str>>,
    },
    /// A struct, by the name of its type.
    Struct(Box<str>),
    /// An array, by its number of elements.
    Array(usize),
    /// A null pointer or a null Reference.
    Null,
    /// A value that cannot be read, by the reason.
    Error(Box<str>),
}

impl FieldValue {
    /// Returns the value that stands for `value`.
    pub(crate) fn of(value: &ChangedValue<'_>) -> Self {
        match value {
            ChangedValue::Struct(type_name) => Self::Struct((*type_name).into()),
            ChangedValue::Array(len) => Self::Array(*len),
            ChangedValue::Value(Value::Reference(link)) => reference(link),
            ChangedValue::Value(Value::Null) => Self::Null,
            ChangedValue::Value(Value::Error(error)) => Self::Error(error.to_string().into()),
            ChangedValue::Value(scalar) => Self::Scalar(scalar.to_string().into()),
            _ => Self::Null,
        }
    }
}

fn reference(link: &Reference<'_>) -> FieldValue {
    FieldValue::Reference {
        record_path: link.record_path().into(),
        at: link.location().map(|at| at.as_str().into()),
    }
}

/// What a row of the field list names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldName {
    /// The name of the Record itself.
    RecordName,
    /// The type of the Record itself.
    RecordType,
    /// A field or array element at this path, e.g. `hull.plates[1].armor`.
    Field(Box<str>),
}

/// One field, array element, name or type that differs between both sides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRow {
    /// What differs.
    pub name: FieldName,
    /// The old value, `None` when the old side has none.
    pub old: Option<FieldValue>,
    /// The new value, `None` when the new side has none.
    pub new: Option<FieldValue>,
}

/// Why a Record cannot be read on one side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideError {
    /// The side that cannot be read.
    pub side: RecordSide,
    /// The reason.
    pub reason: Box<str>,
}

/// The changes inside one Record: its name and type, then every field change, and why it
/// cannot be read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordFields {
    rows: Vec<FieldRow>,
    errors: Vec<SideError>,
}

impl RecordFields {
    /// Takes the changes of `change`: a changed name and type first, then the fields.
    pub(crate) fn of(change: &RecordChange<'_>) -> Self {
        let mut rows = Vec::with_capacity(change.fields().len().saturating_add(2));
        if let (Some(old), Some(new)) = (change.old_record(), change.new_record()) {
            let identity = [
                (FieldName::RecordName, old.name(), new.name()),
                (FieldName::RecordType, old.type_name(), new.type_name()),
            ];
            rows.extend(identity.into_iter().filter(|(_, old, new)| old != new).map(
                |(name, old, new)| FieldRow {
                    name,
                    old: Some(FieldValue::Scalar(old.into())),
                    new: Some(FieldValue::Scalar(new.into())),
                },
            ));
        }
        rows.extend(change.fields().iter().map(field_row));
        Self {
            rows,
            errors: change
                .errors()
                .iter()
                .map(|error| SideError {
                    side: error.side(),
                    reason: error.error().to_string().into(),
                })
                .collect(),
        }
    }

    /// Returns the rows: a changed name and type, then the fields depth first.
    #[must_use]
    pub fn rows(&self) -> &[FieldRow] {
        &self.rows
    }

    /// Returns why the Record cannot be read, one entry per side; empty unless it is unreadable.
    #[must_use]
    pub fn errors(&self) -> &[SideError] {
        &self.errors
    }
}

fn field_row(change: &FieldChange<'_>) -> FieldRow {
    FieldRow {
        name: FieldName::Field(change.path().as_str().into()),
        old: change.old_value().map(FieldValue::of),
        new: change.new_value().map(FieldValue::of),
    }
}
