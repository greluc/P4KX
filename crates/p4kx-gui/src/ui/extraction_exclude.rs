// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Frame, Margin, TextEdit, Ui, vec2};
use p4kx_extraction::{FilterError, MAX_EXCLUSIONS, MAX_FILTER_LEN, MAX_FILTER_WILDCARDS};

use super::extraction::group_label;
use crate::theme::desktop::{
    MODAL_GROUP_GAP, TEXTAREA_LINE_HEIGHT, TEXTAREA_PADDING_X, TEXTAREA_PADDING_Y,
};
use crate::theme::tokens::{FS_BODY, Weight, font};
use crate::widgets::field;
use crate::widgets::frames::px;
use crate::widgets::text::plain;
use crate::{ExtractionSetup, ExtractionText as T, Texts};

/// The rows of the empty field.
const MIN_ROWS: usize = 2;

/// The most characters the field takes: room for several Filters of the longest accepted length.
const CHAR_CAP: usize = 4 * MAX_FILTER_LEN;

/// Returns the height of a text area of `rows` lines, with its padding.
fn height(rows: usize) -> f32 {
    let lines = f32::from(u8::try_from(rows).unwrap_or(u8::MAX));
    (FS_BODY * TEXTAREA_LINE_HEIGHT).mul_add(lines, 2.0 * TEXTAREA_PADDING_Y)
}

/// Shows the heading and the field of the exclusion Filters, one per line, with the problem under
/// it: a Filter that is not accepted, or an exclusion that left no Entry the last time the
/// Extraction was asked to start.
///
/// The field grows with its text inside the scrolling body of the dialog, so the wheel always
/// scrolls the dialog; the text is capped at [`CHAR_CAP`] characters. Returns whether the text is
/// accepted as the exclusion.
pub(super) fn show(ui: &mut Ui, texts: &Texts, setup: &mut ExtractionSetup) -> bool {
    ui.add_space(MODAL_GROUP_GAP);
    group_label(ui, &texts.get(T::ExcludeHeading));
    let problem = setup.exclusion().err();
    let invalid = problem.is_some() || setup.leaves_nothing();
    let mut text = setup.exclude().to_owned();
    let bloom = field::bloom_slot(ui);
    let frame = field::frame_slot(ui);
    let edit = TextEdit::multiline(&mut text)
        .char_limit(CHAR_CAP)
        .hint_text(plain(texts.get(T::ExcludeHint), FS_BODY, Weight::Regular).italics())
        .font(font(FS_BODY, Weight::Regular))
        .frame(Frame::NONE.inner_margin(Margin::symmetric(
            px(TEXTAREA_PADDING_X),
            px(TEXTAREA_PADDING_Y),
        )))
        .min_size(vec2(0.0, height(MIN_ROWS)))
        .desired_width(f32::INFINITY);
    let response = ui.add(edit);
    if invalid {
        field::paint_error_frame(ui, frame, response.rect);
    } else {
        field::paint_frame(ui, frame, response.rect, response.has_focus());
    }
    if response.has_focus() {
        field::paint_bloom(ui, bloom, response.rect);
    }
    if response.changed() {
        setup.set_exclude(text);
    }
    let accepted = problem.is_none();
    if let Some(problem) = problem {
        field::error_note(ui, &problem_text(texts, &problem));
    } else if setup.leaves_nothing() {
        field::error_note(ui, &texts.get(T::ExcludeNothingLeft));
    }
    accepted
}

/// Returns the words for a Filter text that is not accepted.
pub(super) fn problem_text(texts: &Texts, problem: &FilterError) -> String {
    let limit = |max: usize| [("max", u64::try_from(max).unwrap_or(u64::MAX).into())];
    match problem {
        FilterError::TooLong { .. } => texts.format(T::ExcludeTooLong, &limit(MAX_FILTER_LEN)),
        FilterError::TooManyWildcards { .. } => {
            texts.format(T::ExcludeTooManyWildcards, &limit(MAX_FILTER_WILDCARDS))
        }
        FilterError::TooManyExclusions { .. } => {
            texts.format(T::ExcludeTooMany, &limit(MAX_EXCLUSIONS))
        }
        FilterError::EmptyExclusion => texts.get(T::ExcludeEmpty),
    }
}
