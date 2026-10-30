// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::Rest;
use super::eval::AtRest;
use super::graph::{Arith, Binding, Cmp, FieldKind, Node, Operations};
use crate::variables::Variable;

fn binding(field: &str, kind: FieldKind, input: usize) -> Binding {
    Binding {
        widget: 0,
        field: field.to_owned(),
        kind,
        input: Some(input),
    }
}

fn graph(nodes: Vec<Node>, bindings: Vec<Binding>) -> Operations {
    Operations {
        nodes,
        bindings,
        kinds: vec!["Mystery".to_owned()],
    }
}

fn variable(binding: &str, zero: Rest) -> Node {
    Node::Variable {
        binding: binding.to_owned(),
        zero,
    }
}

fn flag() -> Node {
    variable("flag", Rest::Bool(false))
}

#[test]
fn a_cycle_is_unknown_and_does_not_loop() {
    let operations = graph(
        vec![Node::Invert(Some(1)), Node::Invert(Some(0))],
        vec![binding("IsActive", FieldKind::Boolean, 0)],
    );
    let state = AtRest::of(&operations, &[], &[]);
    assert_eq!(state.active(0), None);
    assert_eq!(state.unknown().len(), 1);
}

#[test]
fn a_chain_deeper_than_the_limit_is_unknown() {
    let mut nodes = vec![flag()];
    nodes.extend((0..400).map(|at| Node::Invert(Some(at))));
    let operations = graph(nodes, vec![binding("IsActive", FieldKind::Boolean, 400)]);
    let state = AtRest::of(&operations, &[], &[]);
    assert_eq!(state.active(0), None);
    assert!(
        state
            .unknown()
            .iter()
            .any(|(_, _, why)| why.contains("too deep"))
    );
}

#[test]
fn a_chain_within_the_limit_is_evaluated() {
    let mut nodes = vec![flag()];
    nodes.extend((0..100).map(|at| Node::Invert(Some(at))));
    let operations = graph(nodes, vec![binding("IsActive", FieldKind::Boolean, 100)]);
    assert_eq!(AtRest::of(&operations, &[], &[]).active(0), Some(false));
}

#[test]
fn an_input_that_does_not_exist_is_unknown() {
    let operations = graph(
        vec![Node::Invert(Some(7))],
        vec![binding("IsActive", FieldKind::Boolean, 0)],
    );
    assert_eq!(AtRest::of(&operations, &[], &[]).active(0), None);
}

#[test]
fn a_number_that_is_not_finite_is_unknown() {
    let operations = graph(
        vec![
            variable("level", Rest::Num(0.0)),
            Node::Arith {
                op: Arith::Div,
                amount: 0.0,
                left: Some(0),
                right: None,
                whole: false,
            },
            Node::Compare {
                cmp: Cmp::Equal,
                constant: 0.0,
                left: Some(1),
                right: None,
            },
        ],
        vec![binding("IsActive", FieldKind::Boolean, 2)],
    );
    assert_eq!(AtRest::of(&operations, &[], &[]).active(0), None);
}

#[test]
fn no_number_that_is_not_finite_reaches_an_alpha_or_a_parameter() {
    let operations = graph(
        vec![
            Node::Param {
                parameter: "ParamInput0".to_owned(),
                default: Rest::Num(f64::NAN),
            },
            Node::Same(Some(0)),
            Node::Choose {
                numbers: [0.0, f64::INFINITY],
                input: Some(3),
                branches: [None, None],
                whole: false,
            },
            flag(),
        ],
        vec![
            binding("Alpha", FieldKind::Number, 1),
            binding("ParamInput1", FieldKind::Number, 1),
            binding("Alpha", FieldKind::Number, 2),
        ],
    );
    let state = AtRest::of(&operations, &[], &[]);
    assert_eq!(state.alpha(0), None);
    assert_eq!(state.typed(0), []);
    assert_eq!(state.unknown().len(), 3);
    let clamped = graph(
        vec![
            variable("level", Rest::Num(0.5)),
            Node::Clamp {
                bounds: [f64::NEG_INFINITY, f64::INFINITY],
                input: Some(0),
                inputs: [None, None],
            },
        ],
        vec![binding("Alpha", FieldKind::Number, 1)],
    );
    assert_eq!(AtRest::of(&clamped, &[], &[]).alpha(0), Some(0.5));
}

