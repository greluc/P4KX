// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;

use egui::{Label, Margin, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};
use fluent_bundle::FluentValue;

use super::list_footer;
use super::search::{eyebrow, field};
use super::ui_tab::{danger, loading, message, warning};
use crate::theme::desktop::{ROW_FONT, SCROLLBAR_THIN, TOOLBAR_PADDING, TREE_PADDING_LEFT};
use crate::theme::ui_metrics::UI_ROW_HEIGHT;
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_2, Weight};
use crate::widgets::frames::px;
use crate::widgets::rows::{RowState, focus_inside, paint_row, row_weight};
use crate::widgets::text::{line, paint_left, plain};
use crate::widgets::tooltip;
use crate::{Model, Texts, UiFilesStatus, UiMode, UiScreensStatus, UiText, format};

/// What the user picked in a list.
pub(super) enum Picked {
    File(usize),
    Ship(usize),
    Screen(usize),
}

struct RowData<'a> {
    primary: Cow<'a, str>,
    secondary: Cow<'a, str>,
    tip: Cow<'a, str>,
    selected: bool,
}

/// Shows the search field of the list the mode shows.
fn search(ui: &mut Ui, model: &mut Model) {
    let texts = model.texts();
    let label = eyebrow(&texts.get(UiText::SearchLabel));
    let hint = texts.get(match model.ui().mode() {
        UiMode::Files => UiText::FilesHint,
        UiMode::Ships => UiText::ShipsHint,
    });
    let mut text = model.ui().query().to_owned();
    let changed = egui::Frame::NONE
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let label = ui.label(label);
                field(ui, &mut text, &hint, label.id)
            })
            .inner
        })
        .inner;
    if changed {
        model.ui_mut().set_query(&text);
    }
}

/// Shows the search and the GFx and SWF files; returns the file the user clicked.
pub(super) fn files(ui: &mut Ui, model: &mut Model) -> Option<Picked> {
    search(ui, model);
    let UiFilesStatus::Ready(list) = model.ui().files_status() else {
        return None;
    };
    let texts = model.texts();
    footer(
        ui,
        texts,
        ("ui-files-footer", UiText::FilesCount, UiText::FilesCountShown),
        (list.total(), list.shown_len()),
    );
    if list.shown_len() == 0 {
        message(ui, texts.get(UiText::NoMatch));
        return None;
    }
    let selected = model.ui().selected_file();
    rows(ui, "ui-files-rows", list.shown_len(), |position| {
        let path = list.get(position)?;
        let (folder, name) = path.rsplit_once(['/', '\\']).unwrap_or(("", path));
        Some(RowData {
            primary: Cow::Borrowed(name),
            secondary: Cow::Borrowed(folder),
            tip: Cow::Borrowed(path),
            selected: selected == Some(path),
        })
    })
    .map(Picked::File)
}

/// Shows the search and the ships and vehicles; returns the one the user clicked.
pub(super) fn ships(ui: &mut Ui, model: &mut Model) -> Option<Picked> {
    search(ui, model);
    let list = model.ui().ships()?;
    let texts = model.texts();
    footer(
        ui,
        texts,
        ("ui-ships-footer", UiText::ShipsCount, UiText::ShipsCountShown),
        (list.total(), list.shown_len()),
    );
    if list.shown_len() == 0 {
        message(ui, texts.get(UiText::NoMatch));
        return None;
    }
    let picked = model.ui().ship().map(crate::ShipPick::guid);
    rows(ui, "ui-ships-rows", list.shown_len(), |position| {
        let ship = list.get(position)?;
        let path = ship.record_path();
        let short = path.split_once("/entities/").map_or(path, |(_, rest)| rest);
        Some(RowData {
            primary: Cow::Borrowed(ship.class_name()),
            secondary: Cow::Borrowed(short),
            tip: Cow::Borrowed(path),
            selected: picked == Some(ship.guid()),
        })
    })
    .map(Picked::Ship)
}

