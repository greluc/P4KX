use crate::DataForge;
use crate::query::compile::Column;
use crate::query::operand::{Operand, OperandError};
use crate::query::schema::{FieldDef, StructFields};
use crate::query::{Comparison, Condition, Operator, QueryError, QueryErrorKind};

/// A compiled `where` condition, the predicate a Record has to meet; each comparison is a test numbered in condition order.
#[derive(Debug, Clone)]
pub(crate) enum Predicate {
    Test(usize, Operator),
    Not(Box<Self>),
    And(Box<[Self]>),
    Or(Box<[Self]>),
}

/// How one comparison reads and compares a value in an instance of one struct.
#[derive(Debug, Clone)]
pub(crate) struct Test {
    pub(crate) column: Column,
    pub(crate) operand: Operand,
}

impl Predicate {
    /// Returns the predicate of `condition` and its comparisons in test order.
    pub(crate) fn of(condition: &Condition) -> (Self, Vec<&Comparison>) {
        let mut comparisons = Vec::new();
        let predicate = Self::numbered(condition, &mut comparisons);
        (predicate, comparisons)
    }

    fn numbered<'c>(condition: &'c Condition, comparisons: &mut Vec<&'c Comparison>) -> Self {
        match condition {
            Condition::Compare(comparison) => {
                comparisons.push(comparison);
                Self::Test(comparisons.len().saturating_sub(1), comparison.operator)
            }
            Condition::Not(inner) => Self::Not(Box::new(Self::numbered(inner, comparisons))),
            Condition::And(parts) => Self::And(
                parts
                    .iter()
                    .map(|part| Self::numbered(part, comparisons))
                    .collect(),
            ),
            Condition::Or(parts) => Self::Or(
                parts
                    .iter()
                    .map(|part| Self::numbered(part, comparisons))
                    .collect(),
            ),
        }
    }

    /// Returns whether the instance `bytes` meets the predicate, with `tests` of its struct; `None`
    /// when that is unknown because a compared value is missing or unreadable.
    pub(crate) fn eval<'f>(
        &self,
        forge: &'f DataForge<'f>,
        tests: &[Option<Test>],
        bytes: &[u8],
    ) -> Option<bool> {
        match self {
            Self::Test(index, operator) => {
                let test = tests.get(*index)?.as_ref()?;
                let value = test.column.read(forge, bytes)?;
                test.operand.compare(*operator, &value)
            }
            Self::Not(inner) => inner.eval(forge, tests, bytes).map(|holds| !holds),
            Self::And(parts) => decide(parts, false, |part| part.eval(forge, tests, bytes)),
            Self::Or(parts) => decide(parts, true, |part| part.eval(forge, tests, bytes)),
        }
    }
}

/// Joins parts with Kleene logic: `decisive` from any part decides, otherwise an unknown part
/// leaves the result unknown.
fn decide(
    parts: &[Predicate],
    decisive: bool,
    eval: impl Fn(&Predicate) -> Option<bool>,
) -> Option<bool> {
    let mut result = Some(!decisive);
    for part in parts {
        match eval(part) {
            Some(holds) if holds == decisive => return Some(decisive),
            Some(_) => {}
            None => result = None,
        }
    }
    result
}

/// Checks that some declaration of the compared field can compare with the literal; `declared`
/// is the field of the `from` type, or without `from` every scalar field of that name.
pub(crate) fn check<'f>(
    comparison: &Comparison,
    owner: Option<&str>,
    mut declared: impl Iterator<Item = FieldDef<'f>>,
) -> Result<(), QueryError> {
    let Some(first) = declared.next() else {
        return Ok(());
    };
    let operand = |field: &FieldDef<'_>| {
        Operand::of(
            field.slot.data_type(),
            comparison.operator,
            &comparison.literal,
        )
    };
    let Err(error) = operand(&first) else {
        return Ok(());
    };
    if declared.any(|field| operand(&field).is_ok()) {
        return Ok(());
    }
    let literal = comparison.literal.span;
    Err(match error {
        OperandError::Incompatible(kind) => QueryError::new(
            QueryErrorKind::IncompatibleComparison {
                name: comparison.path.head.as_str().to_owned(),
                owner: owner.map(str::to_owned),
                data_type: first.slot.data_type(),
                operator: comparison.operator,
                literal: kind,
            },
            comparison.span(),
        ),
        OperandError::OutOfRange => QueryError::new(
            QueryErrorKind::NumberOutOfRange(first.slot.data_type()),
            literal,
        ),
        OperandError::InvalidGuid => QueryError::new(QueryErrorKind::InvalidGuid, literal),
    })
}

/// Returns the tests of the struct whose fields are `fields`: per comparison, the field of the
/// wanted name that is scalar, stands at `position` when given, and can compare with the literal.
pub(crate) fn tests(
    fields: &[FieldDef<'_>],
    comparisons: &[(&Comparison, Option<u32>)],
) -> Box<[Option<Test>]> {
    comparisons
        .iter()
        .map(|(comparison, position)| {
            let field = fields
                .iter()
                .find(|field| field.name == comparison.path.head.as_str())
                .filter(|field| field.is_scalar())
                .filter(|field| position.is_none_or(|position| field.position == position))?;
            let operand = Operand::of(
                field.slot.data_type(),
                comparison.operator,
                &comparison.literal,
            )
            .ok()?;
            Some(Test {
                column: Column::of(field),
                operand,
            })
        })
        .collect()
}

/// Returns the scalar fields named `name` in struct order.
pub(crate) fn declarations<'s, 'f>(
    structs: &'s StructFields<'f>,
    name: &'s str,
) -> impl Iterator<Item = FieldDef<'f>> + 's {
    (0..structs.len())
        .filter_map(move |index| structs.field(index, name))
        .filter(FieldDef::is_scalar)
}
