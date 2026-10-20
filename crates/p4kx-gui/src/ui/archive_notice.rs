use std::sync::Arc;

use egui::{Frame, Panel, Rect, Ui, vec2};

use crate::theme::desktop::NOTICE_ICON;
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_3, Weight};
use crate::widgets::frames;
use crate::widgets::indicator::warning_icon;
use crate::widgets::text::plain;
use crate::{Model, Text, WorkTab};

fn channels(model: &Model) -> Option<&str> {
    if model.work_tab() == WorkTab::Comparison {
        return model.comparison().test_channels();
    }
    if model.opening_path().is_some() {
        return None;
    }
    let kind = model.browser()?.archive_index().channel();
    kind.name().filter(|_| kind.is_test_build())
}

fn notice_text(ui: &Ui, model: &Model, channel: &str) -> Arc<str> {
    let id = ui
        .scope_id()
        .with(("archive-test-build", model.texts().language(), channel));
    if let Some(cached) = ui.data(|data| data.get_temp::<Arc<str>>(id)) {
        return cached;
    }
    let text: Arc<str> = model
        .texts()
        .format(Text::ArchiveTestBuild, &[("channel", channel.into())])
        .into();
    ui.data_mut(|data| data.insert_temp(id, Arc::clone(&text)));
    text
}

/// Shows the warning notice that the open Archive belongs to a Star Citizen test build, whose
/// material is confidential. It stays for as long as such an Archive is open.
pub(super) fn show(ui: &mut Ui, model: &Model) {
    let Some(channel) = channels(model) else {
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
                    let notice = notice_text(ui, model, channel);
                    let text = plain(&*notice, FS_SM, Weight::Regular).color(palette::GRAY_1);
                    ui.add(egui::Label::new(text).wrap());
                });
            });
        });
}
