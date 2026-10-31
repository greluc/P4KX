// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The comparisons and the arithmetic of the operations graph.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cmp {
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

impl Cmp {
    pub(super) fn parse(name: Option<&str>) -> Option<Self> {
        match name? {
            "Equal" => Some(Self::Equal),
            "NotEqual" => Some(Self::NotEqual),
            "Greater" => Some(Self::Greater),
            "GreaterOrEqual" => Some(Self::GreaterOrEqual),
            "Less" => Some(Self::Less),
            "LessOrEqual" => Some(Self::LessOrEqual),
            _ => None,
        }
    }

    pub(crate) fn test(self, left: f64, right: f64) -> bool {
        let Some(order) = left.partial_cmp(&right) else {
            return false;
        };
        match self {
            Self::Equal => order.is_eq(),
            Self::NotEqual => order.is_ne(),
            Self::Greater => order.is_gt(),
            Self::GreaterOrEqual => order.is_ge(),
            Self::Less => order.is_lt(),
            Self::LessOrEqual => order.is_le(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Min,
    Max,
}

impl Arith {
    pub(super) fn parse(name: Option<&str>) -> Option<Self> {
        match name? {
            "Add" => Some(Self::Add),
            "Sub" => Some(Self::Sub),
            "Mul" => Some(Self::Mul),
            "Div" => Some(Self::Div),
            "Mod" => Some(Self::Mod),
            "Pow" => Some(Self::Pow),
            "Min" => Some(Self::Min),
            "Max" => Some(Self::Max),
            _ => None,
        }
    }

    pub(crate) fn apply(self, left: f64, right: f64) -> f64 {
        match self {
            Self::Add => left + right,
            Self::Sub => left - right,
            Self::Mul => left * right,
            Self::Div => left / right,
            Self::Mod => left % right,
            Self::Pow => left.powf(right),
            Self::Min => left.min(right),
            Self::Max => left.max(right),
        }
    }
}
