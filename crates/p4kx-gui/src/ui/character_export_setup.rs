// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::Ui;

use super::assembly_export::{folder_notice, form_choice, target_row};
use super::extraction::{foot, group_label, refusal_notice};
use super::{Dialog, Requests, regions};
use crate::theme::desktop::MODAL_GROUP_GAP;
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, FS_XS, Weight};
use crate::widgets::frames::{self, Tone};
use crate::widgets::placement;
use crate::widgets::tab;
use crate::widgets::text::plain;
use crate::{
    AssemblyExportForm, AssemblyExportText as A, CharacterExportSetup, CharacterText as T,
    ExtractionText, Model, Texts,
};

enum Action {
    Submit,
    Close,
    ChooseFile,
    ChooseFolder,
}

/// Shows the export setup of the Character while it is open and applies what the user chose.
pub(super) fn setup(ui: &Ui, model: &mut Model, requests: &mut Requests) {
    let busy = !model.characters().export_available();
    let Some((setup, texts)) = model.character_export_setup_with_texts() else {
        return;
    };
    let title = texts.get(T::ExportSetupTitle);
    let close_dialog = texts.get(crate::Text::CloseDialog);
    let id = frames::modal_id("character-export-setup");
    let shown = placement::hanging_modal(ui.ctx(), id, Tone::Normal).show(ui.ctx(), |ui| {
        ui.set_width(frames::MODAL_WIDE_BODY_WIDTH);
        let closed = frames::head(ui, (&title, Tone::Normal), true, Some(&close_dialog));
        let room = placement::body_room(ui);
        let body = frames::scrolling_body_in(ui, ("character-export-body", room), |ui| {
            choices(ui, texts, (setup, busy))
        });
        regions::record(ui.ctx(), |regions| {
            regions.scrolls = body.content_size.y > body.inner_rect.height() + 0.5;
        });
        let action = body.inner;
        let pressed = foot(
            ui,
            texts,
            (A::Start, setup.is_ready() && !busy),
            ExtractionText::Close,
        );
        let chosen = match pressed {
            Some(true) => Some(Action::Submit),
            Some(false) => Some(Action::Close),
            None => None,
        };
        if closed {
            Some(Action::Close)
        } else {
            action.or(chosen)
        }
    });
    frames::modal_decoration(
        ui.ctx(),
        shown.response.layer_id,
        shown.response.rect,
        Tone::Normal,
    );
    regions::record(ui.ctx(), |regions| {
        regions.dialogs.push(shown.response.rect);
    });
    let close = shown.should_close();
    match shown.inner {
        Some(Action::Submit) => {
            let ctx = ui.ctx().clone();
            model.submit_character_export(move || ctx.request_repaint());
        }
        Some(Action::Close) => model.characters_mut().close_export_setup(),
        Some(Action::ChooseFile) => requests.dialog = Some(Dialog::CharacterFile),
        Some(Action::ChooseFolder) => requests.dialog = Some(Dialog::CharacterPackage),
        None if close => model.characters_mut().close_export_setup(),
        None => {}
    }
}

fn choices(
    ui: &mut Ui,
    texts: &Texts,
    (setup, busy): (&mut CharacterExportSetup, bool),
) -> Option<Action> {
    let subject = texts.format(
        T::ExportSetupName,
        &[("name", setup.name().to_owned().into())],
    );
    ui.label(plain(subject, FS_XS, Weight::Regular).color(palette::GRAY_2_TEXT));
    ui.add_space(MODAL_GROUP_GAP);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            if let Some(chosen) = form_choice(ui, texts, setup.form()) {
                setup.set_form(chosen);
            }
        });
        if setup.form() == AssemblyExportForm::Package {
            ui.add_space(MODAL_GROUP_GAP);
            ui.vertical(|ui| animations(ui, texts, setup));
        }
    });
    let package = setup.form() == AssemblyExportForm::Package;
    if package {
        let note = plain(texts.get(T::ExportPackageNote), FS_XS, Weight::Regular);
        ui.add(egui::Label::new(note.color(palette::GRAY_2_TEXT)).wrap());
    }
    ui.add_space(MODAL_GROUP_GAP);
    let action = if package {
        let texts_of = (A::FolderHeading, A::ChooseFolder, A::FolderNone);
        let pressed = target_row(ui, texts, texts_of, setup.package_folder());
        folder_notice(
            ui,
            texts,
            (setup.package_problem(), setup.package_replaces()),
        );
        pressed.map(|()| Action::ChooseFolder)
    } else {
        let texts_of = (A::FileHeading, A::ChooseFile, A::FileNone);
        target_row(ui, texts, texts_of, setup.file()).map(|()| Action::ChooseFile)
    };
    refusal_notice(ui, texts, setup.refusal());
    if busy {
        let text = plain(texts.get(T::ExportBusy), FS_SM, Weight::Regular);
        ui.add(egui::Label::new(text.color(palette::WARNING)).wrap());
    }
    action
}

fn animations(ui: &mut Ui, texts: &Texts, setup: &mut CharacterExportSetup) {
    group_label(ui, &texts.get(T::ExportAnimationsHeading));
    let labels = [
        texts.get(T::ExportAnimationsNone),
        texts.get(T::ExportAnimationsAll),
    ];
    let active = Some(usize::from(setup.animations()));
    match ui.horizontal(|ui| tab::segments(ui, &labels, active)).inner {
        Some(0) => setup.set_animations(false),
        Some(1) => setup.set_animations(true),
        _ => {}
    }
}
