use egui::{Frame, Panel, Rect, Ui, vec2};
use p4kx_channels::ChannelKind;

use crate::theme::desktop::NOTICE_ICON;
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_3, Weight};
use crate::widgets::frames;
use crate::widgets::indicator::warning_icon;
use crate::widgets::text::plain;
use crate::{Model, Text};

/// Shows the warning notice that the open Archive belongs to a Star Citizen test build, whose
/// material is confidential. It stays for as long as such an Archive is open.
pub(super) fn show(ui: &mut Ui, model: &Model) {
    if model.opening_path().is_some() {
        return;
    }
    let Some(browser) = model.browser() else {
        return;
    };
    let kind = ChannelKind::of_archive(browser.archive_index().archive().path());
    let Some(channel) = kind.name().filter(|_| kind.is_test_build()) else {
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
                    let notice = model
                        .texts()
                        .format(Text::ArchiveTestBuild, &[("channel", channel.into())]);
                    let text = plain(notice, FS_SM, Weight::Regular).color(palette::GRAY_1);
                    ui.add(egui::Label::new(text).wrap());
                });
            });
        });
}
