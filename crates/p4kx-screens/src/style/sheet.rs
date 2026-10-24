// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::rc::Rc;

use p4kx_dataforge::DataForge;

use super::entry::{Entry, Modifiers, parse_entries};
use super::matching::{Subject, TagPaths, applies};
use crate::canvas::{Canvas, StyleRef};
use crate::reader::items;
use crate::widget::Widget;

#[derive(Debug, Default)]
pub(crate) struct CanvasStyles {
    entries: Vec<Entry>,
}

impl CanvasStyles {
    pub(crate) fn build(forge: &DataForge<'_>, canvas: &Canvas, brand: Option<&str>) -> Self {
        let mut entries = Vec::new();
        for style in &canvas.shared {
            entries.extend(style_entries(forge, style));
        }
        entries.extend(canvas.defaults.iter().cloned());
        if let Some(chosen) = canvas.pick_brand(brand) {
            entries.extend(style_entries(forge, &chosen.style));
            entries.extend(chosen.entries.iter().cloned());
        }
        entries.extend(canvas.embedded.iter().cloned());
        Self { entries }
    }

    pub(crate) fn unsupported(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().flat_map(|entry| {
            entry
                .unsupported
                .iter()
                .map(move |selector| (entry.name.as_str(), selector.as_str()))
        })
    }
}

fn style_entries(forge: &DataForge<'_>, style: &StyleRef) -> Vec<Entry> {
    forge
        .record_by_guid(style.guid)
        .and_then(|record| record.tree().ok())
        .map(|tree| parse_entries(items(tree.root(), "entries")))
        .unwrap_or_default()
}

pub(crate) fn resolve(
    sheets: &[Rc<CanvasStyles>],
    widget: &Widget,
    above: &[Subject],
    tags: &TagPaths,
) -> Modifiers {
    let subject = Subject::of(&widget.type_name, &widget.tags);
    let mut resolved = Modifiers::default();
    let all = sheets
        .iter()
        .flat_map(|sheet| sheet.entries.iter())
        .chain(widget.inline.iter());
    for entry in all {
        if applies(entry, &subject, above, tags) {
            merge(&mut resolved, &entry.modifiers);
        }
    }
    resolved
}

fn merge(into: &mut Modifiers, from: &Modifiers) {
    fn set<T: Clone>(slot: &mut Option<T>, value: Option<&T>) {
        if let Some(value) = value {
            *slot = Some(value.clone());
        }
    }
    set(&mut into.background, from.background.as_ref());
    set(&mut into.fill, from.fill.as_ref());
    set(&mut into.stroke, from.stroke.as_ref());
    set(&mut into.enable_background, from.enable_background.as_ref());
    set(&mut into.active, from.active.as_ref());
    set(&mut into.alpha, from.alpha.as_ref());
    for (slot, value) in into.border_colours.iter_mut().zip(&from.border_colours) {
        set(slot, value.as_ref());
    }
    for (slot, value) in into.border_widths.iter_mut().zip(&from.border_widths) {
        set(slot, value.as_ref());
    }
}
