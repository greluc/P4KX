use std::sync::Arc;

use egui::{
    Align, Color32, CornerRadius, Galley, Id, Layout, Rect, Role, Sense, TextStyle, TextWrapMode,
    Ui, UiBuilder, WidgetInfo, WidgetText, pos2, vec2,
};
use fluent_bundle::FluentValue;
use p4kx_comparison::{EntryChange, MAX_DIFF_LEN, MAX_DIFF_LINES};

use super::comparison_summary::kind_look;
use super::diff_rows::{self, Rows};
use crate::theme::desktop::{
    CELL_PADDING_X, PREVIEW_PADDING_X, PREVIEW_PATH_HEIGHT, ROW_HEIGHT, SPINNER_INLINE,
    TABLE_HEAD_SIZE, TABLE_HEAD_TRACKING,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_XS, SPACE_2, Weight};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::text::{caps, hairline_below, line, plain};
use crate::widgets::{chip, frames, indicator};
use crate::{ComparisonView, DiffStatus, DiffText, EntryDiff, Model, Texts, format};

/// A move the user asked for in the diff.
enum Navigation {
    Previous,
    Next,
}

/// The laid-out texts of the pane, kept until the diff, its status, the hunk, the language or the
/// width change.
#[derive(Clone, Default)]
struct PaneTexts {
    key: Option<Id>,
    path: Option<Arc<Galley>>,
    chip: Option<(Arc<Galley>, chip::Tone)>,
    position: Option<Arc<Galley>>,
    previous: String,
    next: String,
    heads: Option<[Arc<Galley>; 2]>,
    head_names: String,
    message: Option<Arc<Galley>>,
}

/// Shows the line diff of the selected change: its path, the hunk navigation, both sides next to
/// each other, or why there is no diff. Starts the diff when the selection changed.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    ui.set_min_size(ui.available_size());
    let ctx = ui.ctx().clone();
    if let Some(view) = model.comparison_mut().view_mut() {
        view.load_diff(move || ctx.request_repaint());
    }
    let scroll_to = model
        .comparison_mut()
        .view_mut()
        .and_then(ComparisonView::diff_mut)
        .and_then(EntryDiff::take_scroll);
    let texts = model.texts();
    let Some(view) = model.comparison().view() else {
        return;
    };
    let (Some(diff), Some(change)) = (view.diff(), view.selected().and_then(|id| view.change(id)))
    else {
        return;
    };
    let width = ui.available_width();
    let cache = PaneTexts::load(ui, texts, (diff, &change), width);
    let navigation = path_line(ui, &cache, diff);
    body(ui, texts, &cache, diff, scroll_to);
    cache.store(ui);
    let Some(diff) = model
        .comparison_mut()
        .view_mut()
        .and_then(ComparisonView::diff_mut)
    else {
        return;
    };
    match navigation {
        Some(Navigation::Previous) => diff.previous_hunk(),
        Some(Navigation::Next) => diff.next_hunk(),
        None => {}
    }
}

fn hunk_count(diff: &EntryDiff) -> usize {
    match diff.status() {
        DiffStatus::Ready(document) => document.hunks().len(),
        _ => 0,
    }
}

fn path_line(ui: &mut Ui, cache: &PaneTexts, diff: &EntryDiff) -> Option<Navigation> {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), PREVIEW_PATH_HEIGHT),
        Sense::hover(),
    );
    hairline_below(ui, rect);
    let inner = rect.shrink2(vec2(PREVIEW_PADDING_X, 0.0));
    let mut navigation = None;
    let current = diff.current_hunk();
    let total = hunk_count(diff);
    let used = ui
        .scope_builder(UiBuilder::new().max_rect(inner), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = SPACE_2;
                let last = current.is_some_and(|hunk| hunk.saturating_add(1) < total);
                if button::button(ui, Kind::Ghost, Size::Compact, &cache.next, last).clicked() {
                    navigation = Some(Navigation::Next);
                }
                let first = current.is_some_and(|hunk| hunk > 0);
                if button::button(ui, Kind::Ghost, Size::Compact, &cache.previous, first).clicked()
                {
                    navigation = Some(Navigation::Previous);
                }
                if let Some(position) = cache.position.clone() {
                    ui.label(position);
                }
            })
            .response
            .rect
        })
        .inner;
    let left = inner.with_max_x((used.left() - SPACE_2).max(inner.left()));
    if let Some(path) = cache.path.clone() {
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
    navigation
}

fn body(ui: &mut Ui, texts: &Texts, cache: &PaneTexts, diff: &EntryDiff, scroll_to: Option<u32>) {
    match diff.status() {
        DiffStatus::Ready(document) if document.has_differences() => {
            heads(ui, cache);
            diff_rows::show(
                ui,
                &Rows {
                    texts,
                    document,
                    generation: diff.generation(),
                    current: diff.current_hunk(),
                    scroll_to,
                },
            );
        }
        status => {
            frames::preview_body().show(ui, |ui| {
                ui.horizontal(|ui| {
                    if matches!(status, DiffStatus::Loading) {
                        indicator::spinner(ui, SPINNER_INLINE);
                    }
                    if let Some(message) = cache.message.clone() {
                        ui.label(message);
                    }
                });
            });
        }
    }
}

