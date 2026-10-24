// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Align, Align2, Frame, Id, Key, Label, Layout, Margin, TextEdit, Ui, vec2};

use super::search::eyebrow;
use crate::theme::desktop::MODAL_GROUP_GAP;
use crate::theme::palette;
use crate::theme::tokens::{FS_BODY, FS_SM, SPACE_3, TOUCH_TARGET, Weight, font};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, Tone, px};
use crate::widgets::text::plain;
use crate::widgets::{field, toggle};
use crate::{ArchiveKeyText, KeyProblem, KeyReason, Model, Text, Texts};

/// The most characters the key field takes: a key of 32 digits with room for what a paste brings
/// along.
const MAX_CHARACTERS: usize = 64;

/// What the user did in the dialog.
#[derive(Debug, Default, Clone, Copy)]
struct Done {
    submit: bool,
    cancel: bool,
    remove: bool,
}

/// Shows the Archive key dialog while it is open: the key field, a note about the key in use and
/// the stored key, and the actions.
pub(super) fn show(ui: &Ui, model: &mut Model) {
    let shown_id = Id::unique("p4kx-archive-key-dialog-shown");
    let Some(dialog) = model.key_dialog().cloned() else {
        ui.data_mut(|data| data.remove::<bool>(shown_id));
        return;
    };
    let first = !ui.data_mut(|data| data.get_temp::<bool>(shown_id).unwrap_or(false));
    ui.data_mut(|data| data.insert_temp(shown_id, true));
    let texts = model.texts();
    let title = texts.get(ArchiveKeyText::Title);
    let close_dialog = texts.get(Text::CloseDialog);
    let stored = model.stored_key_masked().map(str::to_owned);
    let mut text = dialog.text().to_owned();
    let mut revealed = dialog.revealed();
    let ready = !text.trim().is_empty() && !dialog.pending();
    let notices = Notices {
        variable_invalid: model.key_variable_invalid(),
        store_problem: model.key_store_problem(),
    };
    let shown =
        frames::modal(frames::modal_id("archive-key-dialog"), Tone::Normal).show(ui.ctx(), |ui| {
            ui.set_width(frames::MODAL_BODY_WIDTH);
            let closed = frames::head(ui, (&title, Tone::Normal), true, Some(&close_dialog));
            let (entered, removed) = body(
                ui,
                texts,
                &dialog,
                (&mut text, &mut revealed),
                (stored.as_deref(), first),
                notices,
            );
            let mut done = foot(ui, texts, ready);
            done.submit |= entered && ready;
            done.cancel |= closed;
            done.remove |= removed;
            done
        });
    frames::modal_decoration(
        ui.ctx(),
        shown.response.layer_id,
        shown.response.rect,
        Tone::Normal,
    );
    let done = shown.inner;
    if let Some(open) = model.key_dialog_mut() {
        open.set_text(text);
        open.set_revealed(revealed);
    }
    if done.remove {
        model.remove_stored_key();
    } else if done.submit {
        let ctx = ui.ctx().clone();
        model.submit_key_dialog(move || ctx.request_repaint());
    } else if done.cancel || shown.should_close() {
        model.close_key_dialog();
    }
}

/// Shows why the dialog is open, the key field, the note about the key typed and the stored key;
/// returns whether Enter was pressed in the field and whether removing the stored key was asked.
fn body(
    ui: &mut Ui,
    texts: &Texts,
    dialog: &crate::KeyDialog,
    (text, revealed): (&mut String, &mut bool),
    (stored, first): (Option<&str>, bool),
    notices: Notices<'_>,
) -> (bool, bool) {
    let reason = match dialog.reason() {
        KeyReason::Menu => ArchiveKeyText::Intro,
        KeyReason::Needed => ArchiveKeyText::Needed,
        KeyReason::Rejected => ArchiveKeyText::Rejected,
    };
    ui.add(Label::new(plain(texts.get(reason), FS_SM, Weight::Regular)));
    ui.add_space(MODAL_GROUP_GAP - SPACE_3);
    let invalid = matches!(
        dialog.problem(),
        Some(KeyProblem::Length(_) | KeyProblem::Character(_) | KeyProblem::Wrong)
    );
    let entered = key_field(ui, texts, text, (!*revealed, invalid), first) && !dialog.pending();
    if let Some(problem) = dialog.problem() {
        field::error_note(ui, &problem_text(texts, problem));
    }
    ui.add_space(SPACE_3);
    toggle::checkbox(ui, revealed, &texts.get(ArchiveKeyText::Show));
    if dialog.pending() {
        ui.add_space(SPACE_3);
        ui.add(Label::new(
            plain(texts.get(ArchiveKeyText::Checking), FS_SM, Weight::Regular)
                .color(palette::GRAY_2_TEXT),
        ));
    }
    notes(ui, texts, notices);
    ui.add_space(MODAL_GROUP_GAP - SPACE_3);
    let removed = stored_row(ui, texts, stored);
    (entered, removed)
}

