// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{
    Align, Label, Layout, Rect, Role, ScrollArea, Sense, Ui, UiBuilder, WidgetInfo, pos2, vec2,
};

use super::entity_status::{own_status, status_kinds};
use super::tree::rails;
use crate::theme::desktop::{
    CHEVRON, CHIP_PADDING_X, FOCUS_OFFSET, ROW_FONT, ROW_HEIGHT, SCROLLBAR_THIN, SPINNER,
    SPINNER_INLINE, TOGGLE_SIZE,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_2, SPACE_3, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::chip::{self, Tone};
use crate::widgets::focus::focus_ring_square;
use crate::widgets::frames;
use crate::widgets::indicator;
use crate::widgets::rows::{RowState, chevron, focus_inside, paint_row};
use crate::widgets::text::{line, paint_left, plain};
use crate::widgets::toggle::{self, Check};
use crate::widgets::tooltip;
use crate::{
    EntityText, LoadoutRow, LoadoutSession, Model, PortMismatch, ResolveFailure, RowChange, Texts,
};

enum Action {
    Toggle(String),
    Include(String, bool),
    Replace(String),
    Restore(String),
    Hidden(String, bool),
    Reset,
}

/// Shows the Loadout of the selected Entity: its name and changes, then the ports as a tree where
/// a branch can be left out and an Entity replaced.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    let Some(session) = model.entities().loadout() else {
        let text = plain(texts.get(EntityText::LoadoutNone), FS_SM, Weight::Regular)
            .color(palette::GRAY_2_TEXT);
        ui.add(Label::new(text));
        return;
    };
    let action = head(ui, texts, session).or_else(|| body(ui, texts, session));
    let Some(action) = action else {
        return;
    };
    let Some(session) = model.entities_mut().loadout_mut() else {
        return;
    };
    match action {
        Action::Toggle(path) => session.toggle(&path),
        Action::Include(path, included) => {
            let _ignored = session.set_excluded(&path, !included);
        }
        Action::Restore(path) => session.restore(&path),
        Action::Hidden(path, hidden) => session.assembly_mut().set_hidden(&path, hidden),
        Action::Reset => session.reset(),
        Action::Replace(path) => model.entities_mut().begin_replace(&path),
    }
}

fn head(ui: &mut Ui, texts: &Texts, session: &LoadoutSession) -> Option<Action> {
    let mut action = None;
    let (excluded, replaced) = session.change_counts();
    let changes = if excluded + replaced == 0 {
        texts.get(EntityText::NoChanges)
    } else {
        texts.format(
            EntityText::Changes,
            &[
                ("excluded", excluded.to_string().into()),
                ("replaced", replaced.to_string().into()),
            ],
        )
    };
    ui.horizontal(|ui| {
        ui.label(plain(session.name(), FS_SM, Weight::Bold).color(palette::WHITE));
        if session.is_resolving() {
            indicator::spinner(ui, SPINNER_INLINE);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let reset = texts.get(EntityText::Reset);
            let any = excluded + replaced > 0;
            if button::button(ui, Kind::Ghost, Size::Compact, &reset, any).clicked() {
                action = Some(Action::Reset);
            }
            ui.label(plain(changes, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT));
        });
    });
    ui.add_space(SPACE_2);
    action
}

fn failure_alert(ui: &mut Ui, texts: &Texts, failure: &ResolveFailure) {
    let text = match failure {
        ResolveFailure::ArchiveChanged => texts.get(EntityText::LoadoutArchiveChanged),
        ResolveFailure::NoRecord(guid) => texts.format(
            EntityText::LoadoutNoRecord,
            &[("guid", guid.to_string().into())],
        ),
        ResolveFailure::Cancelled => texts.get(EntityText::LoadoutStopped),
        ResolveFailure::Lost => texts.get(EntityText::LoadoutLost),
        ResolveFailure::Failed(reason) => texts.format(
            EntityText::LoadoutFailed,
            &[("reason", reason.clone().into())],
        ),
    };
    frames::danger_alert(ui, |ui| {
        ui.add(Label::new(text).selectable(true));
    });
}

