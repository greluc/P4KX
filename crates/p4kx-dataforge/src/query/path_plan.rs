use crate::DataType;
use crate::query::reader::Reader;
use crate::query::resolve::{Resolved, resolve};
use crate::query::schema::{FieldDef, StructFields};
use crate::query::suggest::closest;
use crate::query::syntax::{Name, Path};
use crate::query::{QueryError, QueryErrorKind};

/// The kind of a field that decides how the rest of a path resolves behind it.
type SlotKey = (DataType, u16, bool);

/// How one path of a Query is read from the Records of each struct.
#[derive(Debug, Clone)]
pub(crate) enum PathPlan {
    /// With `from`: resolved once against the `from` struct, for the structs whose first field
    /// of the path stands at `position` as it does there.
    Fixed {
        head: String,
        position: u32,
        resolved: Resolved,
    },
    /// Without `from`: resolved once per kind of field the path starts with, relative to it.
    ByHead {
        head: String,
        resolved: Vec<(SlotKey, Resolved)>,
    },
}

impl PathPlan {
    /// Resolves `path` against the struct at `nominal`, the `from` struct, or without `from`
    /// against each struct that declares its first field.
    ///
    /// Without `from`, the path is an error only when it resolves in no struct; the error is the
    /// one of the first struct that declares the field.
    pub(crate) fn of(
        structs: &StructFields<'_>,
        nominal: Option<usize>,
        path: &Path,
        from: Option<&Name>,
    ) -> Result<Self, QueryError> {
        let head = &path.head;
        if let (Some(index), Some(from)) = (nominal, from) {
            let Some(field) = structs.field(index, head.as_str()) else {
                return Err(unknown_field(
                    head,
                    Some(from.as_str()),
                    closest(head.as_str(), structs.field_names(index)),
                ));
            };
            return Ok(Self::Fixed {
                head: head.as_str().to_owned(),
                position: field.position,
                resolved: resolve(structs, &field, path, Some(from.as_str()))?,
            });
        }
        let mut seen: Vec<SlotKey> = Vec::new();
        let mut resolved = Vec::new();
        let mut first_error = None;
        for index in 0..structs.len() {
            let Some(field) = structs.field(index, head.as_str()) else {
                continue;
            };
            let key = key_of(&field);
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            let relative = FieldDef {
                start: 0,
                end: field.end.saturating_sub(field.start),
                ..field
            };
            match resolve(structs, &relative, path, None) {
                Ok(done) => resolved.push((key, done)),
                Err(error) => {
                    first_error.get_or_insert(error);
                }
            }
        }
        if resolved.is_empty() {
            return Err(first_error.unwrap_or_else(|| {
                unknown_field(
                    head,
                    None,
                    closest(head.as_str(), structs.all_field_names()),
                )
            }));
        }
        Ok(Self::ByHead {
            head: head.as_str().to_owned(),
            resolved,
        })
    }

    /// Returns the types of the scalars the path ends at, one per way it resolves.
    pub(crate) fn leaves(&self) -> impl Iterator<Item = DataType> + '_ {
        let (fixed, by_head) = match self {
            Self::Fixed { resolved, .. } => (Some(resolved.leaf), &[][..]),
            Self::ByHead { resolved, .. } => (None, &resolved[..]),
        };
        fixed
            .into_iter()
            .chain(by_head.iter().map(|(_, resolved)| resolved.leaf))
    }

    /// Returns how the path is read from the Records of the struct whose fields are `fields`,
    /// with the type of its scalars, or `None` when it does not resolve there.
    pub(crate) fn reader(&self, fields: &[FieldDef<'_>]) -> Option<(Reader, DataType)> {
        match self {
            Self::Fixed {
                head,
                position,
                resolved,
            } => fields
                .iter()
                .find(|field| field.name == head && field.position == *position)
                .map(|_| (resolved.reader.clone(), resolved.leaf)),
            Self::ByHead { head, resolved } => {
                let field = fields.iter().find(|field| field.name == head)?;
                let key = key_of(field);
                resolved
                    .iter()
                    .find(|(candidate, _)| *candidate == key)
                    .map(|(_, done)| (done.reader.shifted(field.start), done.leaf))
            }
        }
    }
}

const fn key_of(field: &FieldDef<'_>) -> SlotKey {
    (
        field.slot.data_type(),
        field.slot.target(),
        field.slot.is_array(),
    )
}

fn unknown_field(name: &Name, owner: Option<&str>, suggestion: Option<String>) -> QueryError {
    QueryError::new(
        QueryErrorKind::UnknownField {
            name: name.as_str().to_owned(),
            owner: owner.map(str::to_owned),
            suggestion,
        },
        name.span(),
    )
}
