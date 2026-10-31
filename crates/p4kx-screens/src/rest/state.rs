// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use p4kx_dataforge::Guid;

use super::Rest;
use crate::style::GeometryModifiers;

pub(super) const PARAMETER: &str = "ParamInput";
pub(super) const STATE_TAG: &str = "StateTag";

/// What the `operations` of one canvas give its widgets at rest, for the parameters its
/// `WidgetCanvas` hands it.
#[derive(Debug, Default)]
pub(crate) struct AtRest {
    pub(super) key: u64,
    pub(super) instantiated: HashMap<usize, bool>,
    pub(super) active: HashMap<usize, bool>,
    pub(super) alpha: HashMap<usize, f32>,
    pub(super) canvases: HashMap<usize, Guid>,
    pub(super) typed: HashMap<usize, Vec<(String, Rest)>>,
    pub(super) state_tags: HashMap<usize, Vec<Guid>>,
    pub(super) geometry: HashMap<usize, GeometryModifiers>,
    pub(super) left_out: Vec<(usize, String)>,
    pub(super) unknown: Vec<(usize, String, String)>,
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
            Rest::Tag(tag) => (3_u8, tag).hash(&mut hasher),
        }
    }
    hasher.finish().max(1)
}

impl AtRest {
    pub(super) fn set(&mut self, widget: usize, field: &str, value: Rest) {
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
            (field, Rest::Num(number)) if GeometryModifiers::has_field(field) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a layout number is held as f32 like the fields of a widget"
                )]
                let value = number as f32;
                self.geometry
                    .entry(widget)
                    .or_default()
                    .number(field, Some(value));
            }
            (field, value) if field.starts_with(PARAMETER) => {
                let values = self.typed.entry(widget).or_default();
                values.retain(|(name, _)| name != field);
                values.push((field.to_owned(), value));
            }
            _ => {}
        }
    }

    /// Adds the state tag `tag` of widget `widget`; none adds nothing.
    pub(super) fn tag(&mut self, widget: usize, tag: Option<Guid>) {
        if let Some(tag) = tag {
            let tags = self.state_tags.entry(widget).or_default();
            if !tags.contains(&tag) {
                tags.push(tag);
            }
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

    /// Returns the number fields of the layout (size, anchor, pivot, scale, turn, position,
    /// padding, margin, bounds, border radius) the bindings set on widget `widget`.
    pub(crate) fn geometry(&self, widget: usize) -> Option<&GeometryModifiers> {
        self.geometry.get(&widget)
    }

    /// Returns the canvas the bindings put in the canvas widget `widget`.
    pub(crate) fn canvas(&self, widget: usize) -> Option<Guid> {
        self.canvases.get(&widget).copied()
    }

    /// Returns the parameters the bindings hand to the canvas of the widget `widget`.
    pub(crate) fn typed(&self, widget: usize) -> &[(String, Rest)] {
        self.typed.get(&widget).map_or(&[], Vec::as_slice)
    }

    /// Returns the state tags the bindings give widget `widget`, which its styles select on like
    /// its own tags.
    pub(crate) fn state_tags(&self, widget: usize) -> &[Guid] {
        self.state_tags.get(&widget).map_or(&[], Vec::as_slice)
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
