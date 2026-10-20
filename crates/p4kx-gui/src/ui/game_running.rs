use egui::{Frame, Panel, Rect, Ui, vec2};

use crate::GAME_CHECK_INTERVAL;
use crate::theme::desktop::NOTICE_ICON;
use crate::theme::palette;
use crate::theme::tokens::{FS_SM, SPACE_3, Weight};
use crate::widgets::frames;
use crate::widgets::indicator::warning_icon;
use crate::widgets::text::plain;
use crate::{Model, Text};

/// Looks at the process list when due and shows the warning notice that the game is running.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    let ctx = ui.ctx().clone();
    model.check_game_when_due(move || ctx.request_repaint());
    ui.ctx().request_repaint_after(GAME_CHECK_INTERVAL);
    if !model.game_running() {
        return;
    }
    Panel::top("game-running")
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
                    let notice = model.texts().get(Text::GameRunning);
                    let text = plain(notice, FS_SM, Weight::Regular).color(palette::GRAY_1);
                    ui.add(egui::Label::new(text).wrap());
                });
            });
        });
}
