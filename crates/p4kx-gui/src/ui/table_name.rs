use egui::{Color32, Label, Rect, Role, Sense, Ui, WidgetInfo, pos2, vec2};

use super::tree::rails;
use crate::theme::desktop::{CHEVRON, FOCUS_OFFSET, ROW_FONT};
use crate::theme::palette;
use crate::theme::tokens::{SPACE_2, Weight};
use crate::widgets::button::focus_ring;
use crate::widgets::rows::chevron;
use crate::widgets::text::{line, plain};
use crate::{Browser, EntryRow, EntryRowKind, GroupText, Text, Texts, format};

/// Shows the name cell of a table row: the chevron slot, the name, and for a texture the number
/// of its parts; parts sit one level in. Returns whether the chevron of a texture was clicked.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    browser: &Browser,
    row: EntryRow,
    (colour, weight): (Color32, Weight),
) -> bool {
    let name = browser.index().name(row.id);
    let rect = ui.max_rect();
    let depth = usize::from(row.kind == EntryRowKind::Part);
    let slot = rails(ui, rect, depth);
    let toggled = matches!(row.kind, EntryRowKind::Texture { .. })
        && toggle(ui, texts, browser, row.id, slot, name);
    ui.add_space(slot.right() - rect.left());
    let count = match row.kind {
        EntryRowKind::Texture { parts } => Some(parts_label(texts, parts)),
        _ => None,
    };
    let count_width = count.as_ref().map_or(0.0, |text| {
        line(ui, plain(text, ROW_FONT, Weight::Regular), f32::INFINITY)
            .size()
            .x
            + SPACE_2
    });
    let room = (rect.right() - ui.cursor().left() - SPACE_2 - count_width).max(0.0);
    ui.scope(|ui| {
        ui.set_max_width(room);
        ui.add(
            Label::new(plain(name, ROW_FONT, weight).color(colour))
                .truncate()
                .selectable(false),
        );
    });
    if let Some(text) = count {
        ui.add_space(SPACE_2);
        ui.add(
            Label::new(plain(text, ROW_FONT, Weight::Regular).color(palette::GRAY_2_TEXT))
                .selectable(false),
        );
    }
    toggled
}

fn parts_label(texts: &Texts, parts: usize) -> String {
    let count = u64::try_from(parts).unwrap_or(u64::MAX);
    texts.format(
        Text::Group(GroupText::Parts),
        &[
            ("count", count.into()),
            ("number", format::number(count, texts.language()).into()),
        ],
    )
}

fn toggle(ui: &Ui, texts: &Texts, browser: &Browser, id: u32, slot: Rect, name: &str) -> bool {
    let expanded = browser.is_texture_expanded(id);
    let text = if expanded {
        GroupText::Collapse
    } else {
        GroupText::Expand
    };
    let response = ui.interact(
        slot,
        ui.scope_id().with(("texture-toggle", id)),
        Sense::click(),
    );
    response.widget_info(|| {
        let label = texts.format(Text::Group(text), &[("name", name.to_owned().into())]);
        WidgetInfo::labeled(Role::Button, true, &label)
    });
    let centre = pos2(slot.left() + CHEVRON / 2.0, slot.center().y);
    chevron(ui.painter(), centre, CHEVRON, expanded, palette::PRIMARY);
    if response.has_focus() {
        let mark = Rect::from_center_size(centre, vec2(CHEVRON, CHEVRON));
        focus_ring(ui, mark, palette::PRIMARY, FOCUS_OFFSET);
    }
    response.clicked()
}
