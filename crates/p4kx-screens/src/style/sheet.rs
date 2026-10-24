// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::rc::Rc;

use p4kx_dataforge::{DataForge, Guid};

use super::entry::{Entry, Modifiers, parse_entries};
use super::matching::{Subject, TagPaths, applies};
use crate::canvas::{Canvas, StyleRef};
use crate::reader::items;
use crate::widget::Widget;

pub(crate) type StyleCache = HashMap<Guid, Rc<[Entry]>>;

#[derive(Debug, Default)]
pub(crate) struct CanvasStyles {
    groups: Vec<Rc<[Entry]>>,
}

impl CanvasStyles {
    pub(crate) fn build(
        forge: &DataForge<'_>,
        cache: &mut StyleCache,
        canvas: &Canvas,
        brand: Option<&str>,
    ) -> Self {
        let mut groups = Vec::new();
        for style in &canvas.shared {
            groups.push(style_entries(forge, cache, style));
        }
        groups.push(Rc::clone(&canvas.defaults));
        if let Some(chosen) = canvas.pick_brand(brand) {
            groups.push(style_entries(forge, cache, &chosen.style));
            groups.push(Rc::clone(&chosen.entries));
        }
        groups.push(Rc::clone(&canvas.embedded));
        Self { groups }
    }

    pub(crate) fn unsupported(&self) -> impl Iterator<Item = (&str, &str)> {
        self.groups.iter().flat_map(|group| {
            group.iter().flat_map(|entry| {
                entry
                    .unsupported
                    .iter()
                    .map(move |selector| (entry.name.as_str(), selector.as_str()))
            })
        })
    }
}

fn style_entries(forge: &DataForge<'_>, cache: &mut StyleCache, style: &StyleRef) -> Rc<[Entry]> {
    if let Some(cached) = cache.get(&style.guid) {
        return Rc::clone(cached);
    }
    let entries: Rc<[Entry]> = forge
        .record_by_guid(style.guid)
        .and_then(|record| record.tree().ok())
        .map(|tree| parse_entries(items(tree.root(), "entries")))
        .unwrap_or_default()
        .into();
    cache.insert(style.guid, Rc::clone(&entries));
    entries
}

pub(crate) fn resolve(
    sheets: &[Rc<CanvasStyles>],
    widget: &Widget,
    above: &[Subject],
    tags: &TagPaths,
    budget: &mut u64,
) -> Modifiers {
    let subject = Subject::of(&widget.type_name, &widget.tags);
    let mut resolved = Modifiers::default();
    let all = sheets
        .iter()
        .flat_map(|sheet| sheet.groups.iter())
        .flat_map(|group| group.iter())
        .chain(widget.inline.iter());
    for entry in all {
        if *budget == 0 {
            break;
        }
        if applies(entry, &subject, above, tags, budget) {
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
    into.text.merge(&from.text);
    into.picture.merge(&from.picture);
    for (slot, value) in into.border_colours.iter_mut().zip(&from.border_colours) {
        set(slot, value.as_ref());
    }
    for (slot, value) in into.border_widths.iter_mut().zip(&from.border_widths) {
        set(slot, value.as_ref());
    }
}
