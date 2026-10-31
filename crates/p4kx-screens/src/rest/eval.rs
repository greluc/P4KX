// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cmp::Ordering;

use p4kx_dataforge::Guid;

use super::Rest;
use super::graph::{FieldKind, Input, Node, Operations};
use super::state::{AtRest, PARAMETER, STATE_TAG, key_of};
use crate::style::GeometryModifiers;
use crate::variables::Variable;

const MAX_DEPTH: usize = 128;
const CANVAS_FIELD: &str = "CanvasReferenceRecord";
const TOO_DEEP: &str = "nested too deep";
const NOT_FINITE: &str = "a number that is not finite";

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
    variables: &'a [Variable],
    memo: Vec<Memo>,
}

impl Eval<'_> {
    fn value(&mut self, at: usize, depth: usize) -> Outcome {
        if depth > MAX_DEPTH {
            return Err(TOO_DEEP.to_owned());
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
        let outcome = nodes
            .get(at)
            .map_or_else(
                || Err("an operation that does not exist".to_owned()),
                |node| self.node(node, depth),
            )
            .and_then(|value| match value {
                Rest::Num(number) if !number.is_finite() => Err(NOT_FINITE.to_owned()),
                value => Ok(value),
            });
        if let Some(slot) = self.memo.get_mut(at) {
            *slot = match &outcome {
                Err(why) if why == TOO_DEEP => Memo::Fresh,
                _ => Memo::Done(outcome.clone()),
            };
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

    /// Evaluates the canvas the node `input` picks.
    fn canvas(&mut self, input: Input, depth: usize) -> Result<Guid, String> {
        let nodes = self.nodes;
        let Some(Node::CanvasChoice { canvases, input }) = input.and_then(|at| nodes.get(at))
        else {
            return Err("a canvas that only the game names at run time".to_owned());
        };
        let at = usize::from(!self.flag(*input, depth + 1)?);
        canvases
            .get(at)
            .copied()
            .flatten()
            .ok_or_else(|| "a branch that names no canvas".to_owned())
    }

    fn node(&mut self, node: &Node, depth: usize) -> Outcome {
        match node {
            Node::Variable { binding, zero } => self
                .variables
                .iter()
                .rev()
                .find(|variable| variable.matches(binding))
                .map_or_else(|| Ok(zero.clone()), |variable| variable.coerce(zero)),
            Node::Param { parameter, default } => Ok(self
                .inputs
                .iter()
                .find(|(name, given)| name == parameter && fits(given, default))
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
            Node::TagChoice { .. } | Node::TagSwitch { .. } => self.tag_node(node, depth),
            other => self.maths(other, depth),
        }
    }

    fn tag_node(&mut self, node: &Node, depth: usize) -> Outcome {
        match node {
            Node::TagChoice {
                tags,
                branches,
                input,
            } => {
                let at = usize::from(!self.flag(*input, depth)?);
                branches.get(at).copied().flatten().map_or_else(
                    || Ok(Rest::Tag(tags.get(at).copied().flatten())),
                    |branch| self.value(branch, depth + 1),
                )
            }
            Node::TagSwitch {
                pairs,
                default,
                input,
            } => {
                let value = self.number(*input, depth)?;
                let tag = pairs
                    .iter()
                    .find(|(first, _)| value.partial_cmp(first).is_some_and(Ordering::is_eq))
                    .map_or(*default, |(_, tag)| *tag);
                Ok(Rest::Tag(tag))
            }
            _ => Err("a node that is not a tag".to_owned()),
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
                Ok(Rest::Num(whole_if(op.apply(left, right), *whole)))
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

/// Returns whether the value `given` for a parameter may stand where `default` is expected: the
/// same kind, or a tag for a string parameter, which is how a nested canvas receives one.
fn fits(given: &Rest, default: &Rest) -> bool {
    std::mem::discriminant(given) == std::mem::discriminant(default)
        || matches!((given, default), (Rest::Tag(_), Rest::Text(_)))
}

/// Reads the tag a node gave: a tag node's tag, or the empty text of a string parameter, which
/// names none.
fn tag_of(value: Rest) -> Result<Option<Guid>, String> {
    match value {
        Rest::Tag(tag) => Ok(tag),
        Rest::Text(text) if text.is_empty() => Ok(None),
        _ => Err("a tag that only the game names at run time".to_owned()),
    }
}

/// Returns whether a binding of the field `field` of the kind `kind` is evaluated at rest.
fn is_evaluated(kind: FieldKind, field: &str) -> bool {
    match (kind, field) {
        (FieldKind::Other, _) => false,
        (FieldKind::Boolean, "Instantiated" | "IsActive") | (FieldKind::Number, "Alpha") => true,
        (FieldKind::Number | FieldKind::Integer, field) if GeometryModifiers::has_field(field) => {
            true
        }
        (_, field) => field.starts_with(PARAMETER),
    }
}

/// Returns why `value` cannot be the value of the layout field `field`; `None` for any other
/// field and for a value it takes.
fn layout_problem(field: &str, value: &Rest) -> Option<&'static str> {
    if !GeometryModifiers::has_field(field) {
        return None;
    }
    match value {
        Rest::Num(number) if GeometryModifiers::takes(field, *number) => None,
        Rest::Num(_) => Some("a number the layout field does not take"),
        _ => Some("a value that is not a number"),
    }
}

const fn whole_if(value: f64, whole: bool) -> f64 {
    if whole { value.trunc() } else { value }
}

impl AtRest {
    /// Evaluates the field bindings of `operations` with the game variables at the values of
    /// `variables`, else at zero, and the component parameters at `inputs`, else at their
    /// defaults.
    pub(crate) fn of(
        operations: &Operations,
        inputs: &[(String, Rest)],
        variables: &[Variable],
    ) -> Self {
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
            variables,
            memo: operations.nodes.iter().map(|_| Memo::Fresh).collect(),
        };
        for binding in &operations.bindings {
            if binding.kind == FieldKind::Text && binding.field == CANVAS_FIELD {
                match eval.canvas(binding.input, 0) {
                    Ok(canvas) => {
                        found.canvases.insert(binding.widget, canvas);
                    }
                    Err(why) => found
                        .unknown
                        .push((binding.widget, binding.field.clone(), why)),
                }
                continue;
            }
            if binding.kind == FieldKind::Text && binding.field.ends_with(STATE_TAG) {
                match eval.input(binding.input, 0).and_then(tag_of) {
                    Ok(tag) => found.tag(binding.widget, tag),
                    Err(why) => found
                        .unknown
                        .push((binding.widget, binding.field.clone(), why)),
                }
                continue;
            }
            if !is_evaluated(binding.kind, &binding.field) {
                found.left_out.push((binding.widget, binding.field.clone()));
                continue;
            }
            match eval.input(binding.input, 0).and_then(|value| {
                layout_problem(&binding.field, &value).map_or(Ok(value), |why| Err(why.to_owned()))
            }) {
                Ok(value) => found.set(binding.widget, &binding.field, value),
                Err(why) => found
                    .unknown
                    .push((binding.widget, binding.field.clone(), why)),
            }
        }
        found
    }
}
