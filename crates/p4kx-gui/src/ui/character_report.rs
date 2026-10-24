// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use egui::{Label, Ui};

use super::search::eyebrow;
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, Weight};
use crate::widgets::frames;
use crate::widgets::text::plain;
use crate::{CharacterText as T, Model, format};

/// Shows the report of the Character: the pieces that could not be fitted or done in full, and
/// the pieces of the Loadout that another piece took the place of.
pub(super) fn show(ui: &mut Ui, model: &Model) {
    let tab = model.characters();
    let texts = model.texts();
    frames::preview_body().show(ui, |ui| {
        let (issues, replaced) = (tab.issues(), tab.replaced());
        if tab.scene().is_none() {
            return;
        }
        if issues.is_empty() && replaced.is_empty() {
            let text = plain(texts.get(T::ReportAll), FS_SM, Weight::Regular);
            ui.add(Label::new(text.color(palette::GRAY_2_TEXT)));
            return;
        }
        let count = format::number(
            u64::try_from(issues.len()).unwrap_or(u64::MAX),
            texts.language(),
        );
        ui.label(eyebrow(
            &texts.format(T::ReportHeading, &[("number", count.into())]),
        ));
        for issue in issues {
            let args = [
                ("name", issue.name.clone().into()),
                ("reason", issue.reason.clone().into()),
            ];
            let (text, colour) = if issue.skipped {
                (texts.format(T::ReportSkipped, &args), palette::WARNING)
            } else {
                (texts.format(T::ReportNote, &args), palette::GRAY_1)
            };
            ui.add(Label::new(plain(text, FS_SM, Weight::Regular).color(colour)).selectable(true));
        }
        for one in replaced {
            let args = [
                ("name", one.name().to_owned().into()),
                ("by", one.by().to_owned().into()),
            ];
            let text = plain(
                texts.format(T::ReportReplaced, &args),
                FS_SM,
                Weight::Regular,
            );
            ui.add(Label::new(text.color(palette::GRAY_1)).selectable(true));
        }
    });
}
