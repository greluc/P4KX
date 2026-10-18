use egui::{Panel, Ui};
use p4kx_extraction::DEFAULT_WATCH_INTERVAL;

use crate::{Model, Text};

/// Compares the open Archive with the disk when due, keeps the window repainting so that this
/// happens while it is idle, and shows the notice with the reload action once it changed.
pub(super) fn show(ui: &mut Ui, model: &mut Model) {
    if model.browser().is_none() {
        return;
    }
    model.check_archive_when_due();
    ui.ctx().request_repaint_after(DEFAULT_WATCH_INTERVAL);
    if !model.archive_changed() || model.opening_path().is_some() {
        return;
    }
    let mut reload = false;
    Panel::top("archive-changed").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let path = model
                .browser()
                .map(|browser| browser.index().archive().path().display().to_string())
                .unwrap_or_default();
            let texts = model.texts();
            ui.colored_label(
                ui.visuals().error_fg_color,
                texts.format(Text::ArchiveChanged, &[("path", path.into())]),
            );
            reload = ui.button(texts.get(Text::ActionReload)).clicked();
        });
    });
    if reload {
        let ctx = ui.ctx().clone();
        model.reload(move || ctx.request_repaint());
    }
}
