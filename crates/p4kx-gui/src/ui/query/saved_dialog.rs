use egui::{Align, Align2, Id, Key, Label, Layout, Margin, TextEdit, Ui, vec2};
use p4kx_settings::{NameProblem, SavedQueries};

use super::super::Requests;
use super::super::search::eyebrow;
use crate::theme::desktop::MODAL_GROUP_GAP;
use crate::theme::palette;
use crate::theme::tokens::{FS_BODY, FS_SM, SPACE_3, TOUCH_TARGET, Weight, font};
use crate::widgets::button::{self, Kind, Size};
use crate::widgets::frames::{self, Tone, px};
use crate::widgets::text::plain;
use crate::{Model, NameNote, QueryText, SavedDialog, Texts, format};

/// Shows the open dialog of the saved Queries: the name to save or rename to, or the question
/// before a delete.
pub(in crate::ui) fn show(ui: &Ui, model: &mut Model, requests: &mut Requests) {
    let shown_id = Id::unique("p4kx-saved-dialog-shown");
    let Some(dialog) = model.query().saved_dialog().cloned() else {
        ui.data_mut(|data| data.remove::<bool>(shown_id));
        return;
    };
    let first = !ui.data_mut(|data| data.get_temp::<bool>(shown_id).unwrap_or(false));
    ui.data_mut(|data| data.insert_temp(shown_id, true));
    let note = model.query().name_note();
    let texts = model.texts();
    let (id, title, action, tone) = parts(&dialog);
    let title = texts.get(title);
    let close_dialog = texts.get(crate::Text::CloseDialog);
    let question = match &dialog {
        SavedDialog::Delete { index } => model.query().saved().get(*index).map(|saved| {
            texts.format(
                QueryText::DeleteQuestion,
                &[("name", saved.name().to_owned().into())],
            )
        }),
        _ => None,
    };
    let mut name = match &dialog {
        SavedDialog::Save { name } | SavedDialog::Rename { name, .. } => Some(name.clone()),
        SavedDialog::Delete { .. } => None,
    };
    let ready = matches!(note, NameNote::Free | NameNote::Replaces);
    let shown = frames::modal(frames::modal_id(id), tone).show(ui.ctx(), |ui| {
        ui.set_width(frames::MODAL_BODY_WIDTH);
        let closed = frames::head(ui, (&title, tone), true, Some(&close_dialog));
        let submit = body(ui, texts, (name.as_mut(), note, first), question.as_deref());
        let pressed = foot(ui, texts, (action, ready), tone);
        (closed, pressed, submit && ready)
    });
    frames::modal_decoration(ui.ctx(), shown.response.layer_id, shown.response.rect, tone);
    let (closed, pressed, submitted) = shown.inner;
    if let (Some(typed), Some(open)) = (name, model.query_mut().saved_dialog_mut())
        && let Some(slot) = open.name_mut()
        && *slot != typed
    {
        *slot = typed;
    }
    if pressed == Some(true) || submitted {
        requests.settings_changed |= model.query_mut().submit_saved_dialog();
    } else if closed || pressed == Some(false) || shown.should_close() {
        model.query_mut().close_saved_dialog();
    }
}

/// Returns the modal id, the title, the action and the tone of `dialog`.
const fn parts(dialog: &SavedDialog) -> (&'static str, QueryText, QueryText, Tone) {
    match dialog {
        SavedDialog::Save { .. } => (
            "query-save-dialog",
            QueryText::SaveTitle,
            QueryText::SaveAction,
            Tone::Normal,
        ),
        SavedDialog::Rename { .. } => (
            "query-rename-dialog",
            QueryText::RenameTitle,
            QueryText::RenameAction,
            Tone::Normal,
        ),
        SavedDialog::Delete { .. } => (
            "query-delete-dialog",
            QueryText::DeleteTitle,
            QueryText::DeleteAction,
            Tone::Danger,
        ),
    }
}

