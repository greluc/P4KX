use std::sync::Arc;

use egui::{
    Color32, Galley, Id, Role, Sense, TextStyle, TextWrapMode, Ui, WidgetInfo, WidgetText, pos2,
    vec2,
};
use fluent_bundle::FluentValue;
use p4kx_comparison::{RecordChangeKind, RecordSide};

use super::field_rows;
use super::record_rows::{kind_look, shown_path};
use crate::theme::desktop::{PREVIEW_PADDING_X, PREVIEW_PATH_HEIGHT, SPINNER_INLINE};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, SPACE_2, Weight};
use crate::widgets::text::{hairline_below, line, plain};
use crate::widgets::{chip, frames, indicator};
use crate::{
    Model, RecordChangeRow, RecordChangeText, RecordDetail, RecordDetailStatus, RecordFields,
    Texts, format,
};

/// The laid-out texts of the panel, kept until the detail, its status, the language or the
/// width change.
#[derive(Clone, Default)]
struct PaneTexts {
    key: Option<Id>,
    path: Option<Arc<Galley>>,
    chip: Option<(Arc<Galley>, chip::Tone)>,
    count: Option<Arc<Galley>>,
    messages: Vec<Arc<Galley>>,
}

/// Shows the field changes of the selected Record: its record path with the change, then every
/// field change with old and new value, or why there are none. Starts comparing the fields when
/// the selection changed.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    ui.set_min_size(ui.available_size());
    let ctx = ui.ctx().clone();
    if let Some(view) = model.comparison_mut().view_mut() {
        view.load_record_detail(move || ctx.request_repaint());
    }
    let texts = model.texts();
    let Some(records) = model.comparison().view().and_then(|view| view.records()) else {
        return;
    };
    let (Some(detail), Some(row)) = (
        records.detail(),
        records.selected().and_then(|id| records.row(id)),
    ) else {
        return;
    };
    let width = ui.available_width();
    let cache = PaneTexts::load(ui, texts, (detail, row), width);
    path_line(ui, &cache);
    match detail.status() {
        RecordDetailStatus::Ready(fields) if cache.messages.is_empty() => {
            field_rows::show(ui, texts, fields, detail.generation());
        }
        status => {
            frames::preview_body().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if matches!(status, RecordDetailStatus::Loading) {
                        indicator::spinner(ui, SPINNER_INLINE);
                    }
                    ui.vertical(|ui| {
                        for message in &cache.messages {
                            ui.label(Arc::clone(message));
                        }
                    });
                });
            });
        }
    }
    cache.store(ui);
}

fn path_line(ui: &mut Ui, cache: &PaneTexts) {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), PREVIEW_PATH_HEIGHT),
        Sense::hover(),
    );
    hairline_below(ui, rect);
    let inner = rect.shrink2(vec2(PREVIEW_PADDING_X, 0.0));
    let count_width = cache
        .count
        .as_ref()
        .map_or(0.0, |count| count.size().x + SPACE_2);
    if let Some(count) = cache.count.clone() {
        let at = pos2(
            inner.right() - count.size().x,
            inner.center().y - count.size().y / 2.0,
        );
        ui.painter().galley(at, count, palette::GRAY_2_TEXT);
    }
    let left = inner.with_max_x((inner.right() - count_width).max(inner.left()));
    let Some(path) = cache.path.clone() else {
        return;
    };
    response.widget_info(|| WidgetInfo::labeled(Role::Label, true, path.text()));
    let at = pos2(left.left(), left.center().y - path.size().y / 2.0);
    let end = (at.x + path.size().x).min(left.right());
    ui.painter()
        .with_clip_rect(left)
        .galley(at, Arc::clone(&path), palette::GRAY_1);
    if let Some((galley, tone)) = cache.chip.clone() {
        let room = left.with_min_x(end + SPACE_2);
        if room.width() > galley.size().x {
            chip::paint(ui, pos2(room.left(), room.center().y), galley, tone);
        }
    }
}

