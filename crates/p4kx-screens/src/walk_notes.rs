// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::geometry::{Geometry, Overflow};
use crate::walk::Walk;
use crate::widget::Widget;

impl Walk<'_, '_, '_, '_> {
    pub(crate) fn note_approximations(&mut self, widget: &Widget, geometry: &Geometry) {
        let overlay = widget
            .other_layout
            .as_ref()
            .map(|layout| format!("{layout} children are laid out as an overlay"));
        let transformed = self.turned > 0;
        let checks = [
            (overlay.is_some(), overlay.as_deref().unwrap_or_default()),
            (
                matches!(widget.overflow, Overflow::Fade | Overflow::ClipFade),
                "fading edges of the overflow are not drawn",
            ),
            (
                !widget.inherits_turn && transformed,
                "the turn and scale of the parent are not undone: they are inherited",
            ),
            (
                widget.gradient && widget.background_enabled,
                "gradient backgrounds are drawn in the plain colour of the background",
            ),
            (
                geometry.invalid,
                "a number of the geometry is not finite: the default is used",
            ),
        ];
        for (applies, reason) in checks {
            if applies {
                self.cx.report.note(&widget.type_name, reason, &widget.name);
            }
        }
    }
}
