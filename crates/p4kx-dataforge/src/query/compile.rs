use p4kx_foundation::PathPattern;

use crate::query::schema::{FieldDef, StructFields, ancestors_include};
use crate::query::suggest::closest;
use crate::query::syntax::{Name, Path, Query, Segment};
use crate::query::{QueryError, QueryErrorKind};
use crate::{DataForge, DataType};

/// A Query checked against one DataForge and ready to run.
#[derive(Debug)]
pub struct CompiledQuery<'f> {
    pub(crate) forge: &'f DataForge<'f>,
    pub(crate) columns: Vec<String>,
    pub(crate) heads: Box<[String]>,
    pub(crate) pattern: Option<PathPattern>,
    pub(crate) plans: Box<[Option<Plan>]>,
}

/// How the columns are read from the Records of one selected struct.
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    /// Per column, the position of the field in the Record tree, or `None` when the struct has
    /// no such scalar field.
    pub(crate) fields: Box<[Option<u32>]>,
}

impl<'f> CompiledQuery<'f> {
    /// Returns the headings of the columns after the record path: each path in canonical form.
    #[must_use]
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Returns the DataForge the Query was compiled for.
    #[must_use]
    pub const fn dataforge(&self) -> &'f DataForge<'f> {
        self.forge
    }
}

impl Query {
    /// Checks the type and field names against `forge` and prepares how each selected struct's
    /// Records are read.
    ///
    /// # Errors
    ///
    /// Returns a [`QueryError`] with the span of an unknown type or field name, of a field that
    /// holds no single scalar value, or of a path that continues behind a scalar field.
    pub fn compile<'f>(&self, forge: &'f DataForge<'f>) -> Result<CompiledQuery<'f>, QueryError> {
        let structs = StructFields::read_all(forge);
        let selected = match &self.from {
            Some(name) => select_type(&structs, name)?,
            None => vec![true; structs.len()],
        };
        let nominal = self.from.as_ref().and_then(|name| {
            let index = structs.position_of(name.as_str())?;
            Some(structs.fields(index).map(|_| index).map_err(|error| {
                QueryError::new(QueryErrorKind::DataForge(error.clone()), name.span())
            }))
        });
        let nominal = nominal.transpose()?;
        let mut plans: Vec<Option<Vec<Option<u32>>>> = selected
            .iter()
            .map(|selected| selected.then(Vec::new))
            .collect();
        for path in &self.select {
            let representative = resolve(&structs, nominal, path, self.from.as_ref())?;
            let head = path.head.as_str();
            for (index, plan) in plans.iter_mut().enumerate() {
                if let Some(plan) = plan {
                    let found = structs
                        .field(index, head)
                        .filter(FieldDef::is_scalar)
                        .filter(|found| {
                            nominal.is_none() || found.position == representative.position
                        })
                        .map(|found| found.position);
                    plan.push(found);
                }
            }
        }
        Ok(CompiledQuery {
            forge,
            columns: self.select.iter().map(ToString::to_string).collect(),
            heads: self
                .select
                .iter()
                .map(|path| path.head.text.clone())
                .collect(),
            pattern: self.at.as_ref().map(|at| PathPattern::new(at.as_str())),
            plans: plans
                .into_iter()
                .map(|plan| {
                    plan.map(|fields| Plan {
                        fields: fields.into(),
                    })
                })
                .collect(),
        })
    }
}

/// Returns the field the path starts with, from the `from` struct at `nominal` or, without
/// `from`, from the struct that declares it first, and checks that it can be selected.
fn resolve<'f>(
    structs: &StructFields<'f>,
    nominal: Option<usize>,
    path: &Path,
    from: Option<&Name>,
) -> Result<FieldDef<'f>, QueryError> {
    let head = &path.head;
    let owner = from.map(|name| name.as_str().to_owned());
    let found = nominal.map_or_else(
        || structs.representative(head.as_str()),
        |index| structs.field(index, head.as_str()),
    );
    let Some(found) = found else {
        let suggestion = nominal.map_or_else(
            || closest(head.as_str(), structs.all_field_names()),
            |index| closest(head.as_str(), structs.field_names(index)),
        );
        return Err(QueryError::new(
            QueryErrorKind::UnknownField {
                name: head.as_str().to_owned(),
                owner,
                suggestion,
            },
            head.span(),
        ));
    };
    check_kind(path, &found, owner.as_deref())?;
    Ok(found)
}
/// Returns, per struct, whether it is the named struct or derives from it.
fn select_type(structs: &StructFields<'_>, name: &Name) -> Result<Vec<bool>, QueryError> {
    let targets: Vec<usize> = structs.positions_of(name.as_str()).collect();
    if targets.is_empty() {
        return Err(QueryError::new(
            QueryErrorKind::UnknownType {
                name: name.as_str().to_owned(),
                suggestion: closest(name.as_str(), structs.names()),
            },
            name.span(),
        ));
    }
    Ok((0..structs.len())
        .map(|index| ancestors_include(structs.forge(), index, &targets))
        .collect())
}

/// Checks that the path ends at its first field and that the field holds one scalar value.
fn check_kind(path: &Path, field: &FieldDef<'_>, owner: Option<&str>) -> Result<(), QueryError> {
    let name = &path.head;
    let next = path.tail.first();
    let Some(data_type) = field.data_type else {
        return Err(QueryError::new(
            QueryErrorKind::DataForge(crate::DataForgeError::UnknownDataType { code: field.code }),
            name.span(),
        ));
    };
    if field.array || !is_scalar(data_type) {
        return Err(QueryError::new(
            QueryErrorKind::UnsupportedField {
                name: name.as_str().to_owned(),
                owner: owner.map(str::to_owned),
                data_type,
                array: field.array,
            },
            name.span(),
        ));
    }
    if let Some(Segment::Field(next)) = next {
        return Err(QueryError::new(
            QueryErrorKind::ScalarHasNoFields {
                name: name.as_str().to_owned(),
                owner: owner.map(str::to_owned),
                data_type,
                next: next.as_str().to_owned(),
            },
            next.span(),
        ));
    }
    Ok(())
}

/// Returns whether values of `data_type` are scalars a column can hold.
pub(crate) const fn is_scalar(data_type: DataType) -> bool {
    !matches!(
        data_type,
        DataType::Class | DataType::StrongPointer | DataType::WeakPointer | DataType::Reference
    )
}