impl PaneTexts {
    fn load(
        ui: &Ui,
        texts: &Texts,
        (detail, row): (&RecordDetail, &RecordChangeRow),
        width: f32,
    ) -> Self {
        let id = Id::unique("comparison-record-texts");
        let mut cache: Self =
            ui.data_mut(|data| std::mem::take(data.get_temp_mut_or_default::<Self>(id)));
        let key = Id::unique((
            texts.language(),
            detail.generation(),
            status_code(detail),
            width.round().to_bits(),
            ui.ctx().pixels_per_point().to_bits(),
        ));
        if cache.key == Some(key) {
            return cache;
        }
        let (kind, tone) = kind_look(row.kind());
        let path = shown_path(texts, row);
        let wrap = 2.0f32.mul_add(-PREVIEW_PADDING_X, width);
        cache =
            Self {
                key: Some(key),
                path: Some(line(
                    ui,
                    plain(path.clone(), FS_XS, Weight::Regular),
                    f32::INFINITY,
                )),
                chip: Some((chip::galley(ui, &texts.get(kind)), tone)),
                count: count(ui, texts, detail),
                messages: messages(texts, detail, (row, &path))
                    .into_iter()
                    .map(|(text, colour)| {
                        WidgetText::from(plain(text, FS_XS, Weight::Regular).color(colour))
                            .into_galley(ui, Some(TextWrapMode::Wrap), wrap, TextStyle::Body)
                    })
                    .collect(),
            };
        cache
    }

    fn store(self, ui: &Ui) {
        let id = Id::unique("comparison-record-texts");
        ui.data_mut(|data| *data.get_temp_mut_or_default::<Self>(id) = self);
    }
}

fn status_code(detail: &RecordDetail) -> u8 {
    match detail.status() {
        RecordDetailStatus::Loading => 0,
        RecordDetailStatus::Ready(_) => 1,
        RecordDetailStatus::Failed(_) => 2,
    }
}

fn count(ui: &Ui, texts: &Texts, detail: &RecordDetail) -> Option<Arc<Galley>> {
    let RecordDetailStatus::Ready(fields) = detail.status() else {
        return None;
    };
    let rows = u64::try_from(fields.rows().len()).unwrap_or(u64::MAX);
    if rows == 0 {
        return None;
    }
    let note = texts.format(
        RecordChangeText::FieldCount,
        &[
            ("count", FluentValue::from(rows)),
            ("number", format::number(rows, texts.language()).into()),
        ],
    );
    Some(line(
        ui,
        plain(note, FS_XS, Weight::Regular).color(palette::GRAY_2_TEXT),
        f32::INFINITY,
    ))
}

/// Returns the messages the panel body shows instead of the field rows; empty when it shows
/// the rows.
fn messages(
    texts: &Texts,
    detail: &RecordDetail,
    (row, path): (&RecordChangeRow, &str),
) -> Vec<(String, Color32)> {
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    match detail.status() {
        RecordDetailStatus::Loading => vec![(
            texts.format(RecordChangeText::Loading, &[path_arg()]),
            palette::GRAY_1,
        )],
        RecordDetailStatus::Failed(Some(reason)) => vec![(
            texts.format(
                RecordChangeText::Failed,
                &[path_arg(), ("reason", reason.to_owned().into())],
            ),
            palette::DANGER_TEXT,
        )],
        RecordDetailStatus::Failed(None) => vec![(
            texts.format(RecordChangeText::Stopped, &[path_arg()]),
            palette::DANGER_TEXT,
        )],
        RecordDetailStatus::Ready(fields) => ready_messages(texts, fields, row.kind()),
    }
}

fn ready_messages(
    texts: &Texts,
    fields: &RecordFields,
    kind: RecordChangeKind,
) -> Vec<(String, Color32)> {
    if !fields.errors().is_empty() {
        return fields
            .errors()
            .iter()
            .map(|error| {
                let text = match error.side {
                    RecordSide::Old => RecordChangeText::UnreadableOld,
                    RecordSide::New => RecordChangeText::UnreadableNew,
                };
                (
                    texts.format(text, &[("reason", error.reason.to_string().into())]),
                    palette::DANGER_TEXT,
                )
            })
            .collect();
    }
    if !fields.rows().is_empty() {
        return Vec::new();
    }
    let text = match kind {
        RecordChangeKind::Added => RecordChangeText::OnlyNew,
        RecordChangeKind::Removed => RecordChangeText::OnlyOld,
        RecordChangeKind::Moved => RecordChangeText::OnlyPath,
        _ => RecordChangeText::NoFieldChanges,
    };
    vec![(texts.get(text), palette::GRAY_1)]
}
