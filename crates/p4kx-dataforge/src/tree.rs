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

    /// Returns every error value in the tree, nested ones included, with the name of the field
    /// that holds it, in the order the tree lists them.
    pub fn errors(&self) -> impl Iterator<Item = (&'f str, &ValueError)> {
        let mut errors = Vec::new();
        collect_errors(&self.fields, &mut errors);
        errors.into_iter()
    }

    /// Returns the number of error values in the tree, nested ones included, without collecting
    /// them.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.fields
            .iter()
            .map(|field| count_errors(std::slice::from_ref(&field.value)))
            .sum()
    }
}

/// Returns the number of error values in the tree, nested ones included.
fn count_errors(values: &[Value<'_>]) -> usize {
    values
        .iter()
        .map(|value| match value {
            Value::Error(_) => 1,
            Value::Struct(structure) => structure
                .fields
                .iter()
                .map(|field| count_errors(std::slice::from_ref(&field.value)))
                .sum(),
            Value::Array(items) => count_errors(items),
            _ => 0,
        })
        .sum()
}

fn collect_errors<'t, 'f>(fields: &'t [Field<'f>], errors: &mut Vec<(&'f str, &'t ValueError)>) {
    for field in fields {
        collect_value_errors(field.name, &field.value, errors);
    }
}

fn collect_value_errors<'t, 'f>(
    name: &'f str,
    value: &'t Value<'f>,
    errors: &mut Vec<(&'f str, &'t ValueError)>,
) {
    match value {
        Value::Error(error) => errors.push((name, error)),
        Value::Struct(structure) => collect_errors(&structure.fields, errors),
        Value::Array(items) => {
            for item in items {
                collect_value_errors(name, item, errors);
            }
        }
        _ => {}
    }
}