fn body(ui: &mut Ui, texts: &Texts, session: &LoadoutSession) -> Option<Action> {
    if let Some(failure) = session.failure() {
        failure_alert(ui, texts, failure);
    }
    let Some(view) = session.view() else {
        if session.is_resolving() {
            ui.add_space(SPINNER);
            ui.label(
                plain(
                    texts.get(EntityText::LoadoutResolving),
                    FS_SM,
                    Weight::Regular,
                )
                .color(palette::GRAY_1),
            );
        }
        return None;
    };
    mismatches(ui, texts, view.mismatches());
    if view.rows().is_empty() {
        let text = plain(
            texts.get(EntityText::LoadoutNoPorts),
            FS_SM,
            Weight::Regular,
        )
        .color(palette::GRAY_2_TEXT);
        ui.add(Label::new(text));
        return None;
    }
    let shown = session.shown();
    let mut action = None;
    ui.spacing_mut().scroll.bar_width = SCROLLBAR_THIN;
    ui.spacing_mut().item_spacing.y = 0.0;
    ScrollArea::vertical()
        .id_salt("entities-loadout")
        .auto_shrink(false)
        .show_rows(ui, ROW_HEIGHT, shown.len(), |ui, visible| {
            for position in visible {
                let row = shown.get(position).and_then(|at| view.rows().get(*at));
                if let Some(row) = row
                    && let Some(done) = port(ui, texts, session, row)
                {
                    action = Some(done);
                }
            }
        });
    action
}

fn mismatches(ui: &mut Ui, texts: &Texts, mismatches: &[PortMismatch]) {
    if mismatches.is_empty() {
        return;
    }
    frames::warning_alert(ui, |ui| {
        for mismatch in mismatches {
            let text = texts.format(
                EntityText::LoadoutMismatch,
                &[
                    ("entity", mismatch.entity.clone().into()),
                    ("port", mismatch.path.clone().into()),
                    ("reason", mismatch.reason.clone().into()),
                ],
            );
            ui.add(Label::new(text).selectable(true));
        }
    });
    ui.add_space(SPACE_2);
}

fn port(ui: &mut Ui, texts: &Texts, session: &LoadoutSession, row: &LoadoutRow) -> Option<Action> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    paint_row(
        ui.painter(),
        rect,
        RowState::of_tree(false, response.hovered()),
    );
    let mut action = None;
    let cell = rails(ui, rect, row.depth());
    if row.has_children() && expander(ui, texts, session, row, cell) {
        action = Some(Action::Toggle(row.path().to_owned()));
    }
    let excluded = row.change() == RowChange::Excluded;
    let box_rect = Rect::from_min_size(
        pos2(cell.right() + SPACE_2, rect.top()),
        vec2(2.0f32.mul_add(SPACE_2, TOGGLE_SIZE), ROW_HEIGHT),
    );
    let label = texts.format(
        EntityText::Include,
        &[("port", row.port().to_owned().into())],
    );
    let toggled = ui
        .scope_builder(UiBuilder::new().max_rect(box_rect), |ui| {
            let check = if excluded { Check::Off } else { Check::On };
            toggle::checkbox_cell(ui, check, &label)
        })
        .inner;
    if toggled {
        action = Some(Action::Include(row.path().to_owned(), excluded));
    }
    let right = ui
        .scope_builder(
            UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::right_to_left(Align::Center)),
            |ui| actions(ui, (texts, session), row),
        )
        .inner;
    action = action.or(right);
    let left = box_rect.right() + SPACE_2;
    let used = status_width(ui, (texts, session), row);
    let text_rect = Rect::from_min_max(
        pos2(left, rect.top()),
        pos2(
            3.0f32.mul_add(-SPACE_3, rect.right() - used).max(left),
            rect.bottom(),
        ),
    );
    names(ui, text_rect, row);
    focus_inside(ui.painter(), rect, &response);
    action
}

fn names(ui: &Ui, rect: Rect, row: &LoadoutRow) {
    let dim = row.change() == RowChange::Excluded;
    let port_colour = if dim {
        palette::GRAY_3
    } else {
        palette::GRAY_2_TEXT
    };
    let port = line(
        ui,
        plain(row.port(), ROW_FONT, Weight::Regular),
        rect.width(),
    );
    let port_right = rect.left() + port.size().x;
    paint_left(ui, rect, port, port_colour);
    if let Some(entity) = row.entity() {
        let tail = rect.with_min_x(port_right + SPACE_3);
        let colour = if dim {
            palette::GRAY_2_TEXT
        } else {
            palette::WHITE
        };
        let galley = line(
            ui,
            plain(entity, ROW_FONT, Weight::Bold),
            tail.width().max(0.0),
        );
        paint_left(ui, tail, galley, colour);
    }
}

