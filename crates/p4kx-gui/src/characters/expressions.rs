// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_assembly::Armour;

use super::CharactersTab;

/// One expression of the head: a morph target the view can apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    part: usize,
    target: usize,
    name: String,
}

impl Expression {
    /// Returns the name of the expression, e.g. `CTL_jaw_open`.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Which expressions the head carries and which one the view applies.
#[derive(Debug, Clone)]
pub(super) struct Choice {
    pub(super) enabled: bool,
    pub(super) list: Vec<Expression>,
    pub(super) chosen: Option<usize>,
    pub(super) weight: f32,
}

impl Default for Choice {
    fn default() -> Self {
        Self {
            enabled: false,
            list: Vec::new(),
            chosen: None,
            weight: 1.0,
        }
    }
}

/// Lists the morph targets of the parts of `armour`.
pub(super) fn listed(armour: &Armour) -> Vec<Expression> {
    armour
        .parts()
        .iter()
        .enumerate()
        .flat_map(|(part, one)| {
            one.morphs()
                .iter()
                .enumerate()
                .map(move |(target, morph)| Expression {
                    part,
                    target,
                    name: morph.name.clone(),
                })
        })
        .collect()
}

impl CharactersTab {
    /// Returns whether the expressions of the head are loaded with the Character.
    #[must_use]
    pub const fn expressions_enabled(&self) -> bool {
        self.expressions.enabled
    }

    /// Loads the expressions of the head with the Character, or leaves them out; building the
    /// Character again is what loads them.
    pub fn set_expressions_enabled(&mut self, enabled: bool) {
        if self.expressions.enabled != enabled {
            self.expressions.enabled = enabled;
            if !enabled {
                self.expressions.chosen = None;
            }
            self.rebuild();
        }
    }

    /// Returns the expressions of the head, once loaded.
    #[must_use]
    pub fn expressions(&self) -> &[Expression] {
        &self.expressions.list
    }

    /// Returns the position of the expression the view applies, or `None` for the neutral face.
    #[must_use]
    pub const fn expression(&self) -> Option<usize> {
        self.expressions.chosen
    }

    /// Returns how strongly the expression is applied, 0 to 1.
    #[must_use]
    pub const fn expression_weight(&self) -> f32 {
        self.expressions.weight
    }

    /// Applies the expression at `position` of [`Self::expressions`] in the view, or shows the
    /// neutral face for `None`; a position past the list shows the neutral face.
    pub fn choose_expression(&mut self, position: Option<usize>) {
        let chosen = position.filter(|at| *at < self.expressions.list.len());
        if chosen != self.expressions.chosen {
            self.expressions.chosen = chosen;
            self.apply_expression();
        }
    }

    /// Sets how strongly the expression is applied; the value is kept within 0 and 1.
    pub fn set_expression_weight(&mut self, weight: f32) {
        let weight = if weight.is_finite() {
            weight.clamp(0.0, 1.0)
        } else {
            1.0
        };
        if (weight - self.expressions.weight).abs() > f32::EPSILON {
            self.expressions.weight = weight;
            if self.expressions.chosen.is_some() {
                self.apply_expression();
            }
        }
    }

    pub(super) fn apply_expression(&mut self) {
        let chosen = self
            .expressions
            .chosen
            .and_then(|at| self.expressions.list.get(at));
        let Some(shown) = &mut self.shown else {
            return;
        };
        let moved = chosen.and_then(|expression| {
            let span = shown.spans.get(expression.part).copied().flatten()?;
            let morph = shown
                .armour
                .parts()
                .get(expression.part)?
                .morphs()
                .get(expression.target)?;
            Some(shown.base.morphed(
                span,
                (&morph.vertices, &morph.deltas),
                self.expressions.weight,
            ))
        });
        shown.scene = moved.map_or_else(|| Arc::clone(&shown.base), Arc::new);
    }
}
