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
use crate::{Model, Text, WorkTab};

fn channels(model: &Model) -> Option<(&str, bool)> {
    if model.work_tab() == WorkTab::Comparison {
        return model
            .comparison()
            .test_channels()
            .map(|channels| (channels.names(), channels.several()));
    }
    if model.opening_path().is_some() {
        return None;
    }
    let kind = model.browser()?.archive_index().channel();
    kind.name()
        .filter(|_| kind.is_test_build())
        .map(|name| (name, false))
}

fn notice_text(ui: &Ui, model: &Model, channel: &str, several: bool) -> Arc<str> {
    let id = ui.scope_id().with((
        "archive-test-build",
        model.texts().language(),
        channel,
        several,
    ));
    if let Some(cached) = ui.data(|data| data.get_temp::<Arc<str>>(id)) {
        return cached;
    }
    let kind = if several { "other" } else { "one" };
    let mut text = model.texts().format(
        Text::ArchiveTestBuild,
        &[("channel", channel.into()), ("kind", kind.into())],
    );
    if channel.split(", ").any(|name| name == "EPTU") {
        text.push(' ');
        text.push_str(&model.texts().get(Text::ArchiveTestBuildNda));
    }
    let text: Arc<str> = text.into();
    ui.data_mut(|data| data.insert_temp(id, Arc::clone(&text)));
    text
}

/// Shows the warning notice that the open Archive belongs to a Channel that carries Star Citizen
/// test builds, whose material is confidential. It stays for as long as such an Archive is open.
pub(super) fn show(ui: &mut Ui, model: &Model) {
    let Some((channel, several)) = channels(model) else {
        return;
    };
    Panel::top("archive-test-build")
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
                    let notice = notice_text(ui, model, channel, several);
                    let text = plain(&*notice, FS_SM, Weight::Regular).color(palette::GRAY_1);
                    ui.add(egui::Label::new(text).wrap());
                });
            });
        });
}
