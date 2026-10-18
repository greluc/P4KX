use std::cmp::Ordering;

use crate::query::matching::{contains, like, starts_with};
use crate::query::{Literal, LiteralKind, LiteralValue, Operator};
use crate::{DataType, Guid, Value};

/// The literal of a comparison as a value of the compared field's type, made once per struct.
#[derive(Debug, Clone)]
pub(crate) enum Operand {
    Integer(i128),
    Single(f32),
    Double(f64),
    Text(Box<str>),
    Guid(Guid),
    Bool(bool),
}

/// Why a literal cannot be compared with the values of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperandError {
    Incompatible(LiteralKind),
    OutOfRange,
    InvalidGuid,
}

impl Operand {
    /// Returns the operand that compares values of `data_type` with `literal` by `operator`.
    pub(crate) fn of(
        data_type: DataType,
        operator: Operator,
        literal: &Literal,
    ) -> Result<Self, OperandError> {
        let kind = kind_of(literal);
        let incompatible = Err(OperandError::Incompatible(kind));
        let textual = matches!(
            data_type,
            DataType::String | DataType::Locale | DataType::Enum
        );
        if operator.matches_text() && !textual {
            return incompatible;
        }
        match (data_type, &literal.value) {
            (
                DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64,
                LiteralValue::Number(text),
            ) if kind == LiteralKind::Integer => text
                .parse()
                .map(Self::Integer)
                .map_err(|_| OperandError::OutOfRange),
            (DataType::Single, LiteralValue::Number(text)) => text
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .map(Self::Single)
                .ok_or(OperandError::OutOfRange),
            (DataType::Double, LiteralValue::Number(text)) => text
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(Self::Double)
                .ok_or(OperandError::OutOfRange),
            (_, LiteralValue::Text(text)) if textual => Ok(Self::Text(text.as_str().into())),
            (DataType::Guid, LiteralValue::Text(text)) => text
                .parse()
                .map(Self::Guid)
                .map_err(|_| OperandError::InvalidGuid),
            (DataType::Bool, LiteralValue::Bool(value)) => Ok(Self::Bool(*value)),
            _ => incompatible,
        }
    }

    /// Returns whether `value` compares with the operand by `operator`, or `None` when the value
    /// is unknown: unreadable or of another type.
    pub(crate) fn compare(&self, operator: Operator, value: &Value<'_>) -> Option<bool> {
        let ordering = match (value, self) {
            (Value::Single(value), Self::Single(operand)) => {
                return Some(by_partial_order(operator, value, operand));
            }
            (Value::Double(value), Self::Double(operand)) => {
                return Some(by_partial_order(operator, value, operand));
            }
            (
                Value::String(text) | Value::Locale(text) | Value::Enum(text),
                Self::Text(operand),
            ) => {
                return Some(match operator {
                    Operator::Contains => contains(text, operand),
                    Operator::StartsWith => starts_with(text, operand),
                    Operator::Like => like(text, operand),
                    _ => by_order(operator, (*text).cmp(operand)),
                });
            }
            (Value::Guid(value), Self::Guid(operand)) => {
                value.text_order().cmp(&operand.text_order())
            }
            (Value::Bool(value), Self::Bool(operand)) => value.cmp(operand),
            (value, Self::Integer(operand)) => integer(value)?.cmp(operand),
            _ => return None,
        };
        Some(by_order(operator, ordering))
    }
}

fn kind_of(literal: &Literal) -> LiteralKind {
    match &literal.value {
        LiteralValue::Number(text) if text.contains(['.', 'e', 'E']) => LiteralKind::Fraction,
        LiteralValue::Number(_) => LiteralKind::Integer,
        LiteralValue::Text(_) => LiteralKind::Text,
        LiteralValue::Bool(_) => LiteralKind::Bool,
    }
}

fn integer(value: &Value<'_>) -> Option<i128> {
    Some(match *value {
        Value::Int8(value) => value.into(),
        Value::Int16(value) => value.into(),
        Value::Int32(value) => value.into(),
        Value::Int64(value) => value.into(),
        Value::UInt8(value) => value.into(),
        Value::UInt16(value) => value.into(),
        Value::UInt32(value) => value.into(),
        Value::UInt64(value) => value.into(),
        _ => return None,
    })
}

const fn by_order(operator: Operator, ordering: Ordering) -> bool {
    match operator {
        Operator::Equal => ordering.is_eq(),
        Operator::NotEqual => ordering.is_ne(),
        Operator::Less => ordering.is_lt(),
        Operator::LessOrEqual => ordering.is_le(),
        Operator::Greater => ordering.is_gt(),
        Operator::GreaterOrEqual => ordering.is_ge(),
        Operator::Contains | Operator::StartsWith | Operator::Like => false,
    }
}

/// Compares floats as IEEE 754 does: NaN is unordered, so only `!=` holds for it.
fn by_partial_order<T: PartialOrd>(operator: Operator, value: &T, operand: &T) -> bool {
    match operator {
        Operator::Equal => value == operand,
        Operator::NotEqual => value != operand,
        Operator::Less => value < operand,
        Operator::LessOrEqual => value <= operand,
        Operator::Greater => value > operand,
        Operator::GreaterOrEqual => value >= operand,
        Operator::Contains | Operator::StartsWith | Operator::Like => false,
    }
}