fn statuses((texts, session): (&Texts, &LoadoutSession), row: &LoadoutRow) -> Vec<(String, Tone)> {
    let own = own_status(
        row.change(),
        row.is_docked(),
        row.entity().is_none() && !row.has_children(),
    );
    let hidden = session.assembly().is_hidden(row.path());
    status_kinds(row.unresolved().is_some(), hidden, row.change(), own)
        .into_iter()
        .map(|(text, tone)| (texts.get(text), tone))
        .collect()
}

fn hide_text(session: &LoadoutSession, row: &LoadoutRow) -> Option<EntityText> {
    if row.entity().is_none() || row.change() == RowChange::Excluded {
        return None;
    }
    Some(if session.assembly().is_hidden(row.path()) {
        EntityText::Show
    } else {
        EntityText::Hide
    })
}

fn status_width(ui: &Ui, (texts, session): (&Texts, &LoadoutSession), row: &LoadoutRow) -> f32 {
    let chips = statuses((texts, session), row);
    let between = u8::try_from(chips.len().saturating_sub(1)).unwrap_or(0);
    let gaps = f32::from(between) * ui.spacing().item_spacing.x;
    let chip = gaps
        + chips
            .iter()
            .map(|(label, _)| chip_width(&chip::galley(ui, label)))
            .sum::<f32>();
    let width = |text| button::width(ui, Size::Compact, &texts.get(text));
    let hide = hide_text(session, row).map_or(0.0, width);
    let replace = if row.change() == RowChange::Excluded {
        0.0
    } else {
        width(EntityText::Replace)
    };
    let restore = if row.change() == RowChange::Replaced {
        width(EntityText::Restore)
    } else {
        0.0
    };
    chip + hide + replace + restore
}

fn chip_width(galley: &egui::Galley) -> f32 {
    2.0f32.mul_add(CHIP_PADDING_X, galley.size().x)
}

fn actions(
    ui: &mut Ui,
    (texts, session): (&Texts, &LoadoutSession),
    row: &LoadoutRow,
) -> Option<Action> {
    let mut action = None;
    ui.add_space(SPACE_2);
    let excluded = row.change() == RowChange::Excluded;
    if !excluded {
        let label = texts.get(EntityText::Replace);
        if button::button(ui, Kind::Ghost, Size::Compact, &label, true).clicked() {
            action = Some(Action::Replace(row.path().to_owned()));
        }
    }
    if row.change() == RowChange::Replaced {
        let label = texts.get(EntityText::Restore);
        if button::button(ui, Kind::Ghost, Size::Compact, &label, true).clicked() {
            action = Some(Action::Restore(row.path().to_owned()));
        }
    }
    if let Some(text) = hide_text(session, row) {
        let label = texts.get(text);
        if button::button(ui, Kind::Ghost, Size::Compact, &label, true).clicked() {
            let hidden = text == EntityText::Hide;
            action = Some(Action::Hidden(row.path().to_owned(), hidden));
        }
    }
    for (label, tone) in statuses((texts, session), row) {
        let galley = chip::galley(ui, &label);
        let (slot, response) =
            ui.allocate_exact_size(vec2(chip_width(&galley), ROW_HEIGHT), Sense::hover());
        chip::paint(ui, pos2(slot.left(), slot.center().y), galley, tone);
        if let Some(reason) = row.unresolved() {
            tooltip::hover(response, |ui| {
                tooltip::line(ui, reason);
            });
        }
    }
    action
}

fn expander(
    ui: &Ui,
    texts: &Texts,
    session: &LoadoutSession,
    row: &LoadoutRow,
    cell: Rect,
) -> bool {
    let open = !session.is_collapsed(row.path());
    let text = if open {
        EntityText::CollapsePort
    } else {
        EntityText::ExpandPort
    };
    let id = ui.scope_id().with(("entities-toggle", row.path()));
    let response = ui.interact(cell, id, Sense::click());
    response.widget_info(|| {
        let name = texts.format(text, &[("name", row.port().to_owned().into())]);
        WidgetInfo::labeled(Role::Button, true, &name)
    });
    let centre = pos2(cell.left() + CHEVRON / 2.0, cell.center().y);
    chevron(ui.painter(), centre, CHEVRON, open, palette::PRIMARY);
    if response.has_focus() {
        let mark = Rect::from_center_size(centre, vec2(CHEVRON, CHEVRON));
        focus_ring_square(ui, mark, palette::PRIMARY, FOCUS_OFFSET);
    }
    response.clicked()
}
