// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use super::Rest;
use super::graph::{FieldKind, Input, Node, Operations};

const MAX_DEPTH: usize = 128;
const PARAMETER: &str = "ParamInput";

type Outcome = Result<Rest, String>;

enum Memo {
    Fresh,
    Busy,
    Done(Outcome),
}

struct Eval<'a> {
    nodes: &'a [Node],
    kinds: &'a [String],
    inputs: &'a [(String, Rest)],
    memo: Vec<Memo>,
}

impl Eval<'_> {
    fn value(&mut self, at: usize, depth: usize) -> Outcome {
        if depth > MAX_DEPTH {
            return Err("nested too deep".to_owned());
        }
        match self.memo.get(at) {
            None => return Err("an operation that does not exist".to_owned()),
            Some(Memo::Done(done)) => return done.clone(),
            Some(Memo::Busy) => return Err("a cycle".to_owned()),
            Some(Memo::Fresh) => {}
        }
        if let Some(slot) = self.memo.get_mut(at) {
            *slot = Memo::Busy;
        }
        let nodes = self.nodes;
        let outcome = nodes.get(at).map_or_else(
            || Err("an operation that does not exist".to_owned()),
            |node| self.node(node, depth),
        );
        if let Some(slot) = self.memo.get_mut(at) {
            *slot = Memo::Done(outcome.clone());
        }
        outcome
    }

    fn input(&mut self, input: Input, depth: usize) -> Outcome {
        input.map_or_else(
            || Err("an input that is not connected".to_owned()),
            |at| self.value(at, depth + 1),
        )
    }

    fn flag(&mut self, input: Input, depth: usize) -> Result<bool, String> {
        match self.input(input, depth)? {
            Rest::Bool(flag) => Ok(flag),
            _ => Err("a value that is not a boolean".to_owned()),
        }
    }

    fn number(&mut self, input: Input, depth: usize) -> Result<f64, String> {
        match self.input(input, depth)? {
            Rest::Num(number) => Ok(number),
            _ => Err("a value that is not a number".to_owned()),
        }
    }

    fn number_or(&mut self, input: Input, constant: f64, depth: usize) -> Result<f64, String> {
        if input.is_some() {
            self.number(input, depth)
        } else {
            Ok(constant)
        }
    }

    fn all(&mut self, inputs: &[Input], wanted: bool, depth: usize) -> Outcome {
        let mut unknown = None;
        for &input in inputs {
            match self.flag(input, depth) {
                Ok(flag) if flag != wanted => return Ok(Rest::Bool(!wanted)),
                Ok(_) => {}
                Err(why) => unknown = Some(why),
            }
        }
        unknown.map_or(Ok(Rest::Bool(wanted)), Err)
    }

    fn node(&mut self, node: &Node, depth: usize) -> Outcome {
        match node {
            Node::Zero(zero) => Ok(zero.clone()),
            Node::Param { parameter, default } => Ok(self
                .inputs
                .iter()
                .find(|(name, given)| {
                    name == parameter
                        && std::mem::discriminant(given) == std::mem::discriminant(default)
                })
                .map_or_else(|| default.clone(), |(_, given)| given.clone())),
            Node::Invert(input) => Ok(Rest::Bool(!self.flag(*input, depth)?)),
            Node::All(inputs) => self.all(inputs, true, depth),
            Node::Any(inputs) => self.all(inputs, false, depth),
            Node::Compare {
                cmp,
                constant,
                left,
                right,
            } => {
                let left = self.number(*left, depth)?;
                let right = self.number_or(*right, *constant, depth)?;
                Ok(Rest::Bool(cmp.test(left, right)))
            }
            Node::Switch {
                default,
                exceptions,
                input,
            } => {
                let value = self.number(*input, depth)?;
                Ok(Rest::Bool(exceptions.contains(&value) != *default))
            }
            Node::TextEmpty(input) => match self.input(*input, depth)? {
                Rest::Text(text) => Ok(Rest::Bool(text.is_empty())),
                _ => Err("a value that is not a text".to_owned()),
            },
            Node::Unsupported(kind) => Err(format!(
                "{} is not evaluated",
                self.kinds.get(*kind).map_or("an operation", String::as_str)
            )),
            other => self.maths(other, depth),
        }
    }

    fn maths(&mut self, node: &Node, depth: usize) -> Outcome {
        match node {
            Node::Choose {
                numbers,
                input,
                branches,
                whole,
            } => {
                let at = usize::from(!self.flag(*input, depth)?);
                let constant = numbers.get(at).copied().unwrap_or(0.0);
                let branch = branches.get(at).copied().flatten();
                let value = self.number_or(branch, constant, depth)?;
                Ok(Rest::Num(whole_if(value, *whole)))
            }
            Node::Same(input) => Ok(Rest::Num(self.number(*input, depth)?)),
            Node::Arith {
                op,
                amount,
                left,
                right,
                whole,
            } => {
                let left = self.number(*left, depth)?;
                let right = self.number_or(*right, *amount, depth)?;
                let result = op.apply(left, right);
                if result.is_finite() {
                    Ok(Rest::Num(whole_if(result, *whole)))
                } else {
                    Err("a number that is not finite".to_owned())
                }
            }
            Node::Clamp {
                bounds,
                input,
                inputs,
            } => {
                let value = self.number(*input, depth)?;
                let low = self.number_or(
                    inputs.first().copied().flatten(),
                    bounds.first().copied().unwrap_or(0.0),
                    depth,
                )?;
                let high = self.number_or(
                    inputs.get(1).copied().flatten(),
                    bounds.get(1).copied().unwrap_or(0.0),
                    depth,
                )?;
                Ok(Rest::Num(value.max(low).min(high)))
            }
            _ => Err("a node that is not a number".to_owned()),
        }
    }
}

