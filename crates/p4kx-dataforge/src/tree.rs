use crate::{Field, Guid, Value, ValueError};

/// The canonical tree of one Record: its type, GUID, record path, name and fields.
///
/// It is the single model every output is made from; text is borrowed from the DataForge.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordTree<'f> {
    pub(crate) type_name: &'f str,
    pub(crate) guid: Guid,
    pub(crate) record_path: &'f str,
    pub(crate) name: &'f str,
    pub(crate) fields: Vec<Field<'f>>,
}

impl<'f> RecordTree<'f> {
    /// Returns the name of the Record's struct, e.g. `EntityClassDefinition`.
    #[must_use]
    pub const fn type_name(&self) -> &'f str {
        self.type_name
    }

    /// Returns the GUID that identifies the Record.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// Returns the record path, without `.xml`.
    #[must_use]
    pub const fn record_path(&self) -> &'f str {
        self.record_path
    }

    /// Returns the Record name, e.g. `EntityClassDefinition.AEGS_Avenger`.
    #[must_use]
    pub const fn name(&self) -> &'f str {
        self.name
    }

    /// Returns the fields in declaration order, the inherited ones first.
    #[must_use]
    pub fn fields(&self) -> &[Field<'f>] {
        &self.fields
    }

    /// Returns the error values with the name of their field, in field order.
    pub fn errors(&self) -> impl Iterator<Item = (&'f str, &ValueError)> {
        self.fields.iter().filter_map(|field| match &field.value {
            Value::Error(error) => Some((field.name, error)),
            _ => None,
        })
    }
}
