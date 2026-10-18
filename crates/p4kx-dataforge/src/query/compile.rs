use std::iter;

use p4kx_foundation::PathPattern;

use crate::query::predicate::{self, Predicate, Test};
use crate::query::schema::{FieldDef, StructFields, ancestors_include};
use crate::query::suggest::closest;
use crate::query::syntax::{Name, Path, Query, Segment};
use crate::query::{Comparison, QueryError, QueryErrorKind};
use crate::slot::Slot;
use crate::{DataForge, DataForgeError, Value};

/// A Query checked against one DataForge and ready to run.
#[derive(Debug)]
pub struct CompiledQuery<'f> {
    pub(crate) forge: &'f DataForge<'f>,
    pub(crate) columns: Vec<String>,
    pub(crate) pattern: Option<PathPattern>,
    pub(crate) predicate: Option<Predicate>,
    pub(crate) plans: Box<[Option<Plan>]>,
}

/// How the columns and the compared values are read from the Records of one selected struct.
#[derive(Debug, Clone)]
pub(crate) enum Plan {
    /// Per column, the field's slot and bytes in an instance, or `None` when the struct has no
    /// such scalar field; per comparison of the condition, how its value is tested, or `None` when
    /// the value is missing.
    Read {
        columns: Box<[Option<Column>]>,
        tests: Box<[Option<Test>]>,
    },
    /// The struct's layout cannot be read, so neither can its Records' values.
    Broken(DataForgeError),
}

/// Where one value lies in an instance of a struct.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Column {
    pub(crate) slot: Slot,
    pub(crate) start: u64,
    pub(crate) end: u64,
}

impl Column {
    pub(crate) const fn of(field: &FieldDef<'_>) -> Self {
        Self {
            slot: field.slot,
            start: field.start,
            end: field.end,
        }
    }

    /// Reads the value from the instance `bytes`, or `None` when they are too short.
    pub(crate) fn read<'f>(&self, forge: &'f DataForge<'f>, bytes: &[u8]) -> Option<Value<'f>> {
        let start = usize::try_from(self.start).ok()?;
        let end = usize::try_from(self.end).ok()?;
        Some(self.slot.read(forge, bytes.get(start..end)?))
    }
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
    /// Records are read and checked against the condition.
    ///
    /// # Errors
    ///
    /// Returns a [`QueryError`] with the span of an unknown type or field name, of a field that
    /// holds no single scalar value, of a path that continues behind a scalar field, of a `from`
    /// type whose layout cannot be read, of a comparison whose literal does not fit the field's
    /// type, or of a literal out of the range of the field's type.
    pub fn compile<'f>(&self, forge: &'f DataForge<'f>) -> Result<CompiledQuery<'f>, QueryError> {
        let structs = StructFields::read_all(forge);
        let selected = match &self.from {
            Some(name) => select_type(&structs, name)?,
            None => vec![true; structs.len()],
        };
        let nominal = self
            .from
            .as_ref()
            .and_then(|name| {
                let index = structs.position_of(name.as_str())?;
                Some(structs.fields(index).map(|_| index).map_err(|error| {
                    QueryError::new(QueryErrorKind::DataForge(error.clone()), name.span())
                }))
            })
            .transpose()?;
        let from = self.from.as_ref();
        let resolved = self
            .select
            .iter()
            .map(|path| resolve(&structs, nominal, path, from))
            .collect::<Result<Vec<_>, _>>()?;
        let (predicate, comparisons) = self.condition.as_ref().map(Predicate::of).unzip();
        let wanted = comparisons
            .unwrap_or_default()
            .into_iter()
            .map(|comparison| compared(&structs, nominal, comparison, from))
            .collect::<Result<Vec<_>, _>>()?;
        let plans = selected
            .iter()
            .enumerate()
            .map(|(index, selected)| {
                selected.then(|| plan(&structs, index, nominal.is_some(), &resolved, &wanted))
            })
            .collect();
        Ok(CompiledQuery {
            forge,
            columns: self.select.iter().map(ToString::to_string).collect(),
            pattern: self.at.as_ref().map(|at| PathPattern::new(at.as_str())),
            predicate,
            plans,
        })
    }
}

/// Checks the field a comparison reads and that it can compare with the literal; returns the
/// comparison with the field's position in the `from` type, if given.
fn compared<'c>(
    structs: &StructFields<'_>,
    nominal: Option<usize>,
    comparison: &'c Comparison,
    from: Option<&Name>,
) -> Result<(&'c Comparison, Option<u32>), QueryError> {
    let field = resolve(structs, nominal, &comparison.path, from)?;
    if let Some(from) = from {
        predicate::check(comparison, Some(from.as_str()), iter::once(field))?;
        return Ok((comparison, Some(field.position)));
    }
    let name = comparison.path.head.as_str();
    predicate::check(comparison, None, predicate::declarations(structs, name))?;
    Ok((comparison, None))
}

/// Returns how the Records of the struct at `index` give the columns of the `resolved` fields
/// and the values of the `wanted` comparisons; with `from`, a field has to stand where it stands
/// in the `from` type.
fn plan(
    structs: &StructFields<'_>,
    index: usize,
    from: bool,
    resolved: &[FieldDef<'_>],
    wanted: &[(&Comparison, Option<u32>)],
) -> Plan {
    let fields = match structs.fields(index) {
        Ok(fields) => fields,
        Err(error) => return Plan::Broken(error.clone()),
    };
    Plan::Read {
        columns: resolved
            .iter()
            .map(|wanted| {
                fields
                    .iter()
                    .find(|field| field.name == wanted.name)
                    .filter(|field| field.is_scalar())
                    .filter(|field| !from || field.position == wanted.position)
                    .map(Column::of)
            })
            .collect(),
        tests: predicate::tests(fields, wanted),
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
    check_kind(path, &found, owner)?;
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
fn check_kind(path: &Path, field: &FieldDef<'_>, owner: Option<String>) -> Result<(), QueryError> {
    let name = &path.head;
    let data_type = field.slot.data_type();
    if !field.is_scalar() {
        return Err(QueryError::new(
            QueryErrorKind::UnsupportedField {
                name: name.as_str().to_owned(),
                owner,
                data_type,
                array: field.slot.is_array(),
            },
            name.span(),
        ));
    }
    match path.tail.first() {
        Some(Segment::Field(next)) => Err(QueryError::new(
            QueryErrorKind::ScalarHasNoFields {
                name: name.as_str().to_owned(),
                owner,
                data_type,
                next: next.as_str().to_owned(),
            },
            next.span(),
        )),
        None => Ok(()),
    }
}
