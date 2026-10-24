use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Color32, CornerRadius, Galley, Id, Popup, PopupKind, Rect, RectAlign, Response, Role,
    ScrollArea, Sense, Ui, WidgetInfo, pos2, vec2,
};
use fluent_bundle::FluentValue;
use p4kx_extraction::query::Completion;

use crate::theme::desktop::{
    COMBOBOX_NOTICE_PADDING, COMPLETION_MAX_HEIGHT, DROPDOWN_ITEM_PADDING_X, DROPDOWN_MIN_WIDTH,
    ROW_HEIGHT,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, SPACE_3, Weight, font};
use crate::widgets::rows::paint_divider;
use crate::widgets::text::{line, plain};
use crate::{Language, QueryText, Texts, format};

/// The laid-out candidates of one shown completion, made again only when it changes.
struct Laid {
    key: (u64, u32, Language),
    names: Vec<(Arc<Galley>, Option<Arc<Galley>>)>,
    notice: Option<Arc<Galley>>,
    width: f32,
}

/// Shows the `.combobox__listbox` of `completion` under `caret`, `active` filled orange;
/// returns the candidate clicked.
pub(super) fn show(
    ui: &Ui,
    editor: &Response,
    (completion, active): (&Completion, usize),
    (caret, generation, typed): (Rect, u64, usize),
    texts: &Texts,
) -> Option<usize> {
    let (laid, fresh) = laid(ui, completion, (generation, typed), texts);
    let moved = moved_to(ui, active);
    let mut clicked = None;
    let mut active_id = None;
    Popup::from_response(editor)
        .id(super::super::query_editor::popup_id())
        .kind(PopupKind::Popup)
        .open(true)
        .anchor(caret)
        .align(RectAlign::BOTTOM_START)
        .align_alternatives(&[RectAlign::TOP_START])
        .width(laid.width)
        .show(|ui| {
            ui.set_width(laid.width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ScrollArea::vertical()
                .max_height(COMPLETION_MAX_HEIGHT)
                .show(ui, |ui| {
                    let last = laid.names.len().saturating_sub(1);
                    for (index, (name, detail)) in laid.names.iter().enumerate() {
                        let shown = (name, detail.as_ref(), completion.candidates().get(index));
                        let response = option(ui, shown, index == active, index < last);
                        if index == active {
                            active_id = Some(response.id);
                            if fresh || moved {
                                response.scroll_to_me(None);
                            }
                        }
                        if response.clicked() {
                            clicked = Some(index);
                        }
                    }
                    if let Some(notice) = &laid.notice {
                        notice_row(ui, notice);
                    }
                });
        });
    announce(ui.ctx(), editor.id, active_id);
    clicked
}

/// Tells screen readers which option of the list the editor points at, or that the list is
/// closed when `active` is `None`.
pub(super) fn announce(ctx: &egui::Context, editor: Id, active: Option<Id>) {
    ctx.accesskit_node_builder(editor, |node| {
        match active {
            Some(option) => node.set_active_descendant(option.accesskit_id()),
            None => node.clear_active_descendant(),
        }
        node.set_expanded(active.is_some());
    });
}

fn option(
    ui: &mut Ui,
    (name, detail, candidate): (
        &Arc<Galley>,
        Option<&Arc<Galley>>,
        Option<&p4kx_extraction::query::Candidate>,
    ),
    active: bool,
    divided: bool,
) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    let label = candidate.map_or("", |candidate| candidate.name());
    response.widget_info(|| WidgetInfo::selected(Role::ListBoxOption, true, active, label));
    let (fill, text, muted) = if active {
        (palette::PRIMARY, palette::BLACK, palette::BLACK)
    } else if response.hovered() {
        (palette::GRAY_3, palette::PRIMARY, palette::GRAY_2_TEXT)
    } else {
        (Color32::TRANSPARENT, palette::GRAY_1, palette::GRAY_2_TEXT)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, fill);
    let inner = rect.shrink2(vec2(DROPDOWN_ITEM_PADDING_X, 0.0));
    let y = |galley: &Galley| inner.center().y - galley.size().y / 2.0;
    painter.galley(pos2(inner.left(), y(name)), Arc::clone(name), text);
    if let Some(detail) = detail {
        let x = inner.right() - detail.size().x;
        painter.galley(pos2(x, y(detail)), Arc::clone(detail), muted);
    }
    if divided {
        paint_divider(painter, rect);
    }
    response
}

