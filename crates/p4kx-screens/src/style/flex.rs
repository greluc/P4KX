// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::flex::{Align, Flex, Justify, Wrap, align, justify};

/// The settings of a flex container that enumerated modifiers of a style entry replace.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct FlexModifiers {
    pub(crate) direction: Option<(bool, bool)>,
    pub(crate) wrap: Option<(Wrap, bool)>,
    pub(crate) justify: Option<Justify>,
    pub(crate) lines: Option<Justify>,
    pub(crate) align: Option<Align>,
}

impl FlexModifiers {
    /// Takes the enumerated modifier `kind` with `value` when it is one of a flex container.
    pub(crate) fn typed(&mut self, kind: &str, value: &str) {
        match kind {
            "FlexDirection" => {
                self.direction = Some((value.contains("Column"), value.contains("Reverse")));
            }
            "FlexWrap" => {
                let wrap = match value {
                    "Wrap" => Wrap::Yes,
                    "WrapReverse" => Wrap::Reverse,
                    _ => Wrap::No,
                };
                self.wrap = Some((wrap, value == "NoWrapInfinite"));
            }
            "FlexAxisJustification" => self.justify = Some(justify(Some(value))),
            "FlexCrossAxisJustification" => self.lines = Some(justify(Some(value))),
            "FlexItemAlignment" => self.align = Some(align(Some(value))),
            _ => {}
        }
    }

    pub(crate) const fn merge(&mut self, from: Self) {
        const fn set<T: Copy>(slot: &mut Option<T>, value: Option<T>) {
            if value.is_some() {
                *slot = value;
            }
        }
        set(&mut self.direction, from.direction);
        set(&mut self.wrap, from.wrap);
        set(&mut self.justify, from.justify);
        set(&mut self.lines, from.lines);
        set(&mut self.align, from.align);
    }

    /// Replaces what `flex` has with what the modifiers set.
    pub(crate) const fn apply(self, flex: &mut Flex) {
        if let Some((column, reverse)) = self.direction {
            flex.column = column;
            flex.reverse = reverse;
        }
        if let Some((wrap, infinite)) = self.wrap {
            flex.wrap = wrap;
            flex.infinite = infinite;
        }
        if let Some(justify) = self.justify {
            flex.justify = justify;
        }
        if let Some(lines) = self.lines {
            flex.lines = lines;
        }
        if let Some(align) = self.align {
            flex.align = align;
        }
    }
}
