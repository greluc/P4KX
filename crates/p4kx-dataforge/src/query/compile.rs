use p4kx_foundation::PathPattern;

use crate::query::path_plan::PathPlan;
use crate::query::predicate::{self, Predicate, Test};
use crate::query::reader::Reader;
use crate::query::schema::{StructFields, ancestors_include};
use crate::query::suggest::closest;
use crate::query::syntax::{Name, Query};
use crate::query::{Comparison, QueryError, QueryErrorKind};
use crate::{DataForge, DataForgeError, DataType};

/// A Query checked against one DataForge and ready to run.
#[derive(Debug)]
pub struct CompiledQuery<'f> {
    pub(crate) forge: &'f DataForge<'f>,
    pub(crate) columns: Vec<String>,
    pub(crate) kinds: Vec<ColumnKind>,
    pub(crate) pattern: Option<PathPattern>,
    pub(crate) predicate: Option<Predicate>,
    pub(crate) plans: Box<[Option<Plan>]>,
}

/// What the cells of one column hold, known before any Record is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnKind {
    multi_valued: bool,
    data_types: Box<[DataType]>,
}

impl ColumnKind {
    /// Returns whether every cell is a list, for a path through `[*]` or `[TypeName]`.
    #[must_use]
    pub const fn is_multi_valued(&self) -> bool {
        self.multi_valued
    }

    /// Returns the types of the scalars the path ends at: one with `from`, without `from` one per
    /// distinct type it resolves to, in struct order.
    #[must_use]
    pub fn data_types(&self) -> &[DataType] {
        &self.data_types
    }
}

/// How the columns and the compared values are read from the Records of one selected struct.
#[derive(Debug, Clone)]
pub(crate) enum Plan {
    /// Per column, how its path is read, or `None` when the path does not resolve in the struct;
    /// per comparison of the condition, how its value is tested, or `None` when the value is
    /// missing.
    Read {
        columns: Box<[Option<Reader>]>,
        tests: Box<[Option<Test>]>,
    },
    /// The struct's layout cannot be read, so neither can its Records' values.
    Broken(DataForgeError),
}

impl<'f> CompiledQuery<'f> {
    /// Returns the headings of the columns after the record path: each path in canonical form.
    #[must_use]
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Returns what the cells of each column after the record path hold, in column order.
    #[must_use]
    pub fn column_kinds(&self) -> &[ColumnKind] {
        &self.kinds
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
    /// Returns a [`QueryError`] with the span of an unknown type or field name, of a path that
    /// does not end at a scalar or continues where the value has no fields or no elements, of a
    /// `[TypeName]` no element can have, of a `from` type whose layout cannot be read, of a
    /// comparison whose literal does not fit the value's type, or of a number too large in
    /// magnitude to compare.
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
        let columns = self
            .select
            .iter()
            .map(|path| PathPlan::of(&structs, nominal, path, from))
            .collect::<Result<Vec<_>, _>>()?;
        let (predicate, comparisons) = self.condition.as_ref().map(Predicate::of).unzip();
        let compared = comparisons
            .unwrap_or_default()
            .into_iter()
            .map(|comparison| compared(&structs, nominal, comparison, from))
            .collect::<Result<Vec<_>, _>>()?;
        let plans = selected
            .iter()
            .enumerate()
            .map(|(index, selected)| selected.then(|| plan(&structs, index, &columns, &compared)))
            .collect();
        let kinds = self
            .select
            .iter()
            .zip(&columns)
            .map(|(path, plan)| {
                let mut data_types: Vec<DataType> = Vec::new();
                for leaf in plan.leaves() {
                    if !data_types.contains(&leaf) {
                        data_types.push(leaf);
                    }
                }
                ColumnKind {
                    multi_valued: path.is_multi_valued(),
                    data_types: data_types.into_boxed_slice(),
                }
            })
            .collect();
        Ok(CompiledQuery {
            forge,
            kinds,
            columns: self.select.iter().map(ToString::to_string).collect(),
            pattern: self.at.as_ref().map(|at| PathPattern::new(at.as_str())),
            predicate,
            plans,
        })
    }
}

/// Resolves the path a comparison reads and checks that a value it reaches can compare with
/// the literal.
fn compared<'c>(
    structs: &StructFields<'_>,
    nominal: Option<usize>,
    comparison: &'c Comparison,
    from: Option<&Name>,
) -> Result<(&'c Comparison, PathPlan), QueryError> {
    let plan = PathPlan::of(structs, nominal, &comparison.path, from)?;
    predicate::check(comparison, from.map(Name::as_str), plan.leaves())?;
    Ok((comparison, plan))
}

/// Returns how the Records of the struct at `index` give the `columns` and the values of the
/// `compared` paths.
fn plan(
    structs: &StructFields<'_>,
    index: usize,
    columns: &[PathPlan],
    compared: &[(&Comparison, PathPlan)],
) -> Plan {
    let fields = match structs.fields(index) {
        Ok(fields) => fields,
        Err(error) => return Plan::Broken(error.clone()),
    };
    Plan::Read {
        columns: columns
            .iter()
            .map(|column| column.reader(fields).map(|(reader, _)| reader))
            .collect(),
        tests: compared
            .iter()
            .map(|(comparison, path)| Test::of(comparison, path.reader(fields)?))
            .collect(),
    }
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
