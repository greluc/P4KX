use std::borrow::Cow;

use egui::scroll_area::ScrollAreaOutput;
use egui::{Align, Color32, Label, Layout, Role, Sense, Ui, Vec2, WidgetInfo, pos2, vec2};
use egui_extras::{Column, TableBuilder, TableRow};

use super::audio_controls::clock;
use super::table::head_fill;
use crate::theme::desktop::{
    CELL_PADDING_X, ROW_FONT, ROW_HEIGHT, TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING,
};
use crate::theme::palette;
use crate::theme::sound_metrics::{SOUND_CODEC_COLUMN, SOUND_DURATION_COLUMN, SOUND_ID_COLUMN};
use crate::theme::tokens::Weight;
use crate::widgets::rows::{RowState, focus_inside, row_weight};
use crate::widgets::scroll_region::{self, Stand};
use crate::widgets::table_row::RowSlots;
use crate::widgets::text::{caps, line, plain};
use crate::{AudioText, BankSound, ScrollText, Text, Texts, format};

const COLUMN_MIN: f32 = 80.0;

/// The texts of the table that do not change from row to row.
struct Fixed {
    unknown: String,
    prefetched: String,
    plugin: String,
}

/// What the user did in the table in this frame.
#[derive(Default)]
struct Outcome {
    clicked: Option<u32>,
    hovered: Option<usize>,
}

/// Shows the WEMs of a soundbank as a virtual table of ID, codec, duration and size, `selected`
/// highlighted; returns the ID of the WEM that was clicked.
pub(super) fn show(
    ui: &mut Ui,
    texts: &Texts,
    sounds: &[BankSound],
    selected: Option<u32>,
) -> Option<u32> {
    let region = scroll_region::begin(
        ui,
        (ui.scope_id().with("soundbank-region"), || {
            texts.get(ScrollText::Soundbank)
        }),
        ROW_HEIGHT,
    );
    let hovered_id = ui.scope_id().with("soundbank-hovered-row");
    let hovered = ui.data(|data| data.get_temp::<usize>(hovered_id));
    let reveal = unrevealed_row(ui, sounds, selected);
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    let left = ui.max_rect().left();
    let mut table = builder(ui);
    if let Some(offset) = region.requested() {
        table = table.vertical_scroll_offset(offset.y);
    }
    if let Some(row) = reveal {
        table = table.scroll_to_row(row, None);
    }
    let rows = Rows {
        texts,
        fixed: Fixed {
            unknown: texts.get(AudioText::Unknown),
            prefetched: texts.get(AudioText::Prefetched),
            plugin: texts.get(AudioText::Plugin),
        },
        sounds,
        selected,
        hovered,
        left,
    };
    let mut outcome = Outcome::default();
    let output = grid(table, &rows, &mut outcome);
    let stand = Stand {
        offset: output.state.clamped_offset(),
        view: output.inner_rect.size(),
        content: output.content_size,
    };
    region.finish(ui, output.inner_rect, stand);
    ui.data_mut(|data| match outcome.hovered {
        Some(index) => {
            data.insert_temp(hovered_id, index);
        }
        None => {
            data.remove_temp::<usize>(hovered_id);
        }
    });
    outcome.clicked
}

/// What the rows of the table are made of.
struct Rows<'a> {
    texts: &'a Texts,
    fixed: Fixed,
    sounds: &'a [BankSound],
    selected: Option<u32>,
    hovered: Option<usize>,
    left: f32,
}

fn grid(table: TableBuilder<'_>, rows: &Rows<'_>, outcome: &mut Outcome) -> ScrollAreaOutput<()> {
    let texts = rows.texts;
    let heads = [
        texts.get(AudioText::ColumnId),
        texts.get(AudioText::Codec),
        texts.get(AudioText::Duration),
        texts.get(Text::ColumnSize),
    ];
    table
        .header(ROW_HEIGHT, |mut header| {
            for (index, label) in heads.iter().enumerate() {
                header.col(|ui| {
                    let from = if index == 0 {
                        rows.left
                    } else {
                        ui.max_rect().left()
                    };
                    head_cell(ui, label, from);
                });
            }
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, rows.sounds.len(), |row| {
                let index = row.index();
                let Some(sound) = rows.sounds.get(index) else {
                    return;
                };
                let active = rows.selected == Some(sound.id());
                let state = RowState::of(active, false, rows.hovered == Some(index));
                let (picked, is_hovered) = show_row(row, (texts, &rows.fixed), sound, state);
                if picked {
                    outcome.clicked = Some(sound.id());
                }
                if is_hovered {
                    outcome.hovered = Some(index);
                }
            });
        })
}
/// Returns the row of `selected` when it was not selected in the previous frame, so the table
/// scrolls it into view once.
fn unrevealed_row(ui: &Ui, sounds: &[BankSound], selected: Option<u32>) -> Option<usize> {
    let id = ui.scope_id().with("soundbank-revealed");
    let revealed = ui.data(|data| data.get_temp::<Option<u32>>(id));
    if revealed == Some(selected) {
        return None;
    }
    ui.data_mut(|data| data.insert_temp(id, selected));
    let wanted = selected?;
    sounds.binary_search_by_key(&wanted, BankSound::id).ok()
}