const fn whole_if(value: f64, whole: bool) -> f64 {
    if whole { value.trunc() } else { value }
}

/// What the `operations` of one canvas give its widgets at rest, for the parameters its
/// `WidgetCanvas` hands it.
#[derive(Debug, Default)]
pub(crate) struct AtRest {
    key: u64,
    instantiated: HashMap<usize, bool>,
    active: HashMap<usize, bool>,
    alpha: HashMap<usize, f32>,
    typed: HashMap<usize, Vec<(String, Rest)>>,
    left_out: Vec<(usize, String)>,
    unknown: Vec<(usize, String, String)>,
}

/// Returns a number that tells apart the parameters a canvas is entered with.
pub(crate) fn key_of(inputs: &[(String, Rest)]) -> u64 {
    if inputs.is_empty() {
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    for (name, value) in inputs {
        name.hash(&mut hasher);
        match value {
            Rest::Bool(flag) => (0_u8, flag).hash(&mut hasher),
            Rest::Num(number) => (1_u8, number.to_bits()).hash(&mut hasher),
            Rest::Text(text) => (2_u8, text).hash(&mut hasher),
        }
    }
    hasher.finish().max(1)
}

impl AtRest {
    /// Evaluates the field bindings of `operations` with every game variable at its zero value
    /// and the component parameters at `inputs`, else at their defaults.
    pub(crate) fn of(operations: &Operations, inputs: &[(String, Rest)]) -> Self {
        let mut found = Self {
            key: key_of(inputs),
            ..Self::default()
        };
        if operations.bindings.is_empty() {
            return found;
        }
        let mut eval = Eval {
            nodes: &operations.nodes,
            kinds: &operations.kinds,
            inputs,
            memo: operations.nodes.iter().map(|_| Memo::Fresh).collect(),
        };
        for binding in &operations.bindings {
            let handled = match (binding.kind, binding.field.as_str()) {
                (FieldKind::Boolean, "Instantiated" | "IsActive")
                | (FieldKind::Number, "Alpha") => true,
                (_, field) => field.starts_with(PARAMETER),
            };
            if !handled || binding.kind == FieldKind::Other {
                found.left_out.push((binding.widget, binding.field.clone()));
                continue;
            }
            match eval.input(binding.input, 0) {
                Ok(value) => found.set(binding.widget, &binding.field, value),
                Err(why) => found
                    .unknown
                    .push((binding.widget, binding.field.clone(), why)),
            }
        }
        found
    }

    fn set(&mut self, widget: usize, field: &str, value: Rest) {
        match (field, value) {
            ("Instantiated", Rest::Bool(flag)) => {
                self.instantiated.insert(widget, flag);
            }
            ("IsActive", Rest::Bool(flag)) => {
                self.active.insert(widget, flag);
            }
            ("Alpha", Rest::Num(number)) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "an alpha is held between 0 and 1"
                )]
                let alpha = number.clamp(0.0, 1.0) as f32;
                self.alpha.insert(widget, alpha);
            }
            (field, value) if field.starts_with(PARAMETER) => {
                let values = self.typed.entry(widget).or_default();
                values.retain(|(name, _)| name != field);
                values.push((field.to_owned(), value));
            }
            _ => {}
        }
    }

    /// Returns what tells this state apart from the states of the same canvas under other
    /// parameters, 0 for the plain one.
    pub(crate) const fn key(&self) -> u64 {
        self.key
    }

    /// Returns whether the bindings leave widget `widget` instantiated, `None` when they do not
    /// say.
    pub(crate) fn instantiated(&self, widget: usize) -> Option<bool> {
        self.instantiated.get(&widget).copied()
    }

    /// Returns what the bindings make of the `IsActive` of widget `widget`.
    pub(crate) fn active(&self, widget: usize) -> Option<bool> {
        self.active.get(&widget).copied()
    }

    /// Returns what the bindings make of the alpha of widget `widget`.
    pub(crate) fn alpha(&self, widget: usize) -> Option<f32> {
        self.alpha.get(&widget).copied()
    }

    /// Returns the parameters the bindings hand to the canvas of the widget `widget`.
    pub(crate) fn typed(&self, widget: usize) -> &[(String, Rest)] {
        self.typed.get(&widget).map_or(&[], Vec::as_slice)
    }

    /// Returns the bindings of fields that are not evaluated: widget and field.
    pub(crate) fn left_out(&self) -> &[(usize, String)] {
        &self.left_out
    }

    /// Returns the bindings whose inputs could not be evaluated: widget, field and why.
    pub(crate) fn unknown(&self) -> &[(usize, String, String)] {
        &self.unknown
    }
}