/// Returns whether the active candidate is another than in the frame before.
fn moved_to(ui: &Ui, active: usize) -> bool {
    let id = Id::unique("p4kx-query-completion-active");
    let before = ui.data_mut(|data| data.get_temp::<usize>(id));
    ui.data_mut(|data| data.insert_temp(id, active));
    before != Some(active)
}

fn notice_row(ui: &mut Ui, notice: &Arc<Galley>) {
    let height = COMBOBOX_NOTICE_PADDING.mul_add(2.0, notice.size().y);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let at = pos2(
        rect.left() + COMBOBOX_NOTICE_PADDING,
        rect.top() + COMBOBOX_NOTICE_PADDING,
    );
    ui.painter()
        .galley(at, Arc::clone(notice), palette::GRAY_2_TEXT);
}

/// Returns the laid-out candidates and whether they were laid out again this frame.
fn laid(
    ui: &Ui,
    completion: &Completion,
    (generation, typed): (u64, usize),
    texts: &Texts,
) -> (Arc<Laid>, bool) {
    let key = (
        generation,
        ui.ctx().pixels_per_point().to_bits(),
        texts.language(),
    );
    let id = Id::unique("p4kx-query-completion-laid");
    if let Some(laid) = ui.data(|data| data.get_temp::<Arc<Laid>>(id))
        && laid.key == key
    {
        return (laid, false);
    }
    let names: Vec<(Arc<Galley>, Option<Arc<Galley>>)> = completion
        .candidates()
        .iter()
        .map(|candidate| {
            let detail = candidate
                .detail()
                .map(|detail| line(ui, plain(detail, FS_XS, Weight::Regular), f32::INFINITY));
            (marked(ui, candidate.name(), typed), detail)
        })
        .collect();
    let widest = names
        .iter()
        .map(|(name, detail)| {
            name.size().x
                + detail
                    .as_ref()
                    .map_or(0.0, |detail| SPACE_3 + detail.size().x)
        })
        .fold(0.0_f32, f32::max);
    let notice = (completion.more() > 0).then(|| {
        let more = u64::try_from(completion.more()).unwrap_or(u64::MAX);
        let text = texts.format(
            QueryText::CompletionMore,
            &[
                ("count", FluentValue::from(more)),
                ("number", format::number(more, texts.language()).into()),
            ],
        );
        line(
            ui,
            plain(text, FS_SM, Weight::Regular).italics(),
            f32::INFINITY,
        )
    });
    let laid = Arc::new(Laid {
        key,
        width: DROPDOWN_ITEM_PADDING_X
            .mul_add(2.0, widest)
            .max(DROPDOWN_MIN_WIDTH),
        names,
        notice,
    });
    ui.data_mut(|data| data.insert_temp(id, Arc::clone(&laid)));
    (laid, true)
}

/// Lays `name` out with its first `typed` bytes in bold, the `<mark>` of the design system,
/// in the colour the row paints it with.
fn marked(ui: &Ui, name: &str, typed: usize) -> Arc<Galley> {
    let mut split = typed.min(name.len());
    while !name.is_char_boundary(split) {
        split -= 1;
    }
    let (start, rest) = name.split_at(split);
    let mut job = LayoutJob::default();
    let format = |weight| TextFormat::simple(font(FS_SM, weight), Color32::PLACEHOLDER);
    job.append(start, 0.0, format(Weight::Bold));
    job.append(rest, 0.0, format(Weight::Regular));
    ui.ctx().fonts_mut(|fonts| fonts.layout_job(job))
}
