// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use egui::{Frame, Panel, Rect, Ui, vec2};

use crate::theme::desktop::NOTICE_ICON;
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_3, Weight};
use crate::widgets::frames;
use crate::widgets::indicator::warning_icon;
use crate::widgets::text::plain;
use p4kx_extraction::Archive;

use crate::{Model, Text};

fn notice_text(ui: &Ui, model: &Model, count: usize) -> Arc<str> {
    let id = ui
        .scope_id()
        .with(("archive-issues", model.texts().language(), count));
    if let Some(cached) = ui.data(|data| data.get_temp::<Arc<str>>(id)) {
        return cached;
    }
    let kind = if count == 1 { "one" } else { "other" };
    let text: Arc<str> = model
        .texts()
        .format(
            Text::ArchiveIssues,
            &[("count", count.into()), ("kind", kind.into())],
        )
        .into();
    ui.data_mut(|data| data.insert_temp(id, Arc::clone(&text)));
    text
}

/// Shows the warning notice that the directory of the open Archive has defects in its Entries,
/// with their number.
pub(super) fn show(ui: &mut Ui, model: &Model) {
    let count = model
        .browser()
        .and_then(|browser| browser.index().archive())
        .map_or(0, Archive::issue_count);
    if count == 0 || model.opening_path().is_some() {
        return;
    }
    Panel::top("archive-issues")
        .frame(Frame::NONE.fill(palette::SURFACE_PANEL))
        .show_separator_line(false)
        .show(ui, |ui| {
            frames::warning_alert(ui, |ui| {
                ui.spacing_mut().item_spacing.x = SPACE_3;
                ui.horizontal(|ui| {
                    let (icon, _) = ui
                        .allocate_exact_size(vec2(NOTICE_ICON, NOTICE_ICON), egui::Sense::hover());
                    warning_icon(
                        ui,
                        Rect::from_min_size(icon.min, icon.size()),
                        palette::WARNING,
                    );
                    let notice = notice_text(ui, model, count);
                    let text = plain(&*notice, FS_SM, Weight::Regular).color(palette::GRAY_1);
                    ui.add(egui::Label::new(text).wrap());
                });
            });
        });
}
