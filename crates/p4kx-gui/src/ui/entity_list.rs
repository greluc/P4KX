// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Id, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};
use fluent_bundle::FluentValue;
use p4kx_extraction::Guid;

use super::tree::rails;
use crate::theme::desktop::{
    CHANNEL_GAP, CHEVRON, CHIP_PADDING_X, FOCUS_OFFSET, ROW_FONT, ROW_HEIGHT, SCROLLBAR_THIN,
};
use crate::theme::palette;
use crate::theme::tokens::SPACE_2;
use crate::widgets::chip::{self, Tone};
use crate::widgets::focus::focus_ring_square;
use crate::widgets::rows::{RowState, chevron, focus_inside, paint_row, row_weight};
use crate::widgets::text::{line, paint_left, plain};
use crate::widgets::tooltip;
use crate::{EntityList, EntityRole, EntityRow, EntityText, Texts, format};

/// What the user did in the list of Entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    /// Clicked the row at this position.
    Select(usize),
    /// Asked to list or fold the variants of the base at this position.
    Toggle(usize),
}

/// Shows the Entities of the picker as rows with the class name and the record path, the variants
/// of a base below it and the number of them beside it; returns what the user did.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    list: &EntityList,
    selected: Option<Guid>,
) -> Option<Action> {
    let mut done = None;
    ui.spacing_mut().scroll.bar_width = SCROLLBAR_THIN;
    ui.spacing_mut().item_spacing.y = 0.0;
    ScrollArea::vertical()
        .id_salt("entities-rows")
        .auto_shrink(false)
        .show_rows(ui, ROW_HEIGHT, list.shown_len(), |ui, visible| {
            for position in visible {
                if let Some(action) = row(ui, texts, (list, position), selected) {
                    done = Some(action);
                }
            }
        });
    done
}

fn row(
    ui: &mut Ui,
    texts: &Texts,
    (list, position): (&EntityList, usize),
    selected: Option<Guid>,
) -> Option<Action> {
    let row = list.row(position)?;
    let entity = row.entity();
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let active = selected == Some(entity.guid());
    response
        .widget_info(|| WidgetInfo::selected(Role::Button, true, active, row_label(texts, &row)));
    let state = RowState::of_list(active, response.hovered());
    paint_row(ui.painter(), rect, state);
    let (_, colour) = state.colours();
    let cell = rails(ui, rect, usize::from(row.role() == EntityRole::Variant));
    let mut action = None;
    let mut right = rect.right() - SPACE_2;
    if let EntityRole::Base { variants, expanded } = row.role() {
        let id = ui.scope_id().with(("variants-toggle", entity.guid()));
        if expander(ui, texts, (id, cell), expanded, entity.class_name()) {
            action = Some(Action::Toggle(position));
        }
        let galley = chip::galley(ui, &count_text(texts, variants));
        let width = 2.0f32.mul_add(CHIP_PADDING_X, galley.size().x);
        right -= width;
        chip::paint(ui, pos2(right, rect.center().y), galley, Tone::Muted);
        right -= CHANNEL_GAP;
    }
    let inner = Rect::from_min_max(
        pos2(cell.right() + SPACE_2, rect.top()),
        pos2(right.max(cell.right() + SPACE_2), rect.bottom()),
    );
    let name = line(
        ui,
        plain(entity.class_name(), ROW_FONT, row_weight(active)),
        inner.width().max(0.0),
    );
    let name_right = inner.left() + name.size().x;
    paint_left(ui, inner, name, colour);
    let tail = inner.with_min_x((name_right + CHANNEL_GAP).min(inner.right()));
    let path = line(
        ui,
        plain(
            short_path(entity.record_path()),
            ROW_FONT,
            row_weight(false),
        ),
        tail.width().max(0.0),
    );
    paint_left(ui, tail, path, palette::GRAY_2_TEXT);
    focus_inside(ui.painter(), rect, &response);
    tooltip::hover(response.clone(), |ui| {
        tooltip::line(ui, entity.record_path());
    });
    if action.is_none() && response.clicked() {
        action = Some(Action::Select(position));
    }
    action
}

fn row_label(texts: &Texts, row: &EntityRow<'_>) -> String {
    let name = row.entity().class_name();
    match row.role() {
        EntityRole::Single => name.to_owned(),
        EntityRole::Base { variants, .. } => texts.format(
            EntityText::BaseRow,
            &[
                ("name", name.to_owned().into()),
                ("variants", count_text(texts, variants).into()),
            ],
        ),
        EntityRole::Variant => {
            texts.format(EntityText::VariantRow, &[("name", name.to_owned().into())])
        }
    }
}

fn count_text(texts: &Texts, variants: usize) -> String {
    let count = u64::try_from(variants).unwrap_or(u64::MAX);
    texts.format(
        EntityText::Variants,
        &[
            ("count", FluentValue::from(count)),
            ("number", format::number(count, texts.language()).into()),
        ],
    )
}

fn expander(ui: &Ui, texts: &Texts, (id, cell): (Id, Rect), expanded: bool, name: &str) -> bool {
    let text = if expanded {
        EntityText::HideVariants
    } else {
        EntityText::ShowVariants
    };
    let response = ui.interact(cell, id, Sense::click());
    response.widget_info(|| {
        let label = texts.format(text, &[("name", name.to_owned().into())]);
        WidgetInfo::labeled(Role::Button, true, &label)
    });
    let centre = pos2(cell.left() + CHEVRON / 2.0, cell.center().y);
    chevron(ui.painter(), centre, CHEVRON, expanded, palette::PRIMARY);
    if response.has_focus() {
        let mark = Rect::from_center_size(centre, vec2(CHEVRON, CHEVRON));
        focus_ring_square(ui, mark, palette::PRIMARY, FOCUS_OFFSET);
    }
    response.clicked()
}

/// Returns the record path below `entities/`, which tells the kind and the family apart.
fn short_path(path: &str) -> &str {
    path.split_once("/entities/").map_or(path, |(_, rest)| rest)
}
