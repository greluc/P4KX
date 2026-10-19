use std::ops::Range;
use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Frame, Galley, Id, Key, Margin, Modifiers, ScrollArea, Stroke, TextBuffer, TextEdit, Ui, vec2,
};

use crate::theme::desktop::{
    ALERT_DANGER_FILL, ERROR_UNDERLINE, QUERY_EDITOR_MAX_ROWS, QUERY_EDITOR_ROWS,
    TEXTAREA_LINE_HEIGHT, TEXTAREA_PADDING_X, TEXTAREA_PADDING_Y,
};
use crate::theme::palette;
use crate::theme::tokens::{FS_BODY, Weight, font};
use crate::widgets::field;
use crate::widgets::frames::px;
use crate::widgets::text::plain;

/// What the user did in the editor this frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Edited {
    /// The text changed.
    pub(super) changed: bool,
    /// Ctrl+Enter asked to run the Query.
    pub(super) submitted: bool,
}

/// The last laid-out text, kept until the text, the mark, the width or the scale change.
#[derive(Clone, Default)]
struct Laid {
    text: String,
    mark: Option<Range<usize>>,
    width: u32,
    pixels_per_point: u32,
    galley: Option<Arc<Galley>>,
}

/// Shows the Query editor, a `textarea` labelled by `label`, with the bytes `mark` of the text
/// underlined as an error; Ctrl+Enter submits instead of breaking the line.
pub(super) fn show(
    ui: &mut Ui,
    text: &mut String,
    (hint, label): (&str, Id),
    mark: Option<&Range<usize>>,
) -> Edited {
    let id = Id::unique("p4kx-query-editor");
    let submitted = ui.memory(|memory| memory.has_focus(id))
        && ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Enter));
    let bloom = field::bloom_slot(ui);
    let rows = |rows: usize| {
        let line_height = FS_BODY * TEXTAREA_LINE_HEIGHT;
        line_height.mul_add(
            f32::from(u8::try_from(rows).unwrap_or(u8::MAX)),
            2.0 * TEXTAREA_PADDING_Y,
        )
    };
    let height = rows(QUERY_EDITOR_ROWS);
    let mut layouter =
        |ui: &Ui, buffer: &dyn TextBuffer, width: f32| laid_out(ui, buffer.as_str(), mark, width);
    let edit = TextEdit::multiline(text)
        .id(id)
        .hint_text(plain(hint, FS_BODY, Weight::Regular).italics())
        .font(font(FS_BODY, Weight::Regular))
        .frame(Frame::NONE.inner_margin(Margin::symmetric(
            px(TEXTAREA_PADDING_X),
            px(TEXTAREA_PADDING_Y),
        )))
        .min_size(vec2(0.0, height))
        .desired_width(f32::INFINITY)
        .layouter(&mut layouter);
    let response = ScrollArea::vertical()
        .id_salt("p4kx-query-editor-scroll")
        .max_height(rows(QUERY_EDITOR_MAX_ROWS))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let frame = field::frame_slot(ui);
            let response = ui.add(edit).labelled_by(label);
            field::paint_frame(ui, frame, response.rect, response.has_focus());
            response
        })
        .inner;
    if response.has_focus() {
        field::paint_bloom(ui, bloom, response.rect);
    }
    Edited {
        changed: response.changed(),
        submitted,
    }
}

fn laid_out(ui: &Ui, text: &str, mark: Option<&Range<usize>>, width: f32) -> Arc<Galley> {
    let id = Id::unique("p4kx-query-editor-galley");
    let width_key = width.to_bits();
    let scale = ui.ctx().pixels_per_point().to_bits();
    let known = ui.data_mut(|data| {
        let laid = data.get_temp_mut_or_default::<Laid>(id);
        (laid.text == text
            && laid.mark.as_ref() == mark
            && laid.width == width_key
            && laid.pixels_per_point == scale)
            .then(|| laid.galley.clone())
            .flatten()
    });
    if let Some(galley) = known {
        return galley;
    }
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let normal = TextFormat {
        font_id: font(FS_BODY, Weight::Regular),
        color: palette::GRAY_1,
        line_height: Some(FS_BODY * TEXTAREA_LINE_HEIGHT),
        ..TextFormat::default()
    };
    let marked = TextFormat {
        background: ALERT_DANGER_FILL,
        underline: Stroke::new(ERROR_UNDERLINE, palette::DANGER),
        ..normal.clone()
    };
    let span = mark.and_then(|span| shown_span(text, span));
    match span {
        Some(span) => {
            job.append(
                text.get(..span.start).unwrap_or_default(),
                0.0,
                normal.clone(),
            );
            job.append(text.get(span.clone()).unwrap_or_default(), 0.0, marked);
            job.append(text.get(span.end..).unwrap_or_default(), 0.0, normal);
        }
        None => job.append(text, 0.0, normal),
    }
    let galley = ui.ctx().fonts_mut(|fonts| fonts.layout_job(job));
    ui.data_mut(|data| {
        let laid = data.get_temp_mut_or_default::<Laid>(id);
        text.clone_into(&mut laid.text);
        laid.mark = mark.cloned();
        laid.width = width_key;
        laid.pixels_per_point = scale;
        laid.galley = Some(Arc::clone(&galley));
    });
    galley
}

/// Returns the bytes of `text` to mark for `span`: the span itself, or the character at an empty
/// span; `None` when nothing of the text can be marked.
fn shown_span(text: &str, span: &Range<usize>) -> Option<Range<usize>> {
    if span.start > text.len() || !text.is_char_boundary(span.start) {
        return None;
    }
    let end = if span.is_empty() {
        span.start + text.get(span.start..)?.chars().next()?.len_utf8()
    } else {
        span.end.min(text.len())
    };
    text.is_char_boundary(end).then_some(span.start..end)
}

#[cfg(test)]
mod tests {
    use super::shown_span;

    #[test]
    fn a_span_is_marked_as_it_is() {
        assert_eq!(
            shown_span("from Item select mas", &(17..20)),
            Some(17..20),
            "span"
        );
    }

    #[test]
    fn an_empty_span_marks_the_character_at_it() {
        assert_eq!(shown_span("from Ä x", &(5..5)), Some(5..7), "two bytes");
    }

    #[test]
    fn an_empty_span_at_the_end_marks_nothing() {
        assert_eq!(shown_span("from", &(4..4)), None, "end");
    }

    #[test]
    fn a_span_of_another_text_marks_nothing() {
        assert_eq!(shown_span("from Ä", &(6..6)), None, "inside a character");
        assert_eq!(shown_span("from", &(9..12)), None, "beyond the end");
    }
}
