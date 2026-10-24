// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Label, Layout, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};

use super::search::{eyebrow, field};
use crate::theme::desktop::{
    CHANNEL_GAP, MODAL_GROUP_GAP, PREVIEW_HEIGHT, ROW_FONT, ROW_HEIGHT, TREE_PADDING_LEFT,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_2, Weight};
use crate::widgets::button::Kind;
use crate::widgets::frames::{self, Tone};
use crate::widgets::rows::{RowState, focus_inside, paint_row, row_weight};
use crate::widgets::text::{line, paint_left, paint_right, plain};
use crate::{CharacterText as T, Chooser, ChooserKind, ExtractionText, Model, Texts, format};

enum Action {
    Choose(usize),
    Close,
}

const fn title(kind: ChooserKind) -> T {
    match kind {
        ChooserKind::Piece => T::ChooseTitlePiece,
        ChooserKind::Loadout => T::ChooseTitleLoadout,
        ChooserKind::Head => T::ChooseTitleHead,
    }
}

/// Shows the choice of a piece, a Loadout or a head while it is open: the candidates, to search
/// and pick from.
pub(super) fn show(ui: &Ui, model: &mut Model) {
    let Some(chooser) = model.characters().chooser() else {
        return;
    };
    let texts = model.texts();
    let heading = texts.get(title(chooser.kind()));
    let close_dialog = texts.get(crate::Text::CloseDialog);
    let mut query = chooser.query().to_owned();
    let mut changed = false;
    let shown =
        frames::modal(frames::modal_id("character-chooser"), Tone::Normal).show(ui.ctx(), |ui| {
            ui.set_width(frames::MODAL_WIDE_BODY_WIDTH);
            let closed = frames::head(ui, (&heading, Tone::Normal), true, Some(&close_dialog));
            let hint = texts.get(T::ChooseHint);
            let label = ui.label(eyebrow(&heading));
            changed = field(ui, &mut query, &hint, label.id);
            ui.add_space(SPACE_2);
            counts(ui, texts, chooser);
            ui.add_space(MODAL_GROUP_GAP / 4.0);
            let picked = list(ui, texts, chooser);
            frames::foot_rule(ui);
            let cancel = frames::foot_action(ui, &texts.get(ExtractionText::Close), Kind::Ghost);
            if closed || cancel {
                Some(Action::Close)
            } else {
                picked.map(Action::Choose)
            }
        });
    frames::modal_decoration(
        ui.ctx(),
        shown.response.layer_id,
        shown.response.rect,
        Tone::Normal,
    );
    let close = shown.should_close();
    match shown.inner {
        Some(Action::Choose(position)) => model.characters_mut().choose(position),
        Some(Action::Close) => model.characters_mut().close_chooser(),
        None if close => model.characters_mut().close_chooser(),
        None => {
            if changed && let Some(chooser) = model.characters_mut().chooser_mut() {
                chooser.set_query(&query);
            }
        }
    }
}

fn counts(ui: &mut Ui, texts: &Texts, chooser: &Chooser) {
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let text = texts.format(
        T::ChooseCount,
        &[
            ("number", number(chooser.shown_len()).into()),
            ("total", number(chooser.total()).into()),
        ],
    );
    ui.add(Label::new(
        plain(text, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT),
    ));
}

fn list(ui: &mut Ui, texts: &Texts, chooser: &Chooser) -> Option<usize> {
    if chooser.shown_len() == 0 {
        let text =
            plain(texts.get(T::ChooseNone), FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT);
        ui.add(Label::new(text));
        return None;
    }
    let mut picked = None;
    ui.with_layout(Layout::top_down(Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ScrollArea::vertical()
            .id_salt("character-chooser-rows")
            .max_height(PREVIEW_HEIGHT.min(frames::modal_room(ui)))
            .show_rows(ui, ROW_HEIGHT, chooser.shown_len(), |ui, visible| {
                for position in visible {
                    if let Some(item) = chooser.get(position)
                        && row(ui, item.name(), item.detail())
                    {
                        picked = Some(position);
                    }
                }
            });
    });
    picked
}

fn row(ui: &mut Ui, name: &str, detail: &str) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(Role::Button, true, name));
    let state = RowState::of_list(false, response.hovered());
    paint_row(ui.painter(), rect, state);
    let (_, colour) = state.colours();
    let inner = Rect::from_min_max(
        pos2(rect.left() + TREE_PADDING_LEFT + SPACE_2, rect.top()),
        pos2(rect.right() - SPACE_2, rect.bottom()),
    );
    let side = line(
        ui,
        plain(detail, ROW_FONT, row_weight(false)),
        inner.width() / 2.0,
    );
    let side_width = side.size().x;
    paint_right(ui, inner, side, palette::GRAY_2_TEXT);
    let name_rect = inner.with_max_x(inner.right() - side_width - CHANNEL_GAP);
    let galley = line(
        ui,
        plain(name, ROW_FONT, row_weight(true)),
        name_rect.width().max(0.0),
    );
    paint_left(ui, name_rect, galley, colour);
    focus_inside(ui.painter(), rect, &response);
    response.clicked()
}