fn builder(ui: &mut Ui) -> TableBuilder<'_> {
    TableBuilder::new(ui)
        .id_salt("soundbank-sounds")
        .striped(false)
        .resizable(true)
        .auto_shrink(false)
        .min_scrolled_height(0.0)
        .sense(Sense::click())
        .cell_layout(Layout::left_to_right(Align::Center))
        .column(Column::exact(SOUND_ID_COLUMN))
        .column(
            Column::initial(SOUND_CODEC_COLUMN)
                .at_least(COLUMN_MIN)
                .clip(true),
        )
        .column(
            Column::initial(SOUND_DURATION_COLUMN)
                .at_least(COLUMN_MIN)
                .clip(true),
        )
        .column(Column::remainder().at_least(COLUMN_MIN).clip(true))
}

fn head_cell(ui: &Ui, label: &str, left: f32) {
    head_fill(ui, left);
    let rect = ui.max_rect();
    let response = ui.interact(
        rect,
        ui.scope_id().with(("soundbank-head", label)),
        Sense::hover(),
    );
    response.widget_info(|| WidgetInfo::labeled(Role::ColumnHeader, true, label));
    let inner = rect.shrink2(vec2(CELL_PADDING_X, 0.0));
    let text = caps(label, TABLE_HEAD_SIZE, Weight::Bold, TABLE_HEAD_TRACKING);
    let galley = line(ui, text, inner.width());
    ui.painter()
        .with_clip_rect(inner.intersect(ui.clip_rect()))
        .galley(
            pos2(inner.left(), inner.center().y - galley.size().y / 2.0),
            galley,
            palette::GRAY_1,
        );
}

/// A text of a row and whether it stands for a missing value, shown muted.
struct Shown<'a> {
    text: Cow<'a, str>,
    muted: bool,
}

impl<'a> Shown<'a> {
    const fn value(text: String) -> Self {
        Self {
            text: Cow::Owned(text),
            muted: false,
        }
    }

    const fn missing(text: &'a str) -> Self {
        Self {
            text: Cow::Borrowed(text),
            muted: true,
        }
    }
}

/// The shown texts of one row.
struct Cells<'a> {
    id: String,
    codec: Shown<'a>,
    duration: Shown<'a>,
}

impl<'a> Cells<'a> {
    fn of(sound: &BankSound, fixed: &'a Fixed) -> Self {
        let info = sound.info();
        let codec = if sound.is_prefetch() {
            Shown::missing(&fixed.prefetched)
        } else if sound.is_plugin() {
            Shown::missing(&fixed.plugin)
        } else {
            info.map_or_else(
                || Shown::missing(&fixed.unknown),
                |info| Shown::value(info.codec.to_string()),
            )
        };
        let duration = info.and_then(|info| info.duration).map_or_else(
            || Shown::missing(&fixed.unknown),
            |duration| Shown::value(clock(duration, true)),
        );
        Self {
            id: sound.id().to_string(),
            codec,
            duration,
        }
    }
}

fn show_row(
    mut row: TableRow<'_, '_>,
    (texts, fixed): (&Texts, &Fixed),
    sound: &BankSound,
    state: RowState,
) -> (bool, bool) {
    row.set_hovered(false);
    let (_, colour) = state.colours();
    let active = state == RowState::Active;
    let weight = row_weight(active);
    let index = row.index();
    let cells = Cells::of(sound, fixed);
    let tint = |shown: &Shown<'_>| {
        if shown.muted {
            palette::GRAY_2_TEXT
        } else {
            colour
        }
    };
    let mut slots = None;
    row.col(|ui| {
        slots = Some(RowSlots::reserve(ui));
        cell_text(ui, &cells.id, colour, weight);
    });
    row.col(|ui| cell_text(ui, &cells.codec.text, tint(&cells.codec), weight));
    row.col(|ui| cell_text(ui, &cells.duration.text, tint(&cells.duration), weight));
    row.col(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(CELL_PADDING_X);
            let size = format::size(sound.size(), texts.language());
            ui.add(Label::new(plain(size, ROW_FONT, weight).color(colour)).selectable(false));
        });
    });
    let response = row.response();
    if let Some(slots) = slots {
        slots.paint(&response, state, index % 2 == 1);
    }
    response.widget_info(|| {
        let args = [
            ("id", cells.id.clone().into()),
            ("codec", cells.codec.text.to_string().into()),
            ("duration", cells.duration.text.to_string().into()),
        ];
        let name = texts.format(AudioText::WemRow, &args);
        WidgetInfo::selected(Role::Row, true, active, name)
    });
    focus_inside(
        &response.ctx.layer_painter(response.layer_id),
        response.rect,
        &response,
    );
    (response.clicked(), response.hovered())
}
fn cell_text(ui: &mut Ui, text: &str, colour: Color32, weight: Weight) {
    ui.add_space(CELL_PADDING_X);
    ui.add(
        Label::new(plain(text, ROW_FONT, weight).color(colour))
            .truncate()
            .selectable(false),
    );
}
