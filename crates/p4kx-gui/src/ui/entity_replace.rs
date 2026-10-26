// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Label, Layout, Rect, Role, ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2};
use p4kx_assembly::{ItemFacts, PortConstraints};
use p4kx_extraction::Guid;

use super::search::{eyebrow, field};
use crate::theme::assembly_metrics::{REPLACE_HEAD_GAP, REPLACE_LIST_HEIGHT};
use crate::theme::desktop::{CHANNEL_GAP, ROW_FONT, ROW_HEIGHT, TREE_PADDING_LEFT};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_2, Weight};
use crate::widgets::frames::{self, Tone};
use crate::widgets::rows::{RowState, focus_inside, paint_row, row_weight};
use crate::widgets::text::{line, paint_left, paint_right, plain};
use crate::{EntityText, Model, ReplaceChooser, Text, Texts, format};

enum Action {
    Choose(Guid),
    Close,
}

/// Shows the choice of the Entity that replaces what a port holds while it is open: the Entities
/// that fit the port, to search and pick from.
pub(super) fn show(ui: &Ui, model: &mut Model) {
    let Some(chooser) = model.entities().replace() else {
        return;
    };
    let texts = model.texts();
    let title = texts.format(
        EntityText::ReplaceTitle,
        &[("port", chooser.port().to_owned().into())],
    );
    let close_dialog = texts.get(Text::CloseDialog);
    let mut query = chooser.query().to_owned();
    let mut changed = false;
    let shown =
        frames::modal(frames::modal_id("entity-replace"), Tone::Normal).show(ui.ctx(), |ui| {
            ui.set_width(frames::MODAL_WIDE_BODY_WIDTH);
            let closed = frames::head(ui, (&title, Tone::Normal), true, Some(&close_dialog));
            takes(ui, texts, chooser);
            ui.add_space(REPLACE_HEAD_GAP);
            let hint = texts.get(EntityText::ReplaceHint);
            let label = ui.label(eyebrow(&texts.get(EntityText::Label)));
            changed = field(ui, &mut query, &hint, label.id);
            ui.add_space(SPACE_2);
            counts(ui, texts, chooser);
            let picked = list(ui, texts, chooser);
            frames::foot_rule(ui);
            let cancel = frames::foot_action(
                ui,
                &texts.get(crate::ExtractionText::Close),
                crate::widgets::button::Kind::Ghost,
            );
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
        Some(Action::Choose(guid)) => model.entities_mut().choose_replacement(guid),
        Some(Action::Close) => model.entities_mut().close_replace(),
        None if close => model.entities_mut().close_replace(),
        None => {
            if changed && let Some(chooser) = model.entities_mut().replace_mut() {
                chooser.set_query(&query);
            }
        }
    }
}

fn takes(ui: &mut Ui, texts: &Texts, chooser: &ReplaceChooser) {
    let text = chooser.constraints().map_or_else(
        || texts.get(EntityText::ReplaceTakesAny),
        |constraints| {
            let types = types_text(constraints);
            let sizes = sizes_text(constraints);
            let single = if constraints.min_size() == constraints.max_size() {
                "yes"
            } else {
                "no"
            };
            match (types.is_empty(), sizes.is_empty()) {
                (false, false) => texts.format(
                    EntityText::ReplaceTakes,
                    &[
                        ("types", types.into()),
                        ("sizes", sizes.into()),
                        ("single", single.into()),
                    ],
                ),
                (false, true) => {
                    texts.format(EntityText::ReplaceTakesTypes, &[("types", types.into())])
                }
                (true, false) => texts.format(
                    EntityText::ReplaceTakesSizes,
                    &[("sizes", sizes.into()), ("single", single.into())],
                ),
                (true, true) => texts.get(EntityText::ReplaceTakesAny),
            }
        },
    );
    ui.add(Label::new(plain(text, FS_SM, Weight::Regular).color(palette::GRAY_1)).wrap());
}

fn types_text(constraints: &PortConstraints) -> String {
    constraints
        .types()
        .iter()
        .map(|accepted| {
            if accepted.sub_types().is_empty() {
                accepted.name().to_owned()
            } else {
                format!("{} ({})", accepted.name(), accepted.sub_types().join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn sizes_text(constraints: &PortConstraints) -> String {
    match (constraints.min_size(), constraints.max_size()) {
        (Some(min), Some(max)) if min == max => min.to_string(),
        (Some(min), Some(max)) => format!("{min}\u{2013}{max}"),
        (Some(0) | None, None) => String::new(),
        (Some(min), None) => format!("\u{2265} {min}"),
        (None, Some(max)) => format!("\u{2264} {max}"),
    }
}

fn counts(ui: &mut Ui, texts: &Texts, chooser: &ReplaceChooser) {
    let number =
        |value: usize| format::number(u64::try_from(value).unwrap_or(u64::MAX), texts.language());
    let text = texts.format(
        EntityText::ReplaceCount,
        &[
            ("number", number(chooser.shown_len()).into()),
            ("total", number(chooser.fitting()).into()),
        ],
    );
    ui.add(Label::new(
        plain(text, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT),
    ));
}

fn list(ui: &mut Ui, texts: &Texts, chooser: &ReplaceChooser) -> Option<Guid> {
    if chooser.shown_len() == 0 {
        let text = plain(texts.get(EntityText::ReplaceNone), FS_SM, Weight::Regular)
            .color(palette::GRAY_2_TEXT);
        ui.add(Label::new(text));
        return None;
    }
    let mut picked = None;
    ui.with_layout(Layout::top_down(Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ScrollArea::vertical()
            .id_salt("entity-replace-rows")
            .max_height(REPLACE_LIST_HEIGHT.min(frames::modal_room(ui)))
            .show_rows(ui, ROW_HEIGHT, chooser.shown_len(), |ui, visible| {
                for position in visible {
                    if let Some(entity) = chooser.get(position)
                        && row(
                            ui,
                            texts,
                            entity.class_name(),
                            (entity.facts(), chooser.fits(position)),
                        )
                    {
                        picked = Some(entity.guid());
                    }
                }
            });
    });
    picked
}

fn row(ui: &mut Ui, texts: &Texts, name: &str, (facts, fits): (&ItemFacts, bool)) -> bool {
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
    let (detail_text, detail_colour, colour) = if fits {
        (facts_text(texts, facts), palette::GRAY_2_TEXT, colour)
    } else {
        (
            format!(
                "{}, {}",
                texts.get(EntityText::ReplaceMisfit),
                facts_text(texts, facts)
            ),
            palette::WARNING,
            palette::GRAY_2_TEXT,
        )
    };
    let detail = line(
        ui,
        plain(detail_text, ROW_FONT, row_weight(false)),
        inner.width() / 2.0,
    );
    let detail_width = detail.size().x;
    paint_right(ui, inner, detail, detail_colour);
    let name_rect = inner.with_max_x(inner.right() - detail_width - CHANNEL_GAP);
    let galley = line(
        ui,
        plain(name, ROW_FONT, row_weight(true)),
        name_rect.width().max(0.0),
    );
    paint_left(ui, name_rect, galley, colour);
    focus_inside(ui.painter(), rect, &response);
    response.clicked()
}

fn facts_text(texts: &Texts, facts: &ItemFacts) -> String {
    match (facts.item_type().is_empty(), facts.size()) {
        (true, _) => texts.get(EntityText::FactsNone),
        (false, Some(size)) => texts.format(
            EntityText::FactsSized,
            &[
                ("type", facts.item_type().to_owned().into()),
                ("size", size.to_string().into()),
            ],
        ),
        (false, None) => texts.format(
            EntityText::Facts,
            &[("type", facts.item_type().to_owned().into())],
        ),
    }
}