#[test]
fn an_alpha_is_held_between_zero_and_one() {
    let operations = graph(
        vec![
            Node::Choose {
                numbers: [9.0, -3.0],
                input: Some(1),
                branches: [None, None],
                whole: false,
            },
            flag(),
        ],
        vec![binding("Alpha", FieldKind::Number, 0)],
    );
    assert_eq!(AtRest::of(&operations, &[], &[]).alpha(0), Some(0.0));
}

#[test]
fn the_deciding_input_of_and_and_or_wins_over_an_unknown_one() {
    let operations = graph(
        vec![
            flag(),
            Node::Unsupported(0),
            Node::All(vec![Some(0), Some(1)]),
            Node::Any(vec![Some(0), Some(1)]),
        ],
        vec![
            binding("IsActive", FieldKind::Boolean, 2),
            binding("Instantiated", FieldKind::Boolean, 3),
        ],
    );
    let state = AtRest::of(&operations, &[], &[]);
    assert_eq!(state.active(0), Some(false), "false and unknown");
    assert_eq!(state.instantiated(0), None, "false or unknown");
}

#[test]
fn a_parameter_of_another_kind_than_the_default_is_ignored() {
    let operations = graph(
        vec![Node::Param {
            parameter: "ParamInput0".to_owned(),
            default: Rest::Bool(true),
        }],
        vec![binding("IsActive", FieldKind::Boolean, 0)],
    );
    let wrong = [("ParamInput0".to_owned(), Rest::Num(0.0))];
    assert_eq!(AtRest::of(&operations, &wrong, &[]).active(0), Some(true));
    let right = [("ParamInput0".to_owned(), Rest::Bool(false))];
    assert_eq!(AtRest::of(&operations, &right, &[]).active(0), Some(false));
}

#[test]
fn a_variable_that_is_set_replaces_its_zero_and_the_last_setting_wins() {
    let operations = graph(
        vec![
            variable("PowerState", Rest::Num(0.0)),
            Node::Compare {
                cmp: Cmp::Equal,
                constant: 1.0,
                left: Some(0),
                right: None,
            },
        ],
        vec![binding("IsActive", FieldKind::Boolean, 1)],
    );
    let set = |value: &str| vec![Variable::new("powerstate", value).expect("variable")];
    assert_eq!(
        AtRest::of(&operations, &[], &set("1")).active(0),
        Some(true)
    );
    assert_eq!(
        AtRest::of(&operations, &[], &set("2")).active(0),
        Some(false)
    );
    let twice = [set("0"), set("1")].concat();
    assert_eq!(AtRest::of(&operations, &[], &twice).active(0), Some(true));
    let other = [Variable::new("other", "1").expect("variable")];
    assert_eq!(AtRest::of(&operations, &[], &other).active(0), Some(false));
}

#[test]
fn a_variable_that_does_not_fit_its_kind_is_unknown() {
    let operations = graph(
        vec![variable("PowerState", Rest::Num(0.0)), Node::Same(Some(0))],
        vec![binding("Alpha", FieldKind::Number, 1)],
    );
    let word = [Variable::new("powerstate", "on").expect("variable")];
    let state = AtRest::of(&operations, &[], &word);
    assert_eq!(state.alpha(0), None);
    assert!(
        state
            .unknown()
            .iter()
            .any(|(_, _, why)| why.contains("powerstate") && why.contains("not a number"))
    );
}

#[test]
fn parameters_tell_states_apart_by_key() {
    let none = AtRest::of(&graph(Vec::new(), Vec::new()), &[], &[]);
    let some = AtRest::of(
        &graph(Vec::new(), Vec::new()),
        &[("ParamInput0".to_owned(), Rest::Bool(true))],
        &[],
    );
    assert_eq!(none.key(), 0);
    assert_ne!(some.key(), 0);
}