fn heads(ui: &mut Ui, cache: &PaneTexts) {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
    let Some(heads) = &cache.heads else {
        return;
    };
    response.widget_info(|| WidgetInfo::labeled(Role::RowHeader, true, &cache.head_names));
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, palette::SURFACE_INPUT);
    let rule = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.max);
    painter.rect_filled(rule, CornerRadius::ZERO, palette::PRIMARY);
    let half = (rect.width() / 2.0).floor();
    for (index, head) in heads.iter().enumerate() {
        let left = if index == 0 {
            rect.left()
        } else {
            rect.left() + half
        };
        let at = pos2(left + CELL_PADDING_X, rect.center().y - head.size().y / 2.0);
        painter.galley(at, Arc::clone(head), palette::GRAY_1);
    }
}

impl PaneTexts {
    fn load(
        ui: &Ui,
        texts: &Texts,
        (diff, change): (&EntryDiff, &EntryChange<'_>),
        width: f32,
    ) -> Self {
        let id = Id::unique("comparison-diff-texts");
        let mut cache: Self =
            ui.data_mut(|data| std::mem::take(data.get_temp_mut_or_default::<Self>(id)));
        let key = Id::unique((
            texts.language(),
            diff.generation(),
            status_code(diff),
            diff.current_hunk(),
            width.round().to_bits(),
            ui.ctx().pixels_per_point().to_bits(),
        ));
        if cache.key == Some(key) {
            return cache;
        }
        let (kind, tone) = kind_look(change.kind());
        let path = change.path().to_owned();
        cache = Self {
            key: Some(key),
            path: Some(line(
                ui,
                plain(path.clone(), FS_XS, Weight::Regular),
                f32::INFINITY,
            )),
            chip: Some((chip::galley(ui, &texts.get(kind)), tone)),
            position: position(ui, texts, diff),
            head_names: format!(
                "{} {}",
                texts.get(DiffText::HeadOld),
                texts.get(DiffText::HeadNew)
            ),
            previous: texts.get(DiffText::Previous),
            next: texts.get(DiffText::Next),
            heads: Some([DiffText::HeadOld, DiffText::HeadNew].map(|text| {
                line(
                    ui,
                    caps(
                        &texts.get(text),
                        TABLE_HEAD_SIZE,
                        Weight::Bold,
                        TABLE_HEAD_TRACKING,
                    ),
                    f32::INFINITY,
                )
            })),
            message: message(texts, diff, &path).map(|(text, colour)| {
                WidgetText::from(plain(text, FS_XS, Weight::Regular).color(colour)).into_galley(
                    ui,
                    Some(TextWrapMode::Wrap),
                    2.0f32.mul_add(-PREVIEW_PADDING_X, width),
                    TextStyle::Body,
                )
            }),
        };
        cache
    }

    fn store(self, ui: &Ui) {
        let id = Id::unique("comparison-diff-texts");
        ui.data_mut(|data| *data.get_temp_mut_or_default::<Self>(id) = self);
    }
}

fn position(ui: &Ui, texts: &Texts, diff: &EntryDiff) -> Option<Arc<Galley>> {
    let current = diff.current_hunk()?;
    let note = texts.format(
        DiffText::Position,
        &[
            ("current", current.saturating_add(1).to_string().into()),
            ("total", hunk_count(diff).to_string().into()),
        ],
    );
    Some(line(
        ui,
        plain(note, FS_XS, Weight::Regular).color(palette::GRAY_2_TEXT),
        f32::INFINITY,
    ))
}

fn status_code(diff: &EntryDiff) -> u8 {
    match diff.status() {
        DiffStatus::Loading => 0,
        DiffStatus::Ready(_) => 1,
        DiffStatus::Binary => 2,
        DiffStatus::TooLarge => 3,
        DiffStatus::Failed(_) => 4,
    }
}

fn message(texts: &Texts, diff: &EntryDiff, path: &str) -> Option<(String, Color32)> {
    let path_arg = || ("path", FluentValue::from(path.to_owned()));
    let language = texts.language();
    Some(match diff.status() {
        DiffStatus::Loading => (
            texts.format(DiffText::Loading, &[path_arg()]),
            palette::GRAY_1,
        ),
        DiffStatus::Ready(document) if document.has_differences() => return None,
        DiffStatus::Ready(_) => (texts.get(DiffText::Equal), palette::GRAY_1),
        DiffStatus::Binary => (
            texts.format(DiffText::Binary, &[path_arg()]),
            palette::GRAY_1,
        ),
        DiffStatus::TooLarge => (
            texts.format(
                DiffText::TooLarge,
                &[
                    path_arg(),
                    ("size", format::size(MAX_DIFF_LEN, language).into()),
                    (
                        "lines",
                        format::number(u64::try_from(MAX_DIFF_LINES).unwrap_or(u64::MAX), language)
                            .into(),
                    ),
                ],
            ),
            palette::GRAY_1,
        ),
        DiffStatus::Failed(Some(reason)) => (
            texts.format(
                DiffText::Failed,
                &[path_arg(), ("reason", reason.to_owned().into())],
            ),
            palette::DANGER_TEXT,
        ),
        DiffStatus::Failed(None) => (
            texts.format(DiffText::Stopped, &[path_arg()]),
            palette::DANGER_TEXT,
        ),
    })
}
