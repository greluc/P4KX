// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The state a canvas has at rest: what its `operations` bind to `Instantiated`, `IsActive`,
//! `Alpha` and the parameters of nested canvases, evaluated with every game variable at its zero
//! value (ADR-0110).

mod eval;
mod graph;
#[cfg(test)]
mod tests;

use std::rc::Rc;

pub(crate) use eval::{AtRest, key_of};
pub(crate) use graph::Operations;

/// A value of the operations graph.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Rest {
    Bool(bool),
    Num(f64),
    Text(String),
}

/// The typed values a `WidgetCanvas` hands to the component parameters of its canvas.
pub(crate) type RestParams = Rc<[(String, Rest)]>;
