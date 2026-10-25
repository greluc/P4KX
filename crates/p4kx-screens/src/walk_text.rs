// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::rc::Rc;

use crate::canvas::Canvas;
use crate::style::Modifiers;
use crate::text::FontStyleInfo;
use crate::walk::Walk;
use crate::widget::Widget;

impl Walk<'_, '_, '_, '_> {
    pub(crate) fn font_of(
        &mut self,
        widget: &Widget,
        style: &Modifiers,
    ) -> Option<Rc<FontStyleInfo>> {
        let spec = widget.text.as_ref()?;
        let guid = style.text.font_style.or(spec.font_style)?;
        self.cx.texts.styles.get(self.store.forge(), guid)
    }

    pub(crate) fn bound_text(&self, canvas: &Canvas, index: usize) -> Option<String> {
        let binding = canvas.text_bindings.get(&index)?;
        let given = self.params.last().and_then(|values| {
            values
                .iter()
                .find(|(name, _)| *name == binding.parameter)
                .map(|(_, value)| value.clone())
        });
        given.or_else(|| binding.default.clone())
    }
}