/// Shows the name field with its note, or the question; returns whether Enter was pressed in
/// the field.
fn body(
    ui: &mut Ui,
    texts: &Texts,
    (name, note, first): (Option<&mut String>, NameNote, bool),
    question: Option<&str>,
) -> bool {
    let mut submit = false;
    if let Some(name) = name {
        submit = name_field(ui, texts, name, first);
        if let Some(text) = note_text(texts, note) {
            let colour = if note == NameNote::Replaces {
                palette::GRAY_2_TEXT
            } else {
                palette::DANGER_TEXT
            };
            ui.add_space(SPACE_3);
            ui.add(Label::new(
                plain(text, FS_SM, Weight::Regular).color(colour),
            ));
        }
    }
    if let Some(question) = question {
        ui.add(Label::new(plain(question, FS_SM, Weight::Regular)));
    }
    submit
}

/// Shows the labelled name field, focused when the dialog opens; returns whether Enter was
/// pressed in it.
fn name_field(ui: &mut Ui, texts: &Texts, name: &mut String, first: bool) -> bool {
    let label = ui.label(eyebrow(&texts.get(QueryText::NameLabel)));
    let field = TextEdit::singleline(name)
        .char_limit(SavedQueries::MAX_NAME_LEN)
        .hint_text(plain(texts.get(QueryText::NameHint), FS_BODY, Weight::Regular).italics())
        .font(font(FS_BODY, Weight::Regular))
        .margin(Margin::symmetric(px(SPACE_3), 0))
        .min_size(vec2(0.0, TOUCH_TARGET))
        .desired_width(ui.available_width())
        .align(Align2::LEFT_CENTER);
    let response = ui.add(field).labelled_by(label.id);
    if first {
        response.request_focus();
    }
    response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter))
}

fn note_text(texts: &Texts, note: NameNote) -> Option<String> {
    let max = |limit: usize| [("max", u64::try_from(limit).unwrap_or(u64::MAX).into())];
    Some(match note {
        NameNote::Free => return None,
        NameNote::Replaces => texts.get(QueryText::NameReplaces),
        NameNote::Problem(NameProblem::Empty) => texts.get(QueryText::NameEmpty),
        NameNote::Problem(NameProblem::TooLong) => {
            texts.format(QueryText::NameTooLong, &max(SavedQueries::MAX_NAME_LEN))
        }
        NameNote::Problem(NameProblem::Taken) => texts.get(QueryText::NameTaken),
        NameNote::Problem(NameProblem::Control) => texts.get(QueryText::NameControl),
        NameNote::Problem(NameProblem::Full) => {
            let size = u64::try_from(SavedQueries::MAX_TOTAL_LEN).unwrap_or(u64::MAX);
            let [limit] = max(SavedQueries::LIMIT);
            texts.format(
                QueryText::NameFull,
                &[limit, ("size", format::size(size, texts.language()).into())],
            )
        }
        NameNote::Problem(NameProblem::BadText) => texts.get(QueryText::NameBadText),
    })
}

/// Shows the foot: the ghost Cancel and, right of it, `action` as the CTA, or as the filled red
/// `.btn-danger` in a danger modal. Returns `Some(true)`
/// for the CTA and `Some(false)` for Cancel.
fn foot(
    ui: &mut Ui,
    texts: &Texts,
    (action, enabled): (QueryText, bool),
    tone: Tone,
) -> Option<bool> {
    let kind = if tone == Tone::Danger {
        Kind::Danger
    } else {
        Kind::Cta
    };
    ui.add_space(MODAL_GROUP_GAP - SPACE_3);
    frames::foot_rule(ui);
    let mut pressed = None;
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let cta = texts.get(action);
        if button::button(ui, kind, Size::Normal, &cta, enabled).clicked() {
            pressed = Some(true);
        }
        let cancel = texts.get(QueryText::Cancel);
        if button::button(ui, Kind::Ghost, Size::Normal, &cancel, true).clicked() {
            pressed = Some(false);
        }
    });
    pressed
}
