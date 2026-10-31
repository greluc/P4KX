// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::rc::Rc;

use crate::canvas::Canvas;
use crate::style::{Modifiers, Subject, resolve};
use crate::text::{FontStyleInfo, TEXT, TEXT_FIELD};
use crate::walk::Walk;
use crate::widget::Widget;

impl Walk<'_, '_, '_, '_> {
    /// Resolves the entries that select the text a text field holds: the selectors of the game
    /// name that text `Text` and look at the field as its parent (`Type Text` below a tag).
    /// `None` for any other widget.
    pub(crate) fn text_part(&mut self, widget: &Widget, field: &Subject) -> Option<Modifiers> {
        if widget.type_name != TEXT_FIELD {
            return None;
        }
        let text = Subject::of(TEXT, &[]);
        self.above.push(field.clone());
        let found = resolve(
            &self.sheets,
            widget,
            (&text, &self.above),
            self.tags,
            &mut self.budget,
        );
        self.above.pop();
        Some(found)
    }

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
