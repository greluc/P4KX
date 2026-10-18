use egui::{ScrollArea, Ui, Window};

use crate::{Model, Text, theme};

pub(super) fn show(ui: &Ui, model: &mut Model) {
    let mut open = model.licence_shown();
    if !open {
        return;
    }
    Window::new(model.texts().get(Text::FontLicence))
        .open(&mut open)
        .default_size([560.0, 480.0])
        .show(ui.ctx(), |ui| {
            ScrollArea::vertical().show(ui, |ui| ui.label(theme::FONT_LICENCE));
        });
    model.set_licence_shown(open);
}