/// Shows the screens of the picked ship; returns the one the user clicked.
pub(super) fn screens(ui: &mut Ui, model: &Model) -> Option<Picked> {
    let texts = model.texts();
    let ship = model.ui().ship()?;
    egui::Frame::NONE
        .inner_margin(Margin::same(px(TOOLBAR_PADDING)))
        .show(ui, |ui| {
            ui.label(eyebrow(&texts.get(UiText::ScreensHeading)));
            ui.add(Label::new(plain(ship.class_name(), FS_SM, Weight::Bold)).selectable(true));
        });
    match ship.status() {
        UiScreensStatus::Resolving => loading(ui, texts, UiText::ScreensLoading),
        UiScreensStatus::ArchiveChanged => warning(ui, texts.get(UiText::ScreensArchiveChanged)),
        UiScreensStatus::Failed(Some(reason)) => danger(
            ui,
            texts.format(UiText::ScreensFailed, &[("reason", reason.to_owned().into())]),
        ),
        UiScreensStatus::Failed(None) => danger(ui, texts.get(UiText::ScreensStopped)),
        UiScreensStatus::Ready(listing) if listing.screens.is_empty() => {
            message(ui, texts.get(UiText::ScreensNone));
        }
        UiScreensStatus::Ready(listing) => {
            let picked = ship.picked();
            return rows(ui, "ui-screens-rows", listing.screens.len(), |position| {
                let screen = listing.screens.get(position)?;
                let size = |value: f32| FluentValue::from(format!("{value:.0}"));
                Some(RowData {
                    primary: if screen.port.is_empty() {
                        Cow::Owned(texts.get(UiText::ScreenOwn))
                    } else {
                        Cow::Borrowed(&screen.port)
                    },
                    secondary: Cow::Owned(format!(
                        "{}  {}",
                        screen.entity,
                        texts.format(
                            UiText::ScreenSize,
                            &[("width", size(screen.width)), ("height", size(screen.height))]
                        )
                    )),
                    tip: Cow::Borrowed(&screen.canvas),
                    selected: picked == Some(position),
                })
            })
            .map(Picked::Screen);
        }
    }
    None
}

fn footer(
    ui: &mut Ui,
    texts: &Texts,
    (id, all, some): (&'static str, UiText, UiText),
    (total, shown): (usize, usize),
) {
    let value = |count: usize| FluentValue::from(u64::try_from(count).unwrap_or(u64::MAX));
    let numbered = |count: usize| format::number(u64::try_from(count).unwrap_or(u64::MAX), texts.language());
    let text = if shown == total {
        texts.format(
            all,
            &[("count", value(total)), ("number", numbered(total).into())],
        )
    } else {
        texts.format(
            some,
            &[
                ("count", value(total)),
                ("number", numbered(shown).into()),
                ("total", numbered(total).into()),
            ],
        )
    };
    list_footer::show(ui, id, |ui| {
        ui.add(
            Label::new(plain(text, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT))
                .selectable(false),
        );
    });
}

fn rows<'a>(
    ui: &mut Ui,
    salt: &'static str,
    len: usize,
    data: impl Fn(usize) -> Option<RowData<'a>>,
) -> Option<usize> {
    let mut clicked = None;
    ui.spacing_mut().scroll.bar_width = SCROLLBAR_THIN;
    ui.spacing_mut().item_spacing.y = 0.0;
    ScrollArea::vertical()
        .id_salt(salt)
        .auto_shrink(false)
        .show_rows(ui, UI_ROW_HEIGHT, len, |ui, visible| {
            for position in visible {
                if let Some(row) = data(position)
                    && paint(ui, &row)
                {
                    clicked = Some(position);
                }
            }
        });
    clicked
}

fn paint(ui: &mut Ui, row: &RowData<'_>) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), UI_ROW_HEIGHT), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::Button, true, row.selected, &*row.primary));
    let state = RowState::of_list(row.selected, response.hovered());
    paint_row(ui.painter(), rect, state);
    let (_, colour) = state.colours();
    let inner = Rect::from_min_max(
        pos2(rect.left() + TREE_PADDING_LEFT + SPACE_2, rect.top()),
        rect.max,
    );
    let (upper, lower) = (
        inner.with_max_y(inner.center().y),
        inner.with_min_y(inner.center().y),
    );
    let name = line(
        ui,
        plain(&*row.primary, ROW_FONT, row_weight(row.selected)),
        (inner.width() - SPACE_2).max(0.0),
    );
    paint_left(ui, upper, name, colour);
    let second = line(
        ui,
        plain(&*row.secondary, ROW_FONT, row_weight(false)),
        (inner.width() - SPACE_2).max(0.0),
    );
    paint_left(ui, lower, second, palette::GRAY_2_TEXT);
    focus_inside(ui.painter(), rect, &response);
    tooltip::hover(response.clone(), |ui| {
        tooltip::line(ui, &*row.tip);
    });
    response.clicked()
}
