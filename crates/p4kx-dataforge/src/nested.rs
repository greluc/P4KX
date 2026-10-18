use std::sync::Arc;

use crate::{Field, Guid};

/// An owned struct inside a Record: an inline class or the target of a strong pointer.
#[derive(Debug, Clone, PartialEq)]
pub struct StructValue<'f> {
    pub(crate) type_name: &'f str,
    pub(crate) fields: Vec<Field<'f>>,
}

impl<'f> StructValue<'f> {
    /// Returns the name of the struct, which for a strong pointer may derive from the declared
    /// one.
    #[must_use]
    pub const fn type_name(&self) -> &'f str {
        self.type_name
    }

    /// Returns the fields in declaration order, the inherited ones first.
    #[must_use]
    pub fn fields(&self) -> &[Field<'f>] {
        &self.fields
    }
}

/// A link to a Record by GUID, never its content (ADR-0004).
///
/// A Reference field links to another Record. A weak pointer links into the Record that holds
/// it: to the same GUID and record path, with the [`location`](Self::location) of the instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference<'f> {
    pub(crate) guid: Guid,
    pub(crate) record_path: &'f str,
    pub(crate) type_name: &'f str,
    pub(crate) location: Option<Arc<str>>,
}

impl<'f> Reference<'f> {
    /// Returns the GUID of the target Record.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// Returns the record path of the target Record, without `.xml`.
    #[must_use]
    pub const fn record_path(&self) -> &'f str {
        self.record_path
    }

    /// Returns the type of the target: the struct of the Record, or of the instance at the
    /// location.
    #[must_use]
    pub const fn type_name(&self) -> &'f str {
        self.type_name
    }

    /// Returns where inside the target Record the link points, or `None` for the Record itself.
    ///
    /// The location names the fields from the Record down to the instance, separated by `.`,
    /// with `[index]` after an array field, e.g. `scene[1].preview`.
    #[must_use]
    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }
}
