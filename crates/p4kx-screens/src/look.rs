// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::colour::ColorSpec;
use crate::style::Modifiers;
use crate::widget::Widget;

#[derive(Debug)]
pub(crate) struct Look<'a> {
    pub(crate) background: Option<&'a ColorSpec>,
    pub(crate) fill: Option<&'a ColorSpec>,
    pub(crate) stroke: Option<&'a ColorSpec>,
    pub(crate) borders: [(f32, Option<&'a ColorSpec>); 4],
}

impl<'a> Look<'a> {
    pub(crate) fn of(widget: &'a Widget, style: &'a Modifiers, unit: f32) -> Self {
        let enabled = style.enable_background.unwrap_or(widget.background_enabled);
        let border = |side: usize| {
            let own = widget.border.get(side);
            let width = style
                .border_widths
                .get(side)
                .copied()
                .flatten()
                .map(|width| width * unit)
                .or_else(|| own.map(|own| own.width))
                .unwrap_or(0.0);
            let colour = style
                .border_colours
                .get(side)
                .and_then(Option::as_ref)
                .or_else(|| own.and_then(|own| own.color.as_ref()));
            (width, colour)
        };
        Self {
            background: if enabled {
                style.background.as_ref().or(widget.background.as_ref())
            } else {
                None
            },
            fill: style.fill.as_ref().or(widget.shape.fill.as_ref()),
            stroke: style.stroke.as_ref().or(widget.shape.stroke.as_ref()),
            borders: [border(0), border(1), border(2), border(3)],
        }
    }
}