/// What the window found wrong with the key it started with.
#[derive(Debug, Default, Clone, Copy)]
struct Notices<'a> {
    variable_invalid: bool,
    store_problem: Option<&'a str>,
}

/// Says in `color-warning` which source of the key was ignored, so that a malformed variable or
/// stored key does not go unnoticed.
fn notes(ui: &mut Ui, texts: &Texts, notices: Notices<'_>) {
    if notices.variable_invalid {
        ui.add_space(SPACE_3);
        ui.add(Label::new(
            plain(
                texts.get(ArchiveKeyText::NoteVariable),
                FS_SM,
                Weight::Regular,
            )
            .color(palette::WARNING),
        ));
    }
    if let Some(reason) = notices.store_problem {
        ui.add_space(SPACE_3);
        let note = texts.format(
            ArchiveKeyText::NoteStore,
            &[("reason", reason.to_owned().into())],
        );
        ui.add(Label::new(
            plain(note, FS_SM, Weight::Regular).color(palette::WARNING),
        ));
    }
}

/// Shows the labelled key field, focused when the dialog opens; returns whether Enter was pressed
/// in it. The border is red while the field holds an error.
fn key_field(
    ui: &mut Ui,
    texts: &Texts,
    text: &mut String,
    (hidden, invalid): (bool, bool),
    first: bool,
) -> bool {
    let label = ui.label(eyebrow(&texts.get(ArchiveKeyText::Label)));
    let bloom = field::bloom_slot(ui);
    let frame = field::frame_slot(ui);
    let edit = TextEdit::singleline(text)
        .password(hidden)
        .char_limit(MAX_CHARACTERS)
        .hint_text(plain(texts.get(ArchiveKeyText::Hint), FS_BODY, Weight::Regular).italics())
        .font(font(FS_BODY, Weight::Regular))
        .frame(Frame::NONE.inner_margin(Margin::symmetric(px(SPACE_3), 0)))
        .min_size(vec2(0.0, TOUCH_TARGET))
        .desired_width(ui.available_width())
        .align(Align2::LEFT_CENTER);
    let response = ui.add(edit).labelled_by(label.id);
    if invalid {
        field::paint_error_frame(ui, frame, response.rect);
    } else {
        field::paint_frame(ui, frame, response.rect, response.has_focus());
    }
    if response.has_focus() {
        field::paint_bloom(ui, bloom, response.rect);
    }
    if first {
        response.request_focus();
    }
    response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter))
}

/// Shows whether a key is stored, with the action that removes it; returns whether that was
/// asked.
fn stored_row(ui: &mut Ui, texts: &Texts, stored: Option<&str>) -> bool {
    let Some(masked) = stored else {
        ui.add(Label::new(
            plain(texts.get(ArchiveKeyText::NotStored), FS_SM, Weight::Regular)
                .color(palette::GRAY_2_TEXT),
        ));
        return false;
    };
    let note = texts.format(
        ArchiveKeyText::Stored,
        &[("masked", masked.to_owned().into())],
    );
    ui.add(Label::new(
        plain(note, FS_SM, Weight::Regular).color(palette::GRAY_2_TEXT),
    ));
    ui.add_space(SPACE_3);
    let remove = texts.get(ArchiveKeyText::Remove);
    button::button(ui, Kind::QuietDanger, Size::Compact, &remove, true).clicked()
}

fn problem_text(texts: &Texts, problem: &KeyProblem) -> String {
    let number = |value: usize| u64::try_from(value).unwrap_or(u64::MAX);
    match problem {
        KeyProblem::Length(digits) => texts.format(
            ArchiveKeyText::ErrorLength,
            &[("digits", number(*digits).into())],
        ),
        KeyProblem::Character(position) => texts.format(
            ArchiveKeyText::ErrorCharacter,
            &[("position", number(*position).into())],
        ),
        KeyProblem::Wrong => texts.get(ArchiveKeyText::ErrorWrong),
        KeyProblem::Check(reason) => texts.format(
            ArchiveKeyText::ErrorCheck,
            &[("reason", reason.clone().into())],
        ),
        KeyProblem::Store(reason) => texts.format(
            ArchiveKeyText::ErrorStore,
            &[("reason", reason.clone().into())],
        ),
        KeyProblem::Remove(reason) => texts.format(
            ArchiveKeyText::ErrorRemove,
            &[("reason", reason.clone().into())],
        ),
    }
}

/// Shows the foot: the ghost Cancel and, right of it, Save as the CTA.
fn foot(ui: &mut Ui, texts: &Texts, ready: bool) -> Done {
    ui.add_space(MODAL_GROUP_GAP - SPACE_3);
    frames::foot_rule(ui);
    let mut done = Done::default();
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let save = texts.get(ArchiveKeyText::Save);
        done.submit = button::button(ui, Kind::Cta, Size::Normal, &save, ready).clicked();
        let cancel = texts.get(ArchiveKeyText::Cancel);
        done.cancel = button::button(ui, Kind::Ghost, Size::Normal, &cancel, true).clicked();
    });
    done
}
